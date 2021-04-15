use std::{
    borrow::Cow,
    future::Future,
    marker::PhantomData,
    sync::Arc,
    task::{Poll, Waker},
};

use async_channel::{Receiver, Sender};
use bevy_ecs_macros::all_tuples;
use futures_lite::pin;
use parking_lot::Mutex;

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    prelude::World,
    query::Access,
};

use super::{
    check_system_change_tick, System, SystemId, SystemParam, SystemParamFetch, SystemParamState,
    SystemState,
};

pub struct Accessor<P: SystemParam> {
    channel: Sender<Box<dyn GenericAccess<P>>>,
    _marker: PhantomData<fn() -> P>,
}

impl<P: SystemParam> Accessor<P> {
    pub fn new() -> (Self, AccessorRunnerSystem<P>) {
        let (tx, rx) = async_channel::unbounded();
        (
            Accessor {
                channel: tx,
                _marker: Default::default(),
            },
            AccessorRunnerSystem {
                system_state: SystemState::with_name(
                    format!("Accessor system {}", std::any::type_name::<P>()).into(),
                ),
                param_state: None,
                channel: rx,
                _marker: Default::default(),
            },
        )
    }
}

impl<P: SystemParam> Clone for Accessor<P> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            _marker: Default::default(),
        }
    }
}

#[doc(hidden)]
pub trait AccessFn<'a, Out, Param: SystemParam, M>: Send + Sync + 'static {
    /// # Safety
    /// this is an internal trait that exists to bypass some limitations of rustc, please ignore it.
    unsafe fn run(
        self: Box<Self>,
        state: &'a mut Param::Fetch,
        system_state: &'a SystemState,
        world: &'a World,
        change_tick: u32,
    ) -> Out;
}
pub struct SingleMarker;
impl<'a, Out, Func, P> AccessFn<'a, Out, P, SingleMarker> for Func
where
    Func: FnOnce(P) -> Out
        + FnOnce(<<P as SystemParam>::Fetch as SystemParamFetch<'a>>::Item) -> Out
        + Send
        + Sync
        + 'static,
    Out: 'static,
    P: SystemParam,
{
    #[inline]
    unsafe fn run(
        self: Box<Self>,
        state: &'a mut <P as SystemParam>::Fetch,
        system_state: &'a SystemState,
        world: &'a World,
        change_tick: u32,
    ) -> Out {
        let param = <<P as SystemParam>::Fetch as SystemParamFetch<'a>>::get_param(
            state,
            system_state,
            world,
            change_tick,
        );
        self(param)
    }
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<'a, Out, Func, $($param: SystemParam),*> AccessFn<'a, Out, ($($param,)*), ()> for Func
        where
            Func:
                FnOnce($($param),*) -> Out +
                FnOnce($(<<$param as SystemParam>::Fetch as SystemParamFetch<'a>>::Item),*) -> Out + Send + Sync + 'static,
            Out: 'static
        {
            #[inline]
            unsafe fn run(
                self: Box<Self>,
                state: &'a mut <($($param,)*) as SystemParam>::Fetch,
                system_state: &'a SystemState, world: &'a World,
                change_tick: u32,
            ) -> Out {
                let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch<'a>>::get_param(state, system_state, world, change_tick);
                self($($param),*)
            }
        }
    };
}

all_tuples!(impl_system_function, 0, 12, F);

impl<P: SystemParam + 'static> Accessor<P> {
    pub fn access<R: Send + Sync + 'static, M: 'static>(
        &mut self,
        sync: impl for<'a> AccessFn<'a, R, P, M> + Send + Sync,
    ) -> impl Future<Output = R> + Send + Sync + 'static {
        AccessFuture {
            state: AccessFutureState::FirstPoll {
                boxed: Box::new(sync),
                tx: self.channel.clone(),
            },
        }
    }
}

struct ParallelAccess<P: SystemParam, Out, M> {
    inner: Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'a, Out, P, M> + Send + Sync>>>>,
    tx: Sender<Out>,
    waker: Waker,
}

trait GenericAccess<P: SystemParam>: Send + Sync + 'static {
    unsafe fn run(
        self: Box<Self>,
        param_state: &mut P::Fetch,
        system_state: &SystemState,
        world: &World,
        change_tick: u32,
    );
}

impl<P, Out, M: 'static> GenericAccess<P> for ParallelAccess<P, Out, M>
where
    P: SystemParam + 'static,
    Out: Send + Sync + 'static,
{
    unsafe fn run(
        self: Box<Self>,
        param_state: &mut P::Fetch,
        state: &SystemState,
        world: &World,
        change_tick: u32,
    ) {
        if let Some(sync) = self.inner.lock().take() {
            self.tx
                .try_send(sync.run(param_state, state, world, change_tick))
                .unwrap();
        }
        self.waker.wake();
    }
}

