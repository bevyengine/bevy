use std::{any::TypeId, borrow::Cow};

use crate::{
    ArchetypeComponent, IntoSystem, Resources, System, SystemId, ThreadLocalExecution, TypeAccess,
    World,
};
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
    systems: Vec<Box<dyn System<In = (), Out = ()>>>,
    system_ids: HashSet<SystemId>,
    executor: Box<dyn SystemStageExecutor>,
    run_criteria: Option<Box<dyn System<In = (), Out = ShouldRun>>>,
    run_criteria_initialized: bool,
    changed_systems: Vec<usize>,
}

impl SystemStage {
    pub fn new(executor: Box<dyn SystemStageExecutor>) -> Self {
        SystemStage {
            executor,
            run_criteria: None,
            run_criteria_initialized: false,
            systems: Default::default(),
            system_ids: Default::default(),
            changed_systems: Default::default(),
        }
    }

    pub fn single<Params, S: System<In = (), Out = ()>, Into: IntoSystem<Params, S>>(
        system: Into,
    ) -> Self {
        Self::serial().with_system(system)
    }

    pub fn serial() -> Self {
        Self::new(Box::new(SerialSystemStageExecutor::default()))
    }

    pub fn parallel() -> Self {
        Self::new(Box::new(ParallelSystemStageExecutor::default()))
    }

    pub fn with_system<S, Params, IntoS>(mut self, system: IntoS) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed(Box::new(system.system()));
        self
    }

    pub fn with_run_criteria<S, Params, IntoS>(mut self, system: IntoS) -> Self
    where
        S: System<In = (), Out = ShouldRun>,
        IntoS: IntoSystem<Params, S>,
    {
        self.run_criteria = Some(Box::new(system.system()));
        self.run_criteria_initialized = false;
        self
    }

    pub fn add_system<S, Params, IntoS>(&mut self, system: IntoS) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed(Box::new(system.system()));
        self
    }

    pub fn add_system_boxed(&mut self, system: Box<dyn System<In = (), Out = ()>>) -> &mut Self {
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

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        let changed_systems = std::mem::take(&mut self.changed_systems);
        for system_index in changed_systems.iter() {
            self.systems[*system_index].initialize(world, resources);
        }
        self.executor
            .execute_stage(&mut self.systems, &changed_systems, world, resources);
    }
}

impl Stage for SystemStage {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        loop {
            let should_run = if let Some(ref mut run_criteria) = self.run_criteria {
                if !self.run_criteria_initialized {
                    run_criteria.initialize(world, resources);
                    self.run_criteria_initialized = true;
                }
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

pub trait IntoStage<Params> {
    type Stage: Stage;
    fn into_stage(self) -> Self::Stage;
}

impl<Params, S: System<In = (), Out = ()>, IntoS: IntoSystem<Params, S>> IntoStage<(Params, S)>
    for IntoS
{
    type Stage = SystemStage;

    fn into_stage(self) -> Self::Stage {
        SystemStage::single(self)
    }
}

impl<S: Stage> IntoStage<()> for S {
    type Stage = S;

    fn into_stage(self) -> Self::Stage {
        self
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
