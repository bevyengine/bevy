use std::{any::TypeId, borrow::Cow};

use crate::{
    ArchetypeComponent, InjectionPoint, Ordering, ParallelSystemDescriptor, Resources, RunCriteria,
    SequentialSystemDescriptor, ShouldRun, System, SystemDescriptor, SystemId, TypeAccess, World,
};
use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};

use super::{ParallelSystemStageExecutor, SerialSystemStageExecutor, SystemStageExecutor};

pub enum StageError {
    SystemAlreadyExists(SystemId),
}

pub trait Stage: Downcast + Send + Sync {
    /// Stages can perform setup here. Initialize should be called for every stage before
    /// calling [Stage::run]. Initialize will be called once per update, so internally this
    /// should avoid re-doing work where possible.
    fn initialize(&mut self, world: &mut World, resources: &mut Resources);

    /// Runs the stage. This happens once per update (after [Stage::initialize] is called).
    fn run(&mut self, world: &mut World, resources: &mut Resources);
}

impl_downcast!(Stage);

type Label = &'static str; // TODO

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemIndex {
    pub set: usize,
    pub system: usize,
}

pub struct SystemStage {
    run_criteria: RunCriteria,
    executor: Box<dyn SystemStageExecutor>,
    system_sets: Vec<SystemSet>,
    at_start: Vec<SystemIndex>,
    before_commands: Vec<SystemIndex>,
    at_end: Vec<SystemIndex>,
    parallel_dependencies: HashMap<SystemIndex, Vec<SystemIndex>>,
}

impl SystemStage {
    pub fn new(executor: Box<dyn SystemStageExecutor>) -> Self {
        SystemStage {
            run_criteria: Default::default(),
            executor,
            system_sets: vec![SystemSet::default()],
            at_start: Default::default(),
            before_commands: Default::default(),
            at_end: Default::default(),
            parallel_dependencies: Default::default(),
        }
    }

    pub fn single(system: impl Into<SystemDescriptor>) -> Self {
        Self::serial().with_system(system)
    }

    pub fn serial() -> Self {
        Self::new(Box::new(SerialSystemStageExecutor::default()))
    }

    pub fn parallel() -> Self {
        Self::new(Box::new(ParallelSystemStageExecutor::default()))
    }

    pub fn with_system(mut self, system: impl Into<SystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn with_system_set(mut self, system_set: SystemSet) -> Self {
        self.add_system_set(system_set);
        self
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.run_criteria.set(Box::new(system));
        self
    }

    pub fn add_system_set(&mut self, system_set: SystemSet) -> &mut Self {
        self.system_sets.push(system_set);
        self
    }

    pub fn add_system(&mut self, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.system_sets[0].add_system(system);
        self
    }

    pub fn get_executor<T: SystemStageExecutor>(&self) -> Option<&T> {
        self.executor.downcast_ref()
    }

    pub fn get_executor_mut<T: SystemStageExecutor>(&mut self) -> Option<&mut T> {
        self.executor.downcast_mut()
    }

    // TODO tests
    fn rebuild_orders_and_dependencies(&mut self) {
        // TODO consider doing this in two passes: collect labels, then resolve labels.
        self.at_start.clear();
        self.before_commands.clear();
        self.at_end.clear();
        self.parallel_dependencies.clear();
        let mut parallel_labels_map = HashMap::<Label, SystemIndex>::default();
        let mut at_start_labels_map = HashMap::<Label, usize>::default();
        let mut before_commands_labels_map = HashMap::<Label, usize>::default();
        let mut at_end_labels_map = HashMap::<Label, usize>::default();
        let insert_index = |index: SystemIndex,
                            descriptor: &SequentialSystemDescriptor,
                            order: &mut Vec<SystemIndex>,
                            map: &mut HashMap<Label, usize>| {
            let order_index = match descriptor.ordering {
                Ordering::None => {
                    order.push(index);
                    order.len() - 1
                }
                Ordering::Before(target) => {
                    let &target_index = map
                        .get(target)
                        .unwrap_or_else(|| todo!("some error message that makes sense"));
                    order.insert(target_index, index);
                    for value in map.values_mut().filter(|value| **value >= target_index) {
                        *value += 1;
                    }
                    target_index
                }
                Ordering::After(target) => {
                    let &target_index = map
                        .get(target)
                        .unwrap_or_else(|| todo!("some error message that makes sense"));
                    order.insert(target_index + 1, index);
                    for value in map.values_mut().filter(|value| **value > target_index) {
                        *value += 1;
                    }
                    target_index + 1
                }
            };
            if let Some(label) = descriptor.label {
                map.insert(label, order_index);
            }
        };
        for (set_index, system_set) in self.system_sets.iter_mut().enumerate() {
            for (system_index, descriptor) in system_set.sequential_systems.iter().enumerate() {
                let index = SystemIndex {
                    set: set_index,
                    system: system_index,
                };
                use InjectionPoint::*;
                match descriptor.injection_point {
                    AtStart => insert_index(
                        index,
                        descriptor,
                        &mut self.at_start,
                        &mut at_start_labels_map,
                    ),
                    BeforeCommands => insert_index(
                        index,
                        descriptor,
                        &mut self.before_commands,
                        &mut before_commands_labels_map,
                    ),
                    AtEnd => {
                        insert_index(index, descriptor, &mut self.at_end, &mut at_end_labels_map)
                    }
                }
            }
            for (system_index, descriptor) in system_set.parallel_systems.iter().enumerate() {
                // TODO dependency tree validation
                let index = SystemIndex {
                    set: set_index,
                    system: system_index,
                };
                if !descriptor.dependencies.is_empty() {
                    let dependencies = descriptor
                        .dependencies
                        .iter()
                        .map(|label| {
                            *parallel_labels_map
                                .get(label)
                                .unwrap_or_else(|| todo!("some error message that makes sense"))
                        })
                        .collect();
                    self.parallel_dependencies.insert(index, dependencies);
                }
                if let Some(label) = descriptor.label {
                    parallel_labels_map.insert(label, index);
                }
            }
        }
    }

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        if self
            .system_sets
            .iter()
            .any(|system_set| system_set.is_dirty)
        {
            self.rebuild_orders_and_dependencies();
        }
        self.executor.execute_stage(
            &mut self.system_sets,
            &self.at_start,
            &self.before_commands,
            &self.at_end,
            &self.parallel_dependencies,
            world,
            resources,
        );
        for system_set in &mut self.system_sets {
            system_set.is_dirty = false;
        }
    }
}

