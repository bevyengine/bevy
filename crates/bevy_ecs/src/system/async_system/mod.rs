//! This should never compile
//! ```compile_fail,E0308
//! use bevy_ecs::prelude::*;
//! thread_local! {
//!     static TEST: std::cell::RefCell<Option<ResMut<'static, String>>> = Default::default();
//! }
//! async fn compile_fail(mut access: Accessor<ResMut<'_, String>>) {
//!     access
//!         .access(|res: ResMut<'_, _>| {
//!             TEST.with(|mut cell| {
//!                 let test = &mut *cell.borrow_mut();
//!                 test.replace(res);
//!             });
//!         })
//!         .await;
//! }
//! ```

use async_channel::{Receiver, Sender};
use futures_lite::pin;
use parking_lot::Mutex;
use std::{
    any::TypeId,
    borrow::Cow,
    future::Future,
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Poll, Waker},
};
use thiserror::Error;

use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;

use crate::{
    ArchetypeComponent, FetchSystemParam, Resources, System, SystemId, SystemParam, SystemState,
    TypeAccess, World,
};

pub trait AsyncSystem<In, Fut: Future, Marker, OutSystems>
where
    Self: Sized,
    In: Send + Sync + 'static,
    Fut::Output: Send + Sync + 'static,
{
    fn systems(
        self,
    ) -> (
        OutSystems,
        AsyncSystemHandle<In, Fut::Output>,
        Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
    );

    fn system(self) -> AsyncChainSystem<In, Fut::Output, OutSystems>
    where
        OutSystems: AccessSystemsTuple,
    {
        let (inner_systems, handle, future) = self.systems();

        AsyncChainSystem {
            inner_systems,
            handle,
            return_handle: None,
            name: Cow::Borrowed(std::any::type_name::<Self>()),
            id: SystemId::new(),
            archetype_component_access: Default::default(),
            component_access: Default::default(),
            resource_access: Default::default(),
            startup_future: Some(future),
        }
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
    inner_systems: Systems,
    handle: AsyncSystemHandle<In, Out>,
    return_handle: Option<AsyncSystemOutput<Out>>,
    name: Cow<'static, str>,
    id: SystemId,
    archetype_component_access: TypeAccess<ArchetypeComponent>,
    component_access: TypeAccess<TypeId>,
    resource_access: TypeAccess<TypeId>,
    startup_future:
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

pub struct AsyncSystemHandle<In, Out> {
    tx: Sender<(In, Sender<Out>)>,
    system_count: Arc<AtomicUsize>,
}

pub struct AsyncSystemOutput<Out: Send + Sync + 'static> {
    rx: Receiver<Out>,
    done: bool,
    counter_ref: Arc<AtomicUsize>,
}

impl<In, Out> Clone for AsyncSystemHandle<In, Out> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            system_count: self.system_count.clone(),
        }
    }
}

impl<In: Send + Sync + 'static, Out: Send + Sync + 'static> AsyncSystemHandle<In, Out> {
    pub fn fire(&mut self, trigger: In) -> AsyncSystemOutput<Out> {
        let (tx, rx) = async_channel::bounded(1);
        self.tx.try_send((trigger, tx)).unwrap();
        self.system_count.fetch_add(1, Ordering::Relaxed);
        let counter_ref = self.system_count.clone();
        AsyncSystemOutput {
            rx,
            done: false,
            counter_ref,
        }
    }

    pub fn active_system_count(&self) -> usize {
        self.system_count.load(Ordering::Relaxed)
    }
}

impl<Out: Send + Sync + 'static> AsyncSystemOutput<Out> {
    pub fn get(&mut self) -> Result<Out, AsyncSystemOutputError> {
        match self.rx.try_recv() {
            Ok(v) => {
                self.counter_ref.fetch_sub(1, Ordering::Relaxed);
                Ok(v)
            }
            Err(async_channel::TryRecvError::Empty) => Err(if self.done {
                AsyncSystemOutputError::OutputMoved
            } else {
                AsyncSystemOutputError::SystemNotFinished
            }),
            Err(async_channel::TryRecvError::Closed) => panic!(),
        }
    }

    pub fn check(&self) -> bool {
        !self.rx.is_empty()
    }
}

