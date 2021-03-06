use std::{
    borrow::Cow,
    future::Future,
    marker::PhantomData,
    sync::Arc,
    task::{Poll, Waker},
};

use async_channel::{Receiver, Sender};
use bevy_ecs_macros::all_tuples;
use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;
use futures_lite::pin;
use parking_lot::Mutex;

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    prelude::{System, World},
    query::Access,
    system::{SystemId, SystemParam, SystemParamFetch, SystemParamState, SystemState},
};

use super::{AccessorTrait, AsyncSystemHandle, AsyncSystemOutput, AsyncSystemOutputError};

pub struct Accessor<P: SystemParam> {
    channel: Sender<Box<dyn GenericAccess<P>>>,
    _marker: PhantomData<fn() -> P>,
}

impl<P: SystemParam> AccessorTrait for Accessor<P> {
    type AccessSystem = AccessorRunnerSystem<P>;

    fn new() -> (Self, Self::AccessSystem) {
        let (tx, rx) = async_channel::unbounded();
        (
            Accessor {
                channel: tx,
                _marker: Default::default(),
            },
            AccessorRunnerSystem {
                system_state: SystemState::new("".into()),
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

pub trait AccessFn<'env, 'a, Out, Param: SystemParam, M>: Send + Sync + 'env {
    fn run(
        self: Box<Self>,
        state: &'a mut Param::Fetch,
        system_state: &'a SystemState,
        world: &'a World,
    ) -> Out;
}
pub struct SingleMarker;
impl<'a, 'env, Out, Func, P> AccessFn<'env, 'a, Out, P, SingleMarker> for Func
where
    Func: FnOnce(P) -> Out
        + FnOnce(<<P as SystemParam>::Fetch as SystemParamFetch<'a>>::Item) -> Out
        + Send
        + Sync
        + 'env,
    Out: 'env,
    P: SystemParam,
{
    #[inline]
    fn run(
        self: Box<Self>,
        state: &'a mut <P as SystemParam>::Fetch,
        system_state: &'a SystemState,
        world: &'a World,
    ) -> Out {
        unsafe {
            let param = <<P as SystemParam>::Fetch as SystemParamFetch<'a>>::get_param(
                state,
                system_state,
                world,
            );
            self(param)
        }
    }
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<'a, 'env, Out, Func, $($param: SystemParam),*> AccessFn<'env, 'a, Out, ($($param,)*), ()> for Func
        where
            Func:
                FnOnce($($param),*) -> Out +
                FnOnce($(<<$param as SystemParam>::Fetch as SystemParamFetch<'a>>::Item),*) -> Out + Send + Sync + 'env,
            Out: 'env
        {
            #[inline]
            fn run(self: Box<Self>, state: &'a mut <($($param,)*) as SystemParam>::Fetch, system_state: &'a SystemState, world: &'a World) -> Out {
                unsafe {
                    let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch<'a>>::get_param(state, system_state, world);
                    self($($param),*)
                }
            }
        }
    };
}

all_tuples!(impl_system_function, 0, 12, F);

impl<P: SystemParam> Accessor<P> {
    pub fn access<'env, R: Send + Sync + 'env, M: 'static>(
        &'env mut self,
        sync: impl for<'a> AccessFn<'env, 'a, R, P, M> + Send + Sync + 'env,
    ) -> impl Future<Output = R> + Send + Sync + 'env {
        AccessFuture {
            state: AccessFutureState::FirstPoll {
                boxed: Box::new(sync),
                tx: self.channel.clone(),
            },
        }
    }
}

struct ParallelAccess<'env, P: SystemParam, Out, M> {
    inner: Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'env, 'a, Out, P, M> + Send + Sync>>>>,
    tx: Sender<Out>,
    waker: Waker,
}

trait GenericAccess<P: SystemParam>: Send + Sync {
    unsafe fn run(
        self: Box<Self>,
        param_state: &mut P::Fetch,
        system_state: &SystemState,
        world: &World,
    );
}

impl<'env, P, Out, M: 'static> GenericAccess<P> for ParallelAccess<'env, P, Out, M>
where
    P: SystemParam + 'env,
    Out: Send + Sync + 'env,
{
    unsafe fn run(self: Box<Self>, param_state: &mut P::Fetch, state: &SystemState, world: &World) {
        if let Some(sync) = self.inner.lock().take() {
            self.tx
                .try_send(sync.run(param_state, state, world))
                .unwrap();
        }
        self.waker.wake();
    }
}

enum AccessFutureState<'env, P, R, M> {
    FirstPoll {
        boxed: Box<dyn for<'a> AccessFn<'env, 'a, R, P, M> + Send + Sync + 'env>,
        tx: Sender<Box<dyn GenericAccess<P>>>,
    },
    WaitingForCompletion(
        Receiver<R>,
        Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'env, 'a, R, P, M> + Send + Sync + 'env>>>>,
    ),
}

