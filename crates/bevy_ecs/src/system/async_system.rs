use async_channel::{Receiver, Sender};
use parking_lot::Mutex;
use std::{any::TypeId, borrow::Cow, marker::PhantomData, ops::Deref, sync::Arc};

use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;

use crate::{
    ArchetypeComponent, BoxedSystem, FetchSystemParam, Resources, System, SystemId, SystemParam,
    SystemState, TypeAccess, World,
};

pub trait AsyncSystem<T1, T2>: Sized {
    type TaskPool: Deref<Target = TaskPool>;

    fn systems(self) -> Vec<BoxedSystem>;
}

pub trait Curry<P: SystemParam, Marker>: Send + Sync {
    fn init(&mut self, accessor: Accessor<P>) -> Option<BoxedFuture<'static, ()>>;
}

pub struct Accessor<P: SystemParam> {
    channel: Sender<Box<dyn FnOnce(&SystemState, &World, &Resources) + Send + Sync>>,
    _marker: OpaquePhantomData<P>,
}

impl<P: SystemParam> Accessor<P> {
    pub async fn access<F, R: Send + 'static>(&mut self, sync: F) -> R
    where
        // Removing the 'static here would allow removing the
        // transmutes, but its currently not possible due to an ICE.
        F: FnOnce(<P::Fetch as FetchSystemParam<'static>>::Item) -> R + Send + Sync + 'static,
    {
        let (tx, rx) = async_channel::bounded(1);
        self.channel
            .send(Box::new(move |state, world, resources| {
                // Safe: the sent closure is executed inside run_unsafe, which provides the correct guarantees.
                match unsafe {
                    P::Fetch::get_param(
                        std::mem::transmute::<_, &'static _>(state),
                        std::mem::transmute::<_, &'static _>(world),
                        std::mem::transmute::<_, &'static _>(resources),
                    )
                } {
                    Some(params) => tx.try_send(sync(params)).unwrap(),
                    None => (),
                }
            }))
            .await
            .unwrap();
        rx.recv().await.unwrap()
    }
}

pub struct AccessorRunnerSystem<P: SystemParam, C: Curry<P, Marker>, Marker> {
    state: SystemState,
    channel: Receiver<Box<dyn FnOnce(&SystemState, &World, &Resources) + Send + Sync>>,
    core: Arc<Mutex<C>>,
    _marker: OpaquePhantomData<(P, Marker)>,
}

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

impl<P: SystemParam + 'static, C: Curry<P, Marker> + 'static, Marker: 'static> System
    for AccessorRunnerSystem<P, C, Marker>
{
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
        match self.channel.try_recv() {
            Ok(sync) => (sync)(&self.state, world, resources),
            Err(async_channel::TryRecvError::Closed) => {
                let (tx, rx) = async_channel::unbounded();
                if let Some(f) = self.core.lock().init(Accessor {
                    channel: tx,
                    _marker: Default::default(),
                }) {
                    let executor = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
                    executor.spawn(f).detach();
                }
                self.channel = rx;
            }
            _ => (),
        }

        Some(())
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        <P::Fetch as FetchSystemParam>::init(&mut self.state, world, resources);

        let (tx, rx) = async_channel::unbounded();
        if let Some(f) = self.core.lock().init(Accessor {
            channel: tx,
            _marker: Default::default(),
        }) {
            let executor = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
            executor.spawn(f).detach();
        }
        self.channel = rx;
    }

    fn update(&mut self, _world: &World) {}

    fn thread_local_execution(&self) -> crate::ThreadLocalExecution {
        crate::ThreadLocalExecution::NextFlush
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        // SAFE: this is called with unique access to SystemState
        unsafe {
            (&mut *self.state.commands.get()).apply(world, resources);
        }
        if let Some(ref commands) = self.state.arc_commands {
            let mut commands = commands.lock();
            commands.apply(world, resources);
        }
    }
}

// Implements AsyncSystem for async functions with up to 6 different accessors
#[doc(hidden)]
pub mod curries {
    use super::*;

