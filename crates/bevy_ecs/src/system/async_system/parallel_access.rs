use std::{
    any::TypeId,
    borrow::Cow,
    future::Future,
    sync::Arc,
    task::{Poll, Waker},
};

use async_channel::{Receiver, Sender};
use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;
use futures_lite::pin;
use parking_lot::Mutex;

use crate::{
    AccessorTrait, ArchetypeComponent, AsyncSystemHandle, AsyncSystemOutput,
    AsyncSystemOutputError, FetchSystemParam, Resources, System, SystemId, SystemParam,
    SystemState, TypeAccess, World,
};

use super::OpaquePhantomData;

pub struct Accessor<P: SystemParam> {
    channel: Sender<Box<dyn GenericAccess>>,
    _marker: OpaquePhantomData<P>,
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
                state: {
                    SystemState {
                        name: std::any::type_name::<Self>().into(),
                        archetype_component_access: TypeAccess::default(),
                        component_access: TypeAccess::default(),
                        resource_access: TypeAccess::default(),
                        is_non_send: false,
                        local_resource_access: TypeAccess::default(),
                        id: SystemId::new(),
                        commands: Default::default(),
                        arc_commands: Default::default(),
                        current_query_index: Default::default(),
                        query_archetype_component_accesses: Vec::new(),
                        query_accesses: Vec::new(),
                        query_type_names: Vec::new(),
                    }
                },
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

pub trait AccessFn<'a, 'env, P: SystemParam, Out, Marker> {
    unsafe fn call(
        self: Box<Self>,
        state: &'a SystemState,
        world: &'a World,
        resources: &'a Resources,
    ) -> Option<Out>;
}
pub struct SimpleMarker;
impl<'a, 'env, Out, P, F> AccessFn<'a, 'env, P, Out, SimpleMarker> for F
where
    P: SystemParam,
    F: FnOnce(P) -> Out + 'env,
    F: FnOnce(<P::Fetch as FetchSystemParam<'a>>::Item) -> Out + 'env,
{
    unsafe fn call(
        self: Box<Self>,
        state: &'a SystemState,
        world: &'a World,
        resources: &'a Resources,
    ) -> Option<Out> {
        Some(self(<P::Fetch as FetchSystemParam<'a>>::get_param(
            state, world, resources,
        )?))
    }
}
pub struct ComplexMarker;
macro_rules! impl_access_fn {
    ($($A: ident),*) => {
        impl<'a, 'env, Out, $($A,)* Func> AccessFn<'a, 'env, ($($A,)*), Out, ComplexMarker> for Func
        where
            $($A: SystemParam,)*
            Func: FnOnce($($A,)*) -> Out + 'env,
            Func: FnOnce(
                $(<$A::Fetch as FetchSystemParam<'a>>::Item,)*
            ) -> Out + 'env,
        {
            unsafe fn call(
                self: Box<Self>,
                state: &'a SystemState,
                world: &'a World,
                resources: &'a Resources,
            ) -> Option<Out> {
                Some(self(
                    $(<$A::Fetch as FetchSystemParam<'a>>::get_param(
                        state, world, resources,
                    )?,)*
                ))
            }
        }
    };
}

impl_access_fn!(A);
impl_access_fn!(A, B);
impl_access_fn!(A, B, C);
impl_access_fn!(A, B, C, D);
impl_access_fn!(A, B, C, D, E);
impl_access_fn!(A, B, C, D, E, F);
impl_access_fn!(A, B, C, D, E, F, G);
impl_access_fn!(A, B, C, D, E, F, G, H);
impl_access_fn!(A, B, C, D, E, F, G, H, I);
impl_access_fn!(A, B, C, D, E, F, G, H, I, J);
impl_access_fn!(A, B, C, D, E, F, G, H, I, J, K);
impl_access_fn!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_access_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_access_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_access_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_access_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

impl<P: SystemParam> Accessor<P> {
    pub fn access<'env, R: Send + Sync + 'static, M: 'static>(
        &mut self,
        sync: impl for<'a> AccessFn<'a, 'env, P, R, M> + Send + Sync + 'env,
    ) -> impl Future<Output = R> + Send + Sync + 'env
    where
        P: 'env,
    {
        AccessFuture {
            state: AccessFutureState::FirstPoll {
                boxed: Box::new(sync),
                tx: self.channel.clone(),
            },
        }
    }
}

struct Access<'env, P: SystemParam, Out, Marker> {
    inner: Arc<
        Mutex<Option<Box<dyn for<'a> AccessFn<'a, 'env, P, Out, Marker> + Send + Sync + 'env>>>,
    >,
    tx: Sender<Out>,
    waker: Waker,
}

trait GenericAccess: Send + Sync {
    unsafe fn run(self: Box<Self>, state: &SystemState, world: &World, resources: &Resources);
}

