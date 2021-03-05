//! This should never compile
//! ```compile_fail,E0308
//! use bevy_ecs::prelude::*;
//! thread_local! {
//!     static TEST: std::cell::RefCell<Option<ResMut<'static, String>>> = Default::default();
//! }
//! async fn compile_fail(mut access: Accessor<(ResMut<'_, String>,)>) {
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

mod exclusive_access;
mod parallel_access;

use async_channel::{Receiver, Sender};
use std::{
    borrow::Cow,
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use thiserror::Error;

use bevy_tasks::TaskPool;
use bevy_utils::BoxedFuture;

pub use exclusive_access::ExclusiveAccessor;
pub use parallel_access::Accessor;

use super::SystemId;

pub trait AsyncSystem<Marker, OutSystems>
where
    Self: Sized,
    <Self::Fut as Future>::Output: Send + Sync + 'static,
{
    type In: Send + Sync + 'static;
    type Fut: Future;

    fn systems(
        self,
    ) -> (
        OutSystems,
        AsyncSystemHandle<Self::In, <Self::Fut as Future>::Output>,
        Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
    );

    fn system(
        self,
    ) -> parallel_access::AsyncChainSystem<Self::In, <Self::Fut as Future>::Output, OutSystems>
    where
        OutSystems: parallel_access::AccessSystemsTuple,
    {
        let (inner_systems, handle, future) = self.systems();

        parallel_access::AsyncChainSystem {
            inner_systems,
            handle,
            return_handle: None,
            name: Cow::Borrowed(std::any::type_name::<Self>()),
            id: SystemId::new(),
            archetype_component_access: Default::default(),
            component_access: Default::default(),
            startup_future: Some(future),
        }
    }

    fn exclusive_system(self) -> exclusive_access::ExclusiveAsyncChainSystem<OutSystems>
    where
        OutSystems: exclusive_access::ExclusiveAccessSystemsTuple,
        Self::Fut: Future<Output = ()>,
        Self: AsyncSystem<Marker, OutSystems, In = ()>,
    {
        let (inner_systems, handle, future) = self.systems();

        exclusive_access::ExclusiveAsyncChainSystem {
            inner_systems,
            handle,
            return_handle: None,
            name: Cow::Borrowed(std::any::type_name::<Self>()),
            id: SystemId::new(),
            startup_future: Some(future),
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

pub trait AccessorTrait: Clone + Send + Sync {
    type AccessSystem;

    fn new() -> (Self, Self::AccessSystem);
}

// Implements AsyncSystem for async functions with up to 16 different accessors
#[doc(hidden)]
pub mod impls {
    use crate::{
        archetype::{Archetype, ArchetypeComponentId},
        query::Access,
        system::{ExclusiveSystem, In, System, SystemParam},
        world::World,
    };

    use super::*;

    pub struct SimpleAsyncMarker;
    pub struct InAsyncMarker;

    macro_rules! impl_async_system {
        ($($i: ident),*) => {
            impl<Func, $($i,)* Fut> AsyncSystem<(SimpleAsyncMarker, $($i,)*), ($($i::AccessSystem,)*)> for Func
            where
                Func: FnMut($($i,)*) -> Fut + Send + Sync + 'static,
                Fut: Future + Send + 'static,
                Fut::Output: Send + Sync + 'static,
                $($i: AccessorTrait + 'static,)*
            {
                type In = ();
                type Fut = Fut;
                #[allow(non_snake_case)]
                fn systems(
                    mut self,
                ) -> (
                    ($($i::AccessSystem,)*),
                    AsyncSystemHandle<(), Fut::Output>,
                    Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
                ) {
                    $(let $i = $i::new();)*
                    let (tx, rx) = async_channel::unbounded();
                    let systems = ( $( $i.1, )* );
                    $(let $i = $i.0;)*
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

            impl<Trigger, Func, $($i,)* Fut> AsyncSystem<(InAsyncMarker, Trigger, Fut, $($i,)*), ($($i::AccessSystem,)*)> for Func
            where
                Trigger: Send + Sync + 'static,
                Func: FnMut(In<Trigger>, $($i,)*) -> Fut + Send + Sync + 'static,
                Fut: Future + Send + 'static,
                Fut::Output: Send + Sync + 'static,
                $($i: AccessorTrait + 'static,)*
            {
                type In = Trigger;
                type Fut = Fut;
                #[allow(non_snake_case)]
                fn systems(
                    mut self,
                ) -> (
                    ($($i::AccessSystem,)*),
                    AsyncSystemHandle<Trigger, Fut::Output>,
                    Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync>,
                ) {
                    $(let $i = $i::new();)*
                    let (tx, rx) = async_channel::unbounded();
                    let systems = ( $( $i.1, )* );
                    $(let $i = $i.0;)*
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
            impl<$($i: SystemParam + 'static,)*> parallel_access::AccessSystemsTuple for ($(super::parallel_access::AccessorRunnerSystem<$i>,)*) {
                fn new_archetype(
                    &mut self,
                    archetype: &Archetype,
                    archetype_component_access: &mut Access<ArchetypeComponentId>,
                ) {
                   let ($($i,)*) = self;
                    $(
                        $i.new_archetype(archetype);
                        archetype_component_access.extend($i.archetype_component_access());
                    )*
                }
                fn is_send(&self) -> bool {
                    let ($($i,)*) = self;
                    $($i.is_send() &&)* true
                }
                fn apply_buffers(&mut self, world: &mut World) {
                    let ($($i,)*) = self;
                    $($i.apply_buffers(world);)*
                }
                fn initialize(&mut self, world: &mut World) {
                    let ($($i,)*) = self;
                    $($i.initialize(world);)*
                }
                unsafe fn run(&mut self, world: &World) {
                    let ($($i,)*) = self;
                    $($i.run_unsafe((), world);)*
                }
            }

            #[allow(unused)]
            #[allow(non_snake_case)]
            impl<$($i: ExclusiveSystem,)*> exclusive_access::ExclusiveAccessSystemsTuple for ($($i,)*) {
                fn initialize(&mut self, world: &mut World) {
                    let ($($i,)*) = self;
                    $($i.initialize(world);)*
                }
                fn run(&mut self, world: &mut World) {
                    let ($($i,)*) = self;
                    $($i.run(world);)*
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
    use super::{Accessor, AsyncSystem};
    use crate::{prelude::*, system::CommandQueue};
    use bevy_tasks::{AsyncComputeTaskPool, TaskPoolBuilder};

    async fn complex_async_system(
        mut access_1: Accessor<(Res<'_, u32>, ResMut<'_, String>)>,
        mut access_2: Accessor<(Res<'_, String>,)>,
    ) {
        loop {
            let mut x = None;
            access_1
                .access(|number: Res<'_, _>, mut res: ResMut<'_, _>| {
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

    async fn simple_async_system(
        mut accessor: Accessor<(Query<'_, (&'static u32, &'static i64)>,)>,
    ) {
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

        let mut cq = CommandQueue::default();
        let mut commands = Commands::new(&mut cq, &world);

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

        cq.apply(&mut world);

        let ((sync_1, sync_2), mut handle, future) = complex_async_system.systems();
        let tp = world.get_resource_mut::<AsyncComputeTaskPool>().unwrap();
        tp.spawn((future)(tp.clone().0)).detach();
        drop(tp);
        handle.fire(());
        let mut stage = SystemStage::parallel();
        stage
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hello", &*string);
                    // This makes the test more consistently
                    std::thread::sleep(std::time::Duration::from_millis(10));
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
            .add_system(
                simple_async_system
                    .system()
                    .chain((|_: In<_>| {}).system())
                    .after("4"),
            );

        stage.run(&mut world);
    }
}