pub struct AccessFuture<'env, P: SystemParam, R, M> {
    state: AccessFutureState<'env, P, R, M>,
}

impl<'env, P, R, M> Future for AccessFuture<'env, P, R, M>
where
    P: SystemParam + 'env,
    R: Send + Sync + 'env,
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
                    let boxed: Box<dyn GenericAccess<P> + 'env> = Box::new(msg);
                    let boxed: Box<dyn GenericAccess<P> + 'static> =
                    // Safe: the reference will only live as long as this struct, as the drop impl will drop the references
                        unsafe { std::mem::transmute(boxed) };
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

impl<'env, P: SystemParam, R, M> Drop for AccessFuture<'env, P, R, M> {
    fn drop(&mut self) {
        if let AccessFutureState::WaitingForCompletion(_, arc) = &self.state {
            *arc.lock() = None;
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
                Ok(sync) => sync.run(
                    &mut self.param_state.as_mut().unwrap(),
                    &self.system_state,
                    world,
                ),
                Err(async_channel::TryRecvError::Closed) => panic!(
                    "`AccessorRunnerSystem` called but all relevant accessors have been dropped"
                ),
                Err(async_channel::TryRecvError::Empty) => break,
            }
        }
    }

    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(<P::Fetch as SystemParamState>::init(
            world,
            &mut self.system_state,
            Default::default(),
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
}

pub trait AccessSystemsTuple: Send + Sync + 'static {
    fn new_archetype(
        &mut self,
        archetype: &Archetype,
        archetype_component_access: &mut Access<ArchetypeComponentId>,
    );
    fn is_send(&self) -> bool;
    fn apply_buffers(&mut self, world: &mut World);
    fn initialize(&mut self, world: &mut World);
    unsafe fn run(&mut self, world: &World);
}

pub struct AsyncChainSystem<In, Out, Systems>
where
    Out: Send + Sync + 'static,
{
    pub(super) inner_systems: Systems,
    pub(super) handle: AsyncSystemHandle<In, Out>,
    pub(super) return_handle: Option<AsyncSystemOutput<Out>>,
    pub(super) name: Cow<'static, str>,
    pub(super) id: SystemId,
    pub(super) component_access: Access<ComponentId>,
    pub(super) archetype_component_access: Access<ArchetypeComponentId>,
    pub(super) startup_future:
        Option<Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync + 'static>>,
}

impl<In, Out, Systems: AccessSystemsTuple> System for AsyncChainSystem<In, Out, Systems>
where
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    type In = In;
    type Out = Option<Out>;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        self.inner_systems.run(world);
        if let Some(ref mut handle) = &mut self.return_handle {
            match handle.get() {
                Ok(v) => {
                    self.return_handle = Some(self.handle.fire(input));
                    Some(v)
                }
                Err(AsyncSystemOutputError::SystemNotFinished) => None,
                Err(AsyncSystemOutputError::OutputMoved) => panic!(),
            }
        } else {
            self.return_handle = Some(self.handle.fire(input));
            None
        }
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.inner_systems.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        if let Some(fut) = self.startup_future.take() {
            let tp = world.get_resource_mut::<AsyncComputeTaskPool>().unwrap();
            tp.spawn((fut)(tp.clone().0)).detach();
        }
        self.inner_systems.initialize(world);
    }

    fn new_archetype(&mut self, archetype: &Archetype) {
        self.inner_systems
            .new_archetype(archetype, &mut self.archetype_component_access);
    }

    fn is_send(&self) -> bool {
        self.inner_systems.is_send()
    }
}
