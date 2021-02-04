use async_channel::{Receiver, Sender};
use parking_lot::Mutex;
use std::{any::TypeId, borrow::Cow, marker::PhantomData, ops::Deref, sync::Arc};

use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;

use crate::{
    ArchetypeComponent, BoxedSystem, FetchSystemParam, Resources, System, SystemId, SystemParam,
    SystemState, TypeAccess, World,
};

pub trait AsyncSystem<T>: Sized {
    type TaskPool: Deref<Target = TaskPool>;

    fn systems(self) -> Vec<BoxedSystem>;
}

pub trait AsyncSystemWith<Params: SystemParam>: Send + Sync + Copy + 'static {}

pub trait Curry<P: SystemParam>: Send + Sync {
    fn init(&mut self, accessor: Accessor<P>) -> Option<BoxedFuture<'static, ()>>;
}

pub mod curries {
    use super::*;

    macro_rules! create_curries {
        ($name: ident, [$($typ: ident {$($nest: ident),*}),*]) => {
            #[allow(non_snake_case)]
            pub struct $name<Func, F, $($typ),*>
            where
                F: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> F + Send + Sync + Copy + 'static,
                $($typ: SystemParam + 'static),*
            {
                func: Func,
                $($typ: Option<Accessor<$typ>>),*
            }

            unsafe impl<Func, F, $($typ),*> Send for $name<Func, F, $($typ),*>
            where
                F: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> F + Send + Sync + Copy + 'static,
                $($typ: SystemParam + 'static),* {}

            unsafe impl<Func, F, $($typ),*> Sync for $name<Func, F, $($typ),*>
            where
                F: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> F + Send + Sync + Copy + 'static,
                $($typ: SystemParam + 'static),* {}

            #[allow(unused_parens)]
            impl<Func, $($typ),*, F> AsyncSystem<($($typ),*)> for Func
            where
                F: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> F + Send + Sync + Copy + 'static,
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
                        Box::new(AccessorRunnerSystem {
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
                impl_curry!($name, $typ, [$($nest),*]);
            )*
        };
    }

    macro_rules! impl_curry {
        ($name: ident, $field: ident, [$($typ: ident),*]) => {
            impl<Func, F, $($typ),*> Curry<$field> for $name<Func, F, $($typ),*>
            where
                F: std::future::Future<Output = ()> + Send + Sync + 'static,
                Func: Fn($(Accessor<$typ>),*) -> F + Send + Sync + Copy + 'static,
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

    create_curries!(Ax, [A { A }]);
    create_curries!(Bx, [A { A, B }, B { A, B }]);
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

pub struct AccessorRunnerSystem<P: SystemParam, C: Curry<P>> {
    state: SystemState,
    channel: Receiver<Box<dyn FnOnce(&SystemState, &World, &Resources) + Send + Sync>>,
    core: Arc<Mutex<C>>,
    _marker: OpaquePhantomData<P>,
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

impl<P: SystemParam + 'static, C: Curry<P> + 'static> System for AccessorRunnerSystem<P, C> {
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

#[cfg(test)]
mod test {
    use bevy_tasks::{AsyncComputeTaskPool, TaskPoolBuilder};

    use super::{Accessor, AsyncSystem};

    use crate::{Commands, IntoSystem, Query, Res, ResMut, Resources, Stage, SystemStage, World};

    use std::convert::TryInto;

    async fn async_system(mut accessor: Accessor<(Query<'_, (&u32, &i64)>, ResMut<'_, String>)>) {
        let mut x = None;
        let returns = accessor
            .access(move |(query, mut res)| {
                //
                *res = "Hi!".to_owned();
                for res in query.iter() {
                    match res {
                        (3, 5) | (7, -8) => (),
                        _ => unreachable!(),
                    }
                }
                assert_eq!(x, None);
                x = Some(3);
                x
            })
            .await;
        assert_eq!(returns, Some(3));
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
            .insert_resource(AsyncComputeTaskPool(
                TaskPoolBuilder::default()
                    .thread_name("Async Compute Task Pool".to_string())
                    .build(),
            ));

        commands.apply(&mut world, &mut resources);

        let [sync_1]: [_; 1] = async_system.systems().try_into().map_err(|_| ()).unwrap();

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
                })
                .system(),
            );

        stage.initialize(&mut world, &mut resources);
        // Crude hack to ensure the async system is fully initialized
        std::thread::sleep(std::time::Duration::from_millis(10));
        stage.run(&mut world, &mut resources);
    }
}
