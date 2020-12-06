use crate::{IntoSystem, Resources, System, SystemId, World};
use bevy_utils::HashSet;
use downcast_rs::{impl_downcast, Downcast};

use super::{ParallelSystemStageExecutor, SerialSystemStageExecutor, SystemStageExecutor};

pub enum StageError {
    SystemAlreadyExists(SystemId),
}

pub trait Stage: Downcast + Send + Sync {
    fn run(&mut self, world: &mut World, resources: &mut Resources);
}

impl_downcast!(Stage);

pub struct SystemStage {
    systems: Vec<Box<dyn System<Input = (), Output = ()>>>,
    system_ids: HashSet<SystemId>,
    executor: Box<dyn SystemStageExecutor>,
    changed_systems: Vec<usize>,
}

impl SystemStage {
    pub fn new(executor: Box<dyn SystemStageExecutor>) -> Self {
        SystemStage {
            executor,
            systems: Default::default(),
            system_ids: Default::default(),
            changed_systems: Default::default(),
        }
    }

    pub fn single<Params, S: System<Input = (), Output = ()>, Into: IntoSystem<Params, S>>(
        system: Into,
    ) -> Self {
        Self::serial().system(system)
    }

    pub fn serial() -> Self {
        Self::new(Box::new(SerialSystemStageExecutor::default()))
    }

    pub fn parallel() -> Self {
        Self::new(Box::new(ParallelSystemStageExecutor::default()))
    }

    pub fn system<S, Params, IntoS>(mut self, system: IntoS) -> Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed(Box::new(system.system()));
        self
    }

    pub fn add_system<S, Params, IntoS>(&mut self, system: IntoS) -> &mut Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed(Box::new(system.system()));
        self
    }

    pub fn add_system_boxed(
        &mut self,
        system: Box<dyn System<Input = (), Output = ()>>,
    ) -> &mut Self {
        if self.system_ids.contains(&system.id()) {
            panic!(
                "System with id {:?} ({}) already exists",
                system.id(),
                system.name()
            );
        }
        self.system_ids.insert(system.id());
        self.changed_systems.push(self.systems.len());
        self.systems.push(system);
        self
    }

    pub fn get_executor<T: SystemStageExecutor>(&self) -> Option<&T> {
        self.executor.downcast_ref()
    }

    pub fn get_executor_mut<T: SystemStageExecutor>(&mut self) -> Option<&mut T> {
        self.executor.downcast_mut()
    }
}

impl Stage for SystemStage {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        let changed_systems = std::mem::take(&mut self.changed_systems);
        for system_index in changed_systems.iter() {
            self.systems[*system_index].initialize(world, resources);
        }
        self.executor
            .execute_stage(&mut self.systems, &changed_systems, world, resources);
    }
}

struct EmptyStage;

impl Stage for EmptyStage {
    fn run(&mut self, _world: &mut World, _resources: &mut Resources) {}
}