#[derive(Debug, Error)]
pub enum AsyncSystemOutputError {
    #[error("The output of this system call has already been taken")]
    OutputMoved,
    #[error("The system has not finished")]
    SystemNotFinished,
}

pub trait AccessorTrait: Sized {
    type AccessSystem;

    fn new() -> (Self, Self::AccessSystem);
}

pub struct Accessor<P: SystemParam> {
    channel: Sender<Box<dyn GenericAccess>>,
    _marker: OpaquePhantomData<P>,
}

pub struct ExclusiveAccessor {
    
}

impl<P: SystemParam> Clone for Accessor<P> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            _marker: Default::default(),
        }
    }
}

pub trait AccessFn<'a, 'env, P: SystemParam, Out> {
    fn call(self: Box<Self>, v: <P::Fetch as FetchSystemParam<'a>>::Item) -> Out;
}

pub trait ExclusiveAccessFn<'a, 'env, Out> {
    fn call(self: Box<Self>, world: &'a mut World, resources: &'a mut Resources) -> Out;
}

impl<'a, 'env, Out, P, F> AccessFn<'a, 'env, P, Out> for F
where
    P: SystemParam,
    F: FnOnce(P) -> Out + 'env,
    F: FnOnce(<P::Fetch as FetchSystemParam<'a>>::Item) -> Out + 'env,
{
    fn call(self: Box<Self>, v: <P::Fetch as FetchSystemParam<'a>>::Item) -> Out {
        self(v)
    }
}

impl<'a, 'env, Out, F> ExclusiveAccessFn<'a, 'env, Out> for F
where
    F: FnOnce(&'a mut World, &'a mut Resources) -> Out + 'env,
{
    fn call(self: Box<Self>, world: &'a mut World, resources: &'a mut Resources) -> Out {
        self(world, resources)
    }
}

impl<P: SystemParam> Accessor<P> {
    pub fn access<'env, R: Send + Sync + 'static>(
        &mut self,
        sync: impl for<'a> AccessFn<'a, 'env, P, R> + Send + Sync + 'env,
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

struct Access<'env, P: SystemParam, Out> {
    inner: Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'a, 'env, P, Out> + Send + Sync + 'env>>>>,
    tx: Sender<Out>,
    waker: Waker,
}

trait GenericAccess: Send + Sync {
    fn run(self: Box<Self>, state: &SystemState, world: &World, resources: &Resources);
}

impl<'env, P: SystemParam, Out: Send + Sync + 'env> GenericAccess for Access<'env, P, Out> {
    fn run(self: Box<Self>, state: &SystemState, world: &World, resources: &Resources) {
        if let Some(params) = unsafe { P::Fetch::get_param(state, world, resources) } {
            if let Some(sync) = self.inner.lock().take() {
                self.tx.try_send(sync.call(params)).unwrap();
            }
        }
        self.waker.wake();
    }
}

enum AccessFutureState<'env, P, R> {
    FirstPoll {
        boxed: Box<dyn for<'a> AccessFn<'a, 'env, P, R> + Send + Sync + 'env>,
        tx: Sender<Box<dyn GenericAccess>>,
    },
    WaitingForCompletion(
        Receiver<R>,
        Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'a, 'env, P, R> + Send + Sync + 'env>>>>,
    ),
}

pub struct AccessFuture<'env, P: SystemParam, R> {
    state: AccessFutureState<'env, P, R>,
}

impl<'env, P: SystemParam + 'env, R: Send + Sync + 'env> Future for AccessFuture<'env, P, R> {
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

impl<'env, P: SystemParam, R> Drop for AccessFuture<'env, P, R> {
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

impl<P: SystemParam + 'static> AccessorRunnerSystem<P> {
    fn new() -> (Self, Accessor<P>) {
        let (tx, rx) = async_channel::unbounded();
        (
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
            Accessor {
                channel: tx,
                _marker: Default::default(),
            },
        )
    }
}

#[derive(Clone)]
struct OpaquePhantomData<T> {
    _phantom: PhantomData<T>,
}

unsafe impl<T> Send for OpaquePhantomData<T> {}
unsafe impl<T> Sync for OpaquePhantomData<T> {}

impl<T> Default for OpaquePhantomData<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
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
                Ok(sync) => sync.run(&mut self.state, world, resources),
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

// Implements AsyncSystem for async functions with up to 16 different accessors
#[doc(hidden)]
pub mod impls {
    use crate::In;