    macro_rules! create_curries {
        ($name: ident, $seperator: ident, [$(($inner_seperator: ident, $typ: ident {$($nest: ident),*})),*]) => {
            #[allow(non_snake_case)]
            pub struct $name<Func, Fut, $($typ),*>
            where
                Fut: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> Fut + Send + Sync + Copy + 'static,
                $($typ: SystemParam + 'static),*
            {
                func: Func,
                $($typ: Option<Accessor<$typ>>),*
            }

            pub struct $seperator;

            #[allow(unused_parens)]
            impl<Func, $($typ),*, Fut> AsyncSystem<($($typ),*), $seperator> for Func
            where
                Fut: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> Fut + Send + Sync + Copy + 'static,
                $($typ: SystemParam + 'static),*
            {
                type TaskPool = AsyncComputeTaskPool;

                fn systems(self) -> Vec<BoxedSystem> {
                    let curry = $name {
                        func: self,
                        $(
                            $typ: None
                        ),*
                    };
                    let arc = Arc::new(Mutex::new(curry));
                    vec![$(
                        Box::new(AccessorRunnerSystem::<_, _, $inner_seperator> {
                            state: {
                                let _: $typ;
                                let mut resource_access = TypeAccess::default();
                                resource_access.add_write(TypeId::of::<Self::TaskPool>());
                                SystemState {
                                    name: std::any::type_name::<Self>().into(),
                                    archetype_component_access: TypeAccess::default(),
                                    resource_access: resource_access,
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
                            channel: async_channel::bounded(1).1,
                            core: arc.clone(),
                            _marker: Default::default(),
                        })
                    ),*]
                }
            }

            $(
                impl_curry!($name, $typ, $inner_seperator [$($nest),*]);
            )*
        };
    }

    macro_rules! impl_curry {
        ($name: ident, $field: ident, $seperator: ident [$($typ: ident),*]) => {
            impl<Func, Fut, $($typ),*> Curry<$field, $seperator> for $name<Func, Fut, $($typ),*>
            where
                Fut: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> Fut + Send + Sync + Copy + 'static,
                $($typ: SystemParam + 'static),*
            {
                fn init(&mut self, accessor: Accessor<$field>) -> Option<BoxedFuture<'static, ()>> {
                    self.$field = Some(accessor);
                    $(
                        if self.$typ.is_none() {
                            return None;
                        }
                    )*

                    Some(Box::pin((self.func)($(self.$typ.take()?),*)))
                }
            }
        };
    }

    create_curries!(Ax, S1, [(S1, A { A })]);
    create_curries!(Bx, S2, [(S1, A { A, B }), (S2, B { A, B })]);
    create_curries!(
        Cx,
        S3,
        [
            (S1, A { A, B, C }),
            (S2, B { A, B, C }),
            (S3, C { A, B, C })
        ]
    );
    create_curries!(
        Dx,
        S4,
        [
            (S1, A { A, B, C, D }),
            (S2, B { A, B, C, D }),
            (S3, C { A, B, C, D }),
            (S4, D { A, B, C, D })
        ]
    );
    create_curries!(
        Ex,
        S5,
        [
            (S1, A { A, B, C, D, E }),
            (S2, B { A, B, C, D, E }),
            (S3, C { A, B, C, D, E }),
            (S4, D { A, B, C, D, E }),
            (S5, E { A, B, C, D, E })
        ]
    );
    create_curries!(
        Fx,
        S6,
        [
            (S1, A { A, B, C, D, E, F }),
            (S2, B { A, B, C, D, E, F }),
            (S3, C { A, B, C, D, E, F }),
            (S4, D { A, B, C, D, E, F }),
            (S5, E { A, B, C, D, E, F }),
            (S6, F { A, B, C, D, E, F })
        ]
    );
}

// Implements SimpleAsyncSystem for async functions with a single accessor
#[doc(hidden)]
mod simple_async_system {
    use super::*;
    pub struct SimpleCurry<P, F, Func>(Func, OpaquePhantomData<P>)
    where
        P: SystemParam + 'static,
        F: std::future::Future<Output = ()> + Send + Sync + 'static,
        Func: Fn(Accessor<P>) -> F + Send + Sync + Copy + 'static;

    impl<P, F, Func> Curry<P, ()> for SimpleCurry<P, F, Func>
    where
        P: SystemParam + 'static,
        F: std::future::Future<Output = ()> + Send + Sync + 'static,
        Func: Fn(Accessor<P>) -> F + Send + Sync + Copy + 'static,
    {
        fn init(&mut self, accessor: Accessor<P>) -> Option<BoxedFuture<'static, ()>> {
            Some(Box::pin((self.0)(accessor)))
        }
    }
    pub trait SimpleAsyncSystem<P, F, Func>
    where
        P: SystemParam + 'static,
        F: std::future::Future<Output = ()> + Send + Sync + 'static,
        Func: Fn(Accessor<P>) -> F + Send + Sync + Copy + 'static,
    {
        fn system(self) -> AccessorRunnerSystem<P, SimpleCurry<P, F, Func>, ()>;
    }

    impl<P, F, Func> SimpleAsyncSystem<P, F, Func> for Func
    where
        P: SystemParam + 'static,
        F: std::future::Future<Output = ()> + Send + Sync + 'static,
        Func: Fn(Accessor<P>) -> F + Send + Sync + Copy + 'static,
    {
        fn system(self) -> AccessorRunnerSystem<P, SimpleCurry<P, F, Func>, ()> {
            AccessorRunnerSystem::<_, _, ()> {
                state: {
                    let mut resource_access = TypeAccess::default();
                    resource_access.add_write(TypeId::of::<AsyncComputeTaskPool>());
                    SystemState {
                        name: std::any::type_name::<Self>().into(),
                        archetype_component_access: TypeAccess::default(),
                        resource_access,
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
                channel: async_channel::bounded(1).1,
                core: Arc::new(Mutex::new(SimpleCurry(self, Default::default()))),
                _marker: Default::default(),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use bevy_tasks::{AsyncComputeTaskPool, TaskPoolBuilder};

    use super::{simple_async_system::SimpleAsyncSystem, Accessor, AsyncSystem};

    use crate::{Commands, IntoSystem, Query, Res, ResMut, Resources, Stage, SystemStage, World};

    use std::convert::TryInto;

    async fn complex_async_system(
        mut access_1: Accessor<(Res<'_, u32>, ResMut<'_, String>)>,
        mut access_2: Accessor<Res<'_, String>>,
    ) {
        let mut x = None;
        let returns = access_1
            .access(move |(number, mut res)| {
                //
                *res = "Hi!".to_owned();
                assert_eq!(x, None);
                x = Some(*number);
                x
            })
            .await;
        assert_eq!(returns, Some(3));

        access_2
            .access(|res| {
                assert_eq!("Hi!", &*res);
            })
            .await;
    }

    async fn simple_async_system(mut accessor: Accessor<Query<'_, (&u32, &i64)>>) {
        accessor
            .access(|query| {
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

        let [sync_1, sync_2]: [_; 2] = complex_async_system
            .systems()
            .try_into()
            .map_err(|_| ())
            .unwrap();

        let mut stage = SystemStage::serial();
        stage
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hello", &*string);
                })
                .system(),
            )
            .add_system_boxed(sync_1)
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hi!", &*string);
                    // Crude hack to ensure the async system moves along
                    std::thread::sleep(std::time::Duration::from_millis(1));
                })
                .system(),
            )
            .add_system_boxed(sync_2)
            .add_system(simple_async_system.system());

        stage.initialize(&mut world, &mut resources);
        // Crude hack to ensure the async system is fully initialized
        std::thread::sleep(std::time::Duration::from_millis(10));
        stage.run(&mut world, &mut resources);
    }
}
