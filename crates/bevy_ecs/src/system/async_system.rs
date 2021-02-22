//! This should never compile
//! ```compile_fail,E0759
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
    self,
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
    ArchetypeComponent, BoxedSystem, FetchSystemParam, Resources, System, SystemId, SystemParam,
    SystemState, TypeAccess, World,
};

// this is stable on nightly, and will land on 2021-03-25 :)
pub trait AsyncSystem<In, Params, Fut: Future, Marker, const ACCESSOR_COUNT: usize>
where
    Self: Sized,
    In: Send + Sync + 'static,
    Fut::Output: Send + Sync + 'static,
{
    fn systems(
        self,
    ) -> (
        [BoxedSystem; ACCESSOR_COUNT],
        AsyncSystemHandle<In, Fut::Output>,
        Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
    );

    fn system(self) -> Box<dyn System<In = In, Out = Fut::Output>> {
        let (innner_systems, handle, startup_future) = self.systems();

        Box::new(AsyncChainSystem {
            innner_systems,
            handle,
            startup_future: Some(startup_future),
            return_handle: None,
            name: Cow::Owned(format!("Async Chain({})", std::any::type_name::<Self>())),
            id: SystemId::new(),
            archetype_component_access: Default::default(),
            component_access: Default::default(),
            resource_access: Default::default(),
        })
    }
}

struct AsyncChainSystem<In, Out, const SC: usize>
where
    Out: Send + Sync + 'static,
{
    innner_systems: [BoxedSystem; SC],
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

impl<In, Out, const SC: usize> System for AsyncChainSystem<In, Out, SC>
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
        for system in self.innner_systems.iter_mut() {
            system.update_access(world);
            self.archetype_component_access
                .extend(system.archetype_component_access());
            self.component_access.extend(system.component_access());
            self.resource_access.extend(system.resource_access());
        }
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
        self.innner_systems.iter().any(|s| s.is_non_send())
    }

    unsafe fn run_unsafe(
        &mut self,
        input: Self::In,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        for system in self.innner_systems.iter_mut() {
            system.run_unsafe((), world, resources);
        }

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
        for system in self.innner_systems.iter_mut() {
            system.apply_buffers(world, resources);
        }
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        if let Some(fut) = self.startup_future.take() {
            let tp = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
            tp.spawn((fut)(tp.clone().0)).detach();
        }
        for system in self.innner_systems.iter_mut() {
            system.initialize(world, resources);
        }
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

pub struct Accessor<P: SystemParam> {
    channel: Sender<Box<dyn GenericAccess>>,
    _marker: OpaquePhantomData<P>,
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
// This trait adds a middle-man that helps rustc figure out lifetime bounds, and avoids an ICE
// https://github.com/rust-lang/rust/issues/74261
pub trait AccessFn<'a, 'env, P: SystemParam, Out> {
    fn call(self: Box<Self>, v: <P::Fetch as FetchSystemParam<'a>>::Item) -> Out;
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

impl<'env, P: SystemParam, Out: Send + Sync + 'static> GenericAccess for Access<'env, P, Out> {
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

impl<'env, P: SystemParam + 'env, R: Send + Sync + 'static> Future for AccessFuture<'env, P, R> {
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
        ($param_count: literal, $($i: ident),*) => {
            impl<Func, $($i,)* Fut> AsyncSystem<(), ($($i,)*), Fut, SimpleAsyncMarker, $param_count> for Func
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
                    [BoxedSystem; $param_count],
                    AsyncSystemHandle<(), Fut::Output>,
                    Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
                ) {
                    $(let $i = AccessorRunnerSystem::<$i>::new();)*
                    let (tx, rx) = async_channel::unbounded();
                    let boxes = [ $( Box::new($i.0) as BoxedSystem, )* ];
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
                    (boxes, handle, Box::new(f))
                }
            }

            impl<Trigger, Func, $($i,)* Fut> AsyncSystem<Trigger, ($($i,)*), Fut, InAsyncMarker, $param_count> for Func
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
                    [BoxedSystem; $param_count],
                    AsyncSystemHandle<Trigger, Fut::Output>,
                    Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
                ) {
                    $(let $i = AccessorRunnerSystem::<$i>::new();)*
                    let (tx, rx) = async_channel::unbounded();
                    let boxes = [ $( Box::new($i.0) as BoxedSystem, )* ];
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
                    (boxes, handle, Box::new(f))
                }
            }
        };
    }

    impl_async_system!(0,);
    impl_async_system!(1, A);
    impl_async_system!(2, A, B);
    impl_async_system!(3, A, B, C);
    impl_async_system!(4, A, B, C, D);
    impl_async_system!(5, A, B, C, D, E);
    impl_async_system!(6, A, B, C, D, E, F);
    impl_async_system!(7, A, B, C, D, E, F, G);
    impl_async_system!(8, A, B, C, D, E, F, G, H);
    impl_async_system!(9, A, B, C, D, E, F, G, H, I);
    impl_async_system!(10, A, B, C, D, E, F, G, H, I, J);
    impl_async_system!(11, A, B, C, D, E, F, G, H, I, J, K);
    impl_async_system!(12, A, B, C, D, E, F, G, H, I, J, K, L);
    impl_async_system!(13, A, B, C, D, E, F, G, H, I, J, K, L, M);
    impl_async_system!(14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
    impl_async_system!(15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
    impl_async_system!(16, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
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

        let ([sync_1, sync_2], mut handle, future) = complex_async_system.systems();
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