    use super::*;

    pub struct SimpleAsyncMarker;
    pub struct InAsyncMarker;

    macro_rules! impl_async_system {
        ($($i: ident),*) => {
            impl<Func, $($i,)* Fut> AsyncSystem<(), Fut, SimpleAsyncMarker, ($(AccessorRunnerSystem<$i>,)*)> for Func
            where
                Func: FnMut($(Accessor<$i>,)*) -> Fut + Send + Sync + 'static,
                Fut: Future + Send + 'static,
                Fut::Output: Send + Sync + 'static,
                $($i: SystemParam + 'static,)*
            {
                #[allow(non_snake_case)]
                fn systems(
                    mut self,
                ) -> (
                    ($(AccessorRunnerSystem<$i>,)*),
                    AsyncSystemHandle<(), Fut::Output>,
                    Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
                ) {
                    $(let $i = AccessorRunnerSystem::<$i>::new();)*
                    let (tx, rx) = async_channel::unbounded();
                    let systems = ( $( $i.0, )* );
                    $(let $i = $i.1;)*
                    let f = |tp: TaskPool| Box::pin(async move {
                        while let Ok((_, return_pipe)) = rx.recv().await {
                            let future = (self)($( $i.clone(), )*);
                            let return_pipe: Sender<_> = return_pipe;
                            tp.spawn(async move {
                                return_pipe.send(future.await).await.unwrap();
                            })
                            .detach();
                        }
                    }) as BoxedFuture<'static, ()>;
                    let handle = AsyncSystemHandle { tx, system_count: Default::default()  };
                    (systems, handle, Box::new(f))
                }
            }

            impl<Trigger, Func, $($i,)* Fut> AsyncSystem<Trigger, Fut, InAsyncMarker, ($(AccessorRunnerSystem<$i>,)*)> for Func
            where
                Trigger: Send + Sync + 'static,
                Func: FnMut(In<Trigger>, $(Accessor<$i>,)*) -> Fut + Send + Sync + 'static,
                Fut: Future + Send + 'static,
                Fut::Output: Send + Sync + 'static,
                $($i: SystemParam + 'static,)*
            {
                #[allow(non_snake_case)]
                fn systems(
                    mut self,
                ) -> (
                    ($(AccessorRunnerSystem<$i>,)*),
                    AsyncSystemHandle<Trigger, Fut::Output>,
                    Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
                ) {
                    $(let $i = AccessorRunnerSystem::<$i>::new();)*
                    let (tx, rx) = async_channel::unbounded();
                    let systems = ( $( $i.0, )* );
                    $(let $i = $i.1;)*
                    let f = |tp: TaskPool| Box::pin(async move {
                        while let Ok((input, return_pipe)) = rx.recv().await {
                            let future = (self)(In(input), $( $i.clone(), )*);
                            let return_pipe: Sender<_> = return_pipe;
                            tp.spawn(async move {
                                return_pipe.send(future.await).await.unwrap();
                            })
                            .detach();
                        }
                    }) as BoxedFuture<'static, ()>;
                    let handle = AsyncSystemHandle { tx, system_count: Default::default() };
                    (systems, handle, Box::new(f))
                }
            }

            #[allow(unused)]
            #[allow(non_snake_case)]
            impl<$($i: SystemParam + 'static,)*> AccessSystemsTuple for ($(AccessorRunnerSystem<$i>,)*) {
                fn update_access(
                    &mut self,
                    world: &World,
                    archetype_component_access: &mut TypeAccess<ArchetypeComponent>,
                    component_access: &mut TypeAccess<TypeId>,
                    resource_access: &mut TypeAccess<TypeId>,
                ) {
                   let ($($i,)*) = self;
                    $(
                        $i.update_access(world);
                        archetype_component_access.extend($i.archetype_component_access());
                        component_access.extend($i.component_access());
                        resource_access.extend($i.resource_access());
                    )*
                }
                fn is_non_send(&self) -> bool {
                    let ($($i,)*) = self;
                    $($i.is_non_send() ||)* false
                }
                fn apply_buffers(&mut self, world: &mut World, resources: &mut Resources) {
                    let ($($i,)*) = self;
                    $($i.apply_buffers(world, resources);)*
                }
                fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
                    let ($($i,)*) = self;
                    $($i.initialize(world, resources);)*
                }
                unsafe fn run(&mut self, world: &World, resources: &Resources) {
                    let ($($i,)*) = self;
                    $($i.run_unsafe((), world, resources);)*
                }
            }
        };
    }

    impl_async_system!();
    impl_async_system!(A);
    impl_async_system!(A, B);
    impl_async_system!(A, B, C);
    impl_async_system!(A, B, C, D);
    impl_async_system!(A, B, C, D, E);
    impl_async_system!(A, B, C, D, E, F);
    impl_async_system!(A, B, C, D, E, F, G);
    impl_async_system!(A, B, C, D, E, F, G, H);
    impl_async_system!(A, B, C, D, E, F, G, H, I);
    impl_async_system!(A, B, C, D, E, F, G, H, I, J);
    impl_async_system!(A, B, C, D, E, F, G, H, I, J, K);
    impl_async_system!(A, B, C, D, E, F, G, H, I, J, K, L);
    impl_async_system!(A, B, C, D, E, F, G, H, I, J, K, L, M);
    impl_async_system!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
    impl_async_system!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
    impl_async_system!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
}

#[cfg(test)]
mod test {
    use bevy_tasks::{AsyncComputeTaskPool, TaskPoolBuilder};

