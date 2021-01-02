use std::{any::TypeId, borrow::Cow};

use crate::{
    ArchetypeComponent, BoxedSystem, Resources, System, SystemId, ThreadLocalExecution, TypeAccess,
    World,
};
use bevy_utils::HashSet;
use downcast_rs::{impl_downcast, Downcast};

use super::{ParallelSystemStageExecutor, SerialSystemStageExecutor, SystemStageExecutor};

pub enum StageError {
    SystemAlreadyExists(SystemId),
}

pub trait Stage: Downcast + Send + Sync {
    /// Stages can perform setup here. Initialize should be called for every stage before calling [Stage::run]. Initialize will
    /// be called once per update, so internally this should avoid re-doing work where possible.
    fn initialize(&mut self, world: &mut World, resources: &mut Resources);

    /// Runs the stage. This happens once per update (after [Stage::initialize] is called).
    fn run(&mut self, world: &mut World, resources: &mut Resources);
}

impl_downcast!(Stage);

pub struct SystemStage {
    systems: Vec<BoxedSystem>,
    system_ids: HashSet<SystemId>,
    executor: Box<dyn SystemStageExecutor>,
    run_criteria: Option<BoxedSystem<(), ShouldRun>>,
    run_criteria_initialized: bool,
    uninitialized_systems: Vec<usize>,
    unexecuted_systems: Vec<usize>,
}

impl SystemStage {
    pub fn new(executor: Box<dyn SystemStageExecutor>) -> Self {
        SystemStage {
            executor,
            run_criteria: None,
            run_criteria_initialized: false,
            systems: Default::default(),
            system_ids: Default::default(),
            uninitialized_systems: Default::default(),
            unexecuted_systems: Default::default(),
        }
    }

    pub fn single<S: System<In = (), Out = ()>>(system: S) -> Self {
        Self::serial().with_system(system)
    }

    pub fn serial() -> Self {
        Self::new(Box::new(SerialSystemStageExecutor::default()))
    }

    pub fn parallel() -> Self {
        Self::new(Box::new(ParallelSystemStageExecutor::default()))
    }

    pub fn with_system<S: System<In = (), Out = ()>>(mut self, system: S) -> Self {
        self.add_system_boxed(Box::new(system));
        self
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.run_criteria = Some(Box::new(system));
        self.run_criteria_initialized = false;
        self
    }

    pub fn add_system<S: System<In = (), Out = ()>>(&mut self, system: S) -> &mut Self {
        self.add_system_boxed(Box::new(system));
        self
    }

    pub fn add_system_boxed(&mut self, system: BoxedSystem) -> &mut Self {
        if self.system_ids.contains(&system.id()) {
            panic!(
                "System with id {:?} ({}) already exists",
                system.id(),
                system.name()
            );
        }
        self.system_ids.insert(system.id());
        self.unexecuted_systems.push(self.systems.len());
        self.uninitialized_systems.push(self.systems.len());
        self.systems.push(system);
        self
    }

    pub fn get_executor<T: SystemStageExecutor>(&self) -> Option<&T> {
        self.executor.downcast_ref()
    }

    pub fn get_executor_mut<T: SystemStageExecutor>(&mut self) -> Option<&mut T> {
        self.executor.downcast_mut()
    }

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        let unexecuted_systems = std::mem::take(&mut self.unexecuted_systems);
        self.executor
            .execute_stage(&mut self.systems, &unexecuted_systems, world, resources);
    }
}

impl Stage for SystemStage {
    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        if let Some(ref mut run_criteria) = self.run_criteria {
            if !self.run_criteria_initialized {
                run_criteria.initialize(world, resources);
                self.run_criteria_initialized = true;
            }
        }

        let uninitialized_systems = std::mem::take(&mut self.uninitialized_systems);
        for system_index in uninitialized_systems.iter() {
            self.systems[*system_index].initialize(world, resources);
        }
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        loop {
            let should_run = if let Some(ref mut run_criteria) = self.run_criteria {
                let should_run = run_criteria.run((), world, resources);
                run_criteria.run_thread_local(world, resources);
                // don't run when no result is returned or false is returned
                should_run.unwrap_or(ShouldRun::No)
            } else {
                ShouldRun::Yes
            };

            match should_run {
                ShouldRun::No => return,
                ShouldRun::Yes => {
                    self.run_once(world, resources);
                    return;
                }
                ShouldRun::YesAndLoop => {
                    self.run_once(world, resources);
                }
            }
        }
    }
}

pub enum ShouldRun {
    /// No, the system should not run
    No,
    /// Yes, the system should run
    Yes,
    /// Yes, the system should run and after running, the criteria should be checked again.
    YesAndLoop,
}

impl<S: System<In = (), Out = ()>> From<S> for SystemStage {
    fn from(system: S) -> Self {
        SystemStage::single(system)
    }
}

pub struct RunOnce {
    ran: bool,
    system_id: SystemId,
    resource_access: TypeAccess<TypeId>,
    archetype_access: TypeAccess<ArchetypeComponent>,
}

impl Default for RunOnce {
    fn default() -> Self {
        Self {
            ran: false,
            system_id: SystemId::new(),
            resource_access: Default::default(),
            archetype_access: Default::default(),
        }
    }
}

impl System for RunOnce {
    type In = ();
    type Out = ShouldRun;

    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<RunOnce>())
    }

    fn id(&self) -> SystemId {
        self.system_id
    }

    fn update(&mut self, _world: &World) {}

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        ThreadLocalExecution::Immediate
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: Self::In,
        _world: &World,
        _resources: &Resources,
    ) -> Option<Self::Out> {
        Some(if self.ran {
            ShouldRun::No
        } else {
            self.ran = true;
            ShouldRun::Yes
        })
    }

    fn run_thread_local(&mut self, _world: &mut World, _resources: &mut Resources) {}

    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}