impl<'env, P: SystemParam, Out: Send + Sync + 'env, Marker> GenericAccess
    for Access<'env, P, Out, Marker>
{
    unsafe fn run(self: Box<Self>, state: &SystemState, world: &World, resources: &Resources) {
        if let Some(sync) = self.inner.lock().take() {
            self.tx
                .try_send(sync.call(state, world, resources).unwrap())
                .unwrap();
        }
        self.waker.wake();
    }
}

enum AccessFutureState<'env, P, R, M> {
    FirstPoll {
        boxed: Box<dyn for<'a> AccessFn<'a, 'env, P, R, M> + Send + Sync + 'env>,
        tx: Sender<Box<dyn GenericAccess>>,
    },
    WaitingForCompletion(
        Receiver<R>,
        Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'a, 'env, P, R, M> + Send + Sync + 'env>>>>,
    ),
}

pub struct AccessFuture<'env, P: SystemParam, R, M> {
    state: AccessFutureState<'env, P, R, M>,
}

impl<'env, P: SystemParam + 'env, R: Send + Sync + 'env, M: 'static> Future
    for AccessFuture<'env, P, R, M>
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
                    let msg = Access {
                        inner: arc,
                        tx,
                        waker: cx.waker().clone(),
                    };
                    let boxed: Box<dyn GenericAccess + 'env> = Box::new(msg);
                    let boxed: Box<dyn GenericAccess + 'static> =
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
    state: SystemState,
    channel: Receiver<Box<dyn GenericAccess>>,
    _marker: OpaquePhantomData<P>,
}

impl<P: SystemParam + 'static> System for AccessorRunnerSystem<P> {
    type In = ();
    type Out = ();

    fn name(&self) -> Cow<'static, str> {
        self.state.name.clone()
    }

    fn id(&self) -> SystemId {
        self.state.id
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.state.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.state.resource_access
    }

    unsafe fn run_unsafe(
        &mut self,
        _: Self::In,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        loop {
            match self.channel.try_recv() {
                Ok(sync) => sync.run(self.state, world, resources),
                Err(async_channel::TryRecvError::Closed) => panic!(
                    "`AccessorRunnerSystem` called but all relevant accessors have been dropped"
                ),
                Err(async_channel::TryRecvError::Empty) => break,
            }
        }
        Some(())
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        <P::Fetch as FetchSystemParam>::init(&mut self.state, world, resources);
    }

    fn apply_buffers(&mut self, world: &mut World, resources: &mut Resources) {
        self.state.commands.get_mut().apply(world, resources);
        if let Some(ref commands) = self.state.arc_commands {
            let mut commands = commands.lock();
            commands.apply(world, resources);
        }
    }

    fn update_access(&mut self, world: &World) {
        self.state.update(world);
    }

    fn component_access(&self) -> &TypeAccess<TypeId> {
        &self.state.component_access
    }

    fn is_non_send(&self) -> bool {
        self.state.is_non_send
    }
}

pub trait AccessSystemsTuple: Send + Sync + 'static {
    fn update_access(
        &mut self,
        world: &World,
        archetype_component_access: &mut TypeAccess<ArchetypeComponent>,
        component_access: &mut TypeAccess<TypeId>,
        resource_access: &mut TypeAccess<TypeId>,
    );
    fn is_non_send(&self) -> bool;
    fn apply_buffers(&mut self, world: &mut World, resources: &mut Resources);
    fn initialize(&mut self, world: &mut World, resources: &mut Resources);
    unsafe fn run(&mut self, world: &World, resources: &Resources);
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
    pub(super) archetype_component_access: TypeAccess<ArchetypeComponent>,
    pub(super) component_access: TypeAccess<TypeId>,
    pub(super) resource_access: TypeAccess<TypeId>,
    pub(super) startup_future:
        Option<Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync + 'static>>,
}

impl<In, Out, Systems: AccessSystemsTuple> System for AsyncChainSystem<In, Out, Systems>
where
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    type In = In;
    type Out = Out;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn update_access(&mut self, world: &World) {
        self.archetype_component_access.clear();
        self.component_access.clear();
        self.resource_access.clear();
        self.inner_systems.update_access(
            world,
            &mut self.archetype_component_access,
            &mut self.component_access,
            &mut self.resource_access,
        );
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn component_access(&self) -> &TypeAccess<TypeId> {
        &self.component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn is_non_send(&self) -> bool {
        self.inner_systems.is_non_send()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: Self::In,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        self.inner_systems.run(world, resources);
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

    fn apply_buffers(&mut self, world: &mut World, resources: &mut Resources) {
        self.inner_systems.apply_buffers(world, resources);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        if let Some(fut) = self.startup_future.take() {
            let tp = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
            tp.spawn((fut)(tp.clone().0)).detach();
        }
        self.inner_systems.initialize(world, resources);
    }
}