enum AccessFutureState<P, R, M> {
    FirstPoll {
        boxed: Box<dyn for<'a> AccessFn<'a, R, P, M> + Send + Sync + 'static>,
        tx: Sender<Box<dyn GenericAccess<P>>>,
    },
    WaitingForCompletion(
        Receiver<R>,
        Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'a, R, P, M> + Send + Sync + 'static>>>>,
    ),
}

pub struct AccessFuture<P: SystemParam, R, M> {
    state: AccessFutureState<P, R, M>,
}

impl<P, R, M> Future for AccessFuture<P, R, M>
where
    P: SystemParam + 'static,
    R: Send + Sync + 'static,
    M: 'static,
{
    type Output = R;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match &mut self.state {
            AccessFutureState::FirstPoll { .. } => {
                let (tx, rx) = async_channel::bounded(1);
                let arc = Arc::new(Mutex::new(None));
                if let AccessFutureState::FirstPoll { boxed, tx: mtx } = std::mem::replace(
                    &mut self.state,
                    AccessFutureState::WaitingForCompletion(rx, arc.clone()),
                ) {
                    *arc.lock() = Some(boxed);
                    let msg = ParallelAccess {
                        inner: arc,
                        tx,
                        waker: cx.waker().clone(),
                    };
                    let boxed: Box<dyn GenericAccess<P>> = Box::new(msg);
                    mtx.try_send(boxed).unwrap();
                    Poll::Pending
                } else {
                    unreachable!()
                }
            }
            AccessFutureState::WaitingForCompletion(rx, _) => {
                let future = rx.recv();
                pin!(future);
                future.poll(cx).map(|v| v.unwrap())
            }
        }
    }
}

pub struct AccessorRunnerSystem<P: SystemParam> {
    system_state: SystemState,
    param_state: Option<P::Fetch>,
    channel: Receiver<Box<dyn GenericAccess<P>>>,
    _marker: PhantomData<fn() -> P>,
}

impl<P: SystemParam + 'static> System for AccessorRunnerSystem<P> {
    type In = ();
    type Out = ();

    fn name(&self) -> Cow<'static, str> {
        self.system_state.name.clone()
    }

    fn id(&self) -> SystemId {
        self.system_state.id
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.system_state.archetype_component_access
    }

    unsafe fn run_unsafe(&mut self, _: Self::In, world: &World) -> Self::Out {
        loop {
            match self.channel.try_recv() {
                Ok(sync) => {
                    let change_tick = world.increment_change_tick();
                    sync.run(
                        &mut self.param_state.as_mut().unwrap(),
                        &self.system_state,
                        world,
                        change_tick,
                    );
                    self.system_state.last_change_tick = change_tick;
                }
                Err(async_channel::TryRecvError::Closed) => panic!(
                    "`AccessorRunnerSystem` called but all relevant accessors have been dropped!"
                ),
                Err(async_channel::TryRecvError::Empty) => break,
            }
        }
    }

    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(<P::Fetch as SystemParamState>::init(
            world,
            &mut self.system_state,
            <P::Fetch as SystemParamState>::default_config(),
        ))
    }

    fn apply_buffers(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.apply(world);
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.system_state.component_access_set.combined_access()
    }

    fn new_archetype(&mut self, archetype: &Archetype) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.new_archetype(archetype, &mut self.system_state);
    }

    fn is_send(&self) -> bool {
        self.system_state.is_send()
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: u32) {
        check_system_change_tick(
            &mut self.system_state.last_change_tick,
            change_tick,
            self.system_state.name.as_ref(),
        );
    }
}

#[cfg(test)]
mod test {
    use bevy_tasks::TaskPool;

    use crate::{
        prelude::{Res, ResMut, World},
        schedule::{Stage, SystemStage},
    };

    use super::Accessor;

    #[test]
    fn simple_test() {
        let mut world = World::new();
        let ctp = TaskPool::new();
        world.insert_resource("hi".to_string());
        world.insert_resource(3u32);
        let (mut accessor, system) = Accessor::<(Res<u32>, ResMut<String>)>::new();
        let mut stage = SystemStage::parallel();
        stage.add_system(system);
        let _a = accessor.clone();
        ctp.spawn(async move {
            accessor
                .access(|(r, mut s): (Res<u32>, ResMut<String>)| {
                    assert_eq!(*r, 3);
                    *s = "hello".into();
                })
                .await;
        })
        .detach();

        let start = std::time::Instant::now();
        loop {
            stage.run(&mut world);
            if world.get_resource::<String>().unwrap() == "hello" {
                break;
            } else if std::time::Instant::now() - start > std::time::Duration::from_secs_f32(0.1) {
                panic!("timeout!");
            }
        }
    }
}