    use super::{Accessor, AsyncSystem};

    use crate::{
        Commands, IntoSystem, ParallelSystemDescriptorCoercion, Query, Res, ResMut, Resources,
        Stage, SystemStage, World,
    };

    async fn complex_async_system(
        mut access_1: Accessor<(Res<'_, u32>, ResMut<'_, String>)>,
        mut access_2: Accessor<Res<'_, String>>,
    ) {
        loop {
            let mut x = None;
            access_1
                .access(|(number, mut res): (Res<'_, _>, ResMut<'_, _>)| {
                    *res = "Hi!".to_owned();
                    assert_eq!(x, None);
                    x = Some(*number);
                })
                .await;
            assert_eq!(x, Some(3));

            access_2
                .access(|res: Res<'_, _>| {
                    assert_eq!("Hi!", &*res);
                })
                .await;
        }
    }

    async fn simple_async_system(mut accessor: Accessor<Query<'_, (&u32, &i64)>>) {
        accessor
            .access(|query: Query<'_, (&u32, &i64)>| {
                for res in query.iter() {
                    match res {
                        (3, 5) | (7, -8) => (),
                        _ => unreachable!(),
                    }
                }
            })
            .await;
    }

    #[test]
    fn run_async_system() {
        let mut world = World::new();
        let mut resources = Resources::default();

        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());

        commands
            .spawn((3u32, 5i64))
            .spawn((7u32, -8i64))
            .insert_resource("Hello".to_owned())
            .insert_resource(3u32)
            .insert_resource(AsyncComputeTaskPool(
                TaskPoolBuilder::default()
                    .thread_name("Async Compute Task Pool".to_string())
                    .build(),
            ));

        commands.apply(&mut world, &mut resources);

        let ((sync_1, sync_2), mut handle, future) = complex_async_system.systems();
        let tp = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
        tp.spawn((future)(tp.clone().0)).detach();
        drop(tp);
        handle.fire(());
        let mut stage = SystemStage::parallel();
        stage
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hello", &*string);
                })
                .system()
                .label("1"),
            )
            .add_system(sync_1.label("2").after("1"))
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hi!", &*string);
                })
                .system()
                .label("3")
                .after("2"),
            )
            .add_system(sync_2.label("4").after("3"))
            .add_system(simple_async_system.system().after("4"));

        stage.run(&mut world, &mut resources);
    }
}
