use std::{any::TypeId, borrow::Cow, marker::PhantomData, ops::Deref};

use async_channel::{Receiver, Sender};

use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;

use crate::{ArchetypeComponent, BoxedSystem, FetchSystemParam, Resources, System, SystemId, SystemParam, SystemState, TypeAccess, World};

pub trait AsyncSystem: Sized {
    type TaskPool: Deref<Target = TaskPool>;

     fn system(self, primary: bool) -> Vec<(&'static str, BoxedSystem)>;
}

pub trait AsyncSystemWith<Params: SystemParam>: Send + Sync + Copy + 'static {
    fn init(self, accessor: Accessor<Params>) -> Option<BoxedFuture<'static, ()>>;
}

impl<P, F, Func> AsyncSystemWith<P> for Func
where
    P: SystemParam + 'static,
    F: std::future::Future<Output = ()> + Send + Sync + 'static,
    Func: Fn(Accessor<P>) -> F + Send + Sync + Copy + 'static,
{
    fn init(self, accessor: Accessor<P>) -> Option<BoxedFuture<'static, ()>> {
        todo!()
    }
    // type TaskPool = bevy_tasks::AsyncComputeTaskPool;

    // fn run(self, accessor: Accessor<P>) -> BoxedFuture<'static, ()> {
    //     Box::pin((self)(accessor))
    // }

    // fn system(self, primary: bool) -> AsyncSystemSyncSystem<P, Self> {
    //     let mut resource_access = TypeAccess::default();
    //     resource_access.add_write(TypeId::of::<Self::TaskPool>());

    //     AsyncSystemSyncSystem::<P, Self> {
    //         state: SystemState {
    //             name: std::any::type_name::<Self>().into(),
    //             archetype_component_access: TypeAccess::default(),
    //             resource_access,
    //             local_resource_access: TypeAccess::default(),
    //             id: SystemId::new(),
    //             commands: Default::default(),
    //             arc_commands: Default::default(),
    //             current_query_index: Default::default(),
    //             query_archetype_component_accesses: Vec::new(),
    //             query_accesses: Vec::new(),
    //             query_type_names: Vec::new(),
    //         },
    //         channel: async_channel::bounded(1).1,
    //         core: if primary { Some(self) } else { None },
    //         _marker: Default::default(),
    //     }
    // }
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

pub struct AsyncSystemSyncSystem<P: SystemParam, S: AsyncSystemWith<P>> {
    state: SystemState,
    channel: Receiver<Box<dyn FnOnce(&SystemState, &World, &Resources) + Send + Sync>>,
    core: Option<S>,
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

impl<P: SystemParam + 'static, S: AsyncSystemWith<P> + Copy> System
    for AsyncSystemSyncSystem<P, S>
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
                if let Some(core) = self.core {
                    let (tx, rx) = async_channel::unbounded();
                    if let Some(f) = core.init(Accessor {
                        channel: tx,
                        _marker: Default::default(),
                    }) {
                        let executor = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
                        executor.spawn(f).detach();
                    }
                    self.channel = rx;
                }
            }
            _ => (),
        }

        Some(())
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        <P::Fetch as FetchSystemParam>::init(&mut self.state, world, resources);

        if let Some(core) = self.core {
            let (tx, rx) = async_channel::unbounded();
            if let Some(f) = core.init(Accessor {
                channel: tx,
                _marker: Default::default(),
            }) {
                let executor = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
                executor.spawn(f).detach();
            }
            self.channel = rx;
        }
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

    use super::{Accessor, AsyncSystemWith};

    use crate::{Commands, IntoSystem, Query, Res, ResMut, Resources, Stage, SystemStage, World};

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

        let mut stage = SystemStage::serial();
        stage
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hello", &*string);
                })
                .system(),
            )
            .add_system(async_system.system())
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