impl Stage for SystemStage {
    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        for set in &mut self.system_sets {
            set.initialize(world, resources);
        }
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        loop {
            match self.run_criteria.should_run(world, resources) {
                ShouldRun::No => return,
                ShouldRun::Yes => {
                    self.run_once(world, resources);
                    return;
                }
                ShouldRun::YesAndLoop => {
                    self.run_once(world, resources);
                }
                ShouldRun::NoAndLoop => {
                    panic!("`NoAndLoop` run criteria would loop infinitely in this situation.")
                }
            }
        }
    }
}

#[derive(Default)]
pub struct SystemSet {
    run_criteria: RunCriteria,
    is_dirty: bool,
    parallel_systems: Vec<ParallelSystemDescriptor>,
    sequential_systems: Vec<SequentialSystemDescriptor>,
    uninitialized_parallel: Vec<usize>,
    uninitialized_sequential: Vec<usize>,
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        for index in self.uninitialized_sequential.drain(..) {
            self.sequential_systems[index]
                .system
                .initialize(world, resources);
        }
        for index in self.uninitialized_parallel.drain(..) {
            self.parallel_systems[index]
                .system_mut()
                .initialize(world, resources);
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub(crate) fn run_criteria_mut(&mut self) -> &mut RunCriteria {
        &mut self.run_criteria
    }

    pub(crate) fn exclusive_system_mut(
        &mut self,
        index: usize,
    ) -> &mut dyn System<In = (), Out = ()> {
        &mut *self.sequential_systems[index].system
    }

    pub(crate) fn parallel_system_mut(
        &mut self,
        index: usize,
    ) -> &mut dyn System<In = (), Out = ()> {
        self.parallel_systems[index].system_mut()
    }

    /// # Safety
    /// Ensure no other borrows of this system exist along with this one.
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn parallel_system_mut_unsafe(
        &self,
        index: usize,
    ) -> &mut dyn System<In = (), Out = ()> {
        self.parallel_systems[index].system_mut_unsafe()
    }

    pub(crate) fn parallel_systems_len(&self) -> usize {
        self.parallel_systems.len()
    }

    pub(crate) fn parallel_systems(&self) -> impl Iterator<Item = &dyn System<In = (), Out = ()>> {
        self.parallel_systems
            .iter()
            .map(|descriptor| descriptor.system())
    }

    pub(crate) fn parallel_systems_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut dyn System<In = (), Out = ()>> {
        self.parallel_systems
            .iter_mut()
            .map(|descriptor| descriptor.system_mut())
    }

    pub fn with_system(mut self, system: impl Into<SystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn add_system(&mut self, system: impl Into<SystemDescriptor>) -> &mut Self {
        match system.into() {
            SystemDescriptor::Parallel(descriptor) => {
                self.uninitialized_parallel
                    .push(self.parallel_systems.len());
                self.parallel_systems.push(descriptor);
            }
            SystemDescriptor::Sequential(descriptor) => {
                self.uninitialized_sequential
                    .push(self.sequential_systems.len());
                self.sequential_systems.push(descriptor);
            }
        }
        self.is_dirty = true;
        self
    }
}

impl<S: Into<SystemDescriptor>> From<S> for SystemStage {
    fn from(system: S) -> Self {
        SystemStage::single(system)
    }
}

pub struct RunOnce {
    ran: bool,
    system_id: SystemId,
    archetype_component_access: TypeAccess<ArchetypeComponent>,
    resource_access: TypeAccess<TypeId>,
}

impl Default for RunOnce {
    fn default() -> Self {
        Self {
            ran: false,
            system_id: SystemId::new(),
            archetype_component_access: Default::default(),
            resource_access: Default::default(),
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

    fn update_access(&mut self, _world: &World) {}

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn is_thread_local(&self) -> bool {
        false
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

    fn run_exclusive(&mut self, _world: &mut World, _resources: &mut Resources) {}

    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}
