use bevy_utils::{HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use std::{any::TypeId, borrow::Cow};

use super::{ParallelSystemStageExecutor, SerialSystemStageExecutor, SystemStageExecutor};
use crate::{
    ArchetypeComponent, ExclusiveSystem, ExclusiveSystemDescriptor, InjectionPoint, Ordering,
    ParallelSystemDescriptor, Resources, RunCriteria, ShouldRun, System, SystemId, TypeAccess,
    World,
};

pub enum StageError {
    SystemAlreadyExists(SystemId),
}

pub trait Stage: Downcast + Send + Sync {
    /// Runs the stage; this happens once per update.
    /// Implementors must initialize all of their state and systems before running the first time.
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

    pub fn single(system: impl Into<ExclusiveSystemDescriptor>) -> Self {
        Self::serial().with_exclusive_system(system)
    }

    pub fn serial() -> Self {
        Self::new(Box::new(SerialSystemStageExecutor::default()))
    }

    pub fn parallel() -> Self {
        Self::new(Box::new(ParallelSystemStageExecutor::default()))
    }

    pub fn with_system(mut self, system: impl Into<ParallelSystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn with_exclusive_system(mut self, system: impl Into<ExclusiveSystemDescriptor>) -> Self {
        self.add_exclusive_system(system);
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

    pub fn add_system(&mut self, system: impl Into<ParallelSystemDescriptor>) -> &mut Self {
        self.system_sets[0].add_system(system);
        self
    }

    pub fn add_exclusive_system(
        &mut self,
        system: impl Into<ExclusiveSystemDescriptor>,
    ) -> &mut Self {
        self.system_sets[0].add_exclusive_system(system);
        self
    }

    pub fn get_executor<T: SystemStageExecutor>(&self) -> Option<&T> {
        self.executor.downcast_ref()
    }

    pub fn get_executor_mut<T: SystemStageExecutor>(&mut self) -> Option<&mut T> {
        self.executor.downcast_mut()
    }

    /// Determines if the parallel systems dependency graph has a cycle using depth first search.
    fn has_a_dependency_cycle(&self) -> bool {
        fn is_part_of_a_cycle(
            index: &SystemIndex,
            visited: &mut HashSet<SystemIndex>,
            current: &mut HashSet<SystemIndex>,
            graph: &HashMap<SystemIndex, Vec<SystemIndex>>,
        ) -> bool {
            if current.contains(index) {
                return true;
            } else if visited.contains(index) {
                return false;
            }
            visited.insert(*index);
            current.insert(*index);
            for dependency in graph.get(index).unwrap() {
                if is_part_of_a_cycle(dependency, visited, current, graph) {
                    return true;
                }
            }
            current.remove(index);
            false
        }
        let mut visited =
            HashSet::with_capacity_and_hasher(self.parallel_dependencies.len(), Default::default());
        let mut current =
            HashSet::with_capacity_and_hasher(self.parallel_dependencies.len(), Default::default());
        for system_index in self.parallel_dependencies.keys() {
            if is_part_of_a_cycle(
                system_index,
                &mut visited,
                &mut current,
                &self.parallel_dependencies,
            ) {
                return true;
            }
        }
        false
    }

    // TODO tests
    fn rebuild_orders_and_dependencies(&mut self) {
        self.parallel_dependencies.clear();
        self.at_start.clear();
        self.before_commands.clear();
        self.at_end.clear();
        let mut parallel_labels_map = HashMap::<Label, SystemIndex>::default();
        let mut at_start_labels_map = HashMap::<Label, SystemIndex>::default();
        let mut before_commands_labels_map = HashMap::<Label, SystemIndex>::default();
        let mut at_end_labels_map = HashMap::<Label, SystemIndex>::default();
        // Collect labels.
        for (set_index, system_set) in self.system_sets.iter().enumerate() {
            for (system_index, descriptor) in system_set.parallel_systems.iter().enumerate() {
                if let Some(label) = descriptor.label {
                    parallel_labels_map.insert(
                        label,
                        SystemIndex {
                            set: set_index,
                            system: system_index,
                        },
                    );
                }
            }
            for (system_index, descriptor) in system_set.exclusive_systems.iter().enumerate() {
                if let Some(label) = descriptor.label {
                    let index = SystemIndex {
                        set: set_index,
                        system: system_index,
                    };
                    use InjectionPoint::*;
                    match descriptor.injection_point {
                        AtStart => at_start_labels_map.insert(label, index),
                        BeforeCommands => before_commands_labels_map.insert(label, index),
                        AtEnd => at_end_labels_map.insert(label, index),
                    };
                }
            }
        }
        // Populate parallel dependency tree and sequential orders.
        let mut new_dependencies = Vec::new(); // Scratch space.
        for (set_index, system_set) in self.system_sets.iter().enumerate() {
            for (system_index, descriptor) in system_set.parallel_systems.iter().enumerate() {
                if !descriptor.dependencies.is_empty() {
                    let index = SystemIndex {
                        set: set_index,
                        system: system_index,
                    };
                    let dependencies = self
                        .parallel_dependencies
                        .entry(index)
                        .or_insert_with(Vec::new);
                    new_dependencies.extend(
                        descriptor
                            .dependencies
                            .iter()
                            .map(|label| {
                                // TODO better error message
                                *parallel_labels_map
                                    .get(label)
                                    .unwrap_or_else(|| panic!("no such system"))
                            })
                            .filter(|dependency| !dependencies.contains(dependency)),
                    );
                    dependencies.extend(new_dependencies.drain(..));
                    for dependant in descriptor.dependants.iter().map(|label| {
                        // TODO better error message
                        *parallel_labels_map
                            .get(label)
                            .unwrap_or_else(|| panic!("no such system"))
                    }) {
                        let dependencies = self
                            .parallel_dependencies
                            .entry(dependant)
                            .or_insert_with(Vec::new);
                        if !dependencies.contains(&index) {
                            dependencies.push(index);
                        }
                    }
                }
            }
            for (system_index, descriptor) in system_set.exclusive_systems.iter().enumerate() {
                let index = SystemIndex {
                    set: set_index,
                    system: system_index,
                };
                use InjectionPoint::*;
                match descriptor.injection_point {
                    AtStart => insert_sequential_system(
                        index,
                        descriptor.ordering,
                        &mut self.at_start,
                        &at_start_labels_map,
                    ),
                    BeforeCommands => insert_sequential_system(
                        index,
                        descriptor.ordering,
                        &mut self.before_commands,
                        &before_commands_labels_map,
                    ),
                    AtEnd => insert_sequential_system(
                        index,
                        descriptor.ordering,
                        &mut self.at_end,
                        &at_end_labels_map,
                    ),
                }
            }
        }
        if self.has_a_dependency_cycle() {
            panic!("the graph cycles"); // TODO better error message.
        }
    }

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        let mut is_dirty = false;
        for system_set in self
            .system_sets
            .iter_mut()
            .filter(|system_set| system_set.is_dirty)
        {
            is_dirty = true;
            system_set.initialize(world, resources);
        }
        if is_dirty {
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

fn find_target_index(
    target: Label,
    order: &[SystemIndex],
    map: &HashMap<Label, SystemIndex>,
) -> Option<usize> {
    // TODO better error message
    let target = map.get(target).unwrap_or_else(|| panic!("no such system"));
    order
        .iter()
        .enumerate()
        .find_map(|(order_index, system_index)| {
            if system_index == target {
                Some(order_index)
            } else {
                None
            }
        })
}

fn insert_sequential_system(
    system_index: SystemIndex,
    ordering: Ordering,
    order: &mut Vec<SystemIndex>,
    map: &HashMap<Label, SystemIndex>,
) {
    match ordering {
        Ordering::None => order.push(system_index),
        Ordering::Before(target) => {
            if let Some(target) = find_target_index(target, order, map) {
                order.insert(target, system_index);
            }
        }
        Ordering::After(target) => {
            if let Some(target) = find_target_index(target, order, map) {
                order.insert(target + 1, system_index);
            }
        }
    }
}

impl Stage for SystemStage {
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
    exclusive_systems: Vec<ExclusiveSystemDescriptor>,
    uninitialized_parallel: Vec<usize>,
    uninitialized_exclusive: Vec<usize>,
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        for index in self.uninitialized_exclusive.drain(..) {
            self.exclusive_systems[index]
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

    pub(crate) fn exclusive_system_mut(&mut self, index: usize) -> &mut impl ExclusiveSystem {
        &mut self.exclusive_systems[index].system
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

    pub fn with_system(mut self, system: impl Into<ParallelSystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn with_exclusive_system(mut self, system: impl Into<ExclusiveSystemDescriptor>) -> Self {
        self.add_exclusive_system(system);
        self
    }

    pub fn add_system(&mut self, system: impl Into<ParallelSystemDescriptor>) -> &mut Self {
        self.uninitialized_parallel
            .push(self.parallel_systems.len());
        self.parallel_systems.push(system.into());

        self.is_dirty = true;
        self
    }

    pub fn add_exclusive_system(
        &mut self,
        system: impl Into<ExclusiveSystemDescriptor>,
    ) -> &mut Self {
        self.uninitialized_exclusive
            .push(self.exclusive_systems.len());
        self.exclusive_systems.push(system.into());
        self.is_dirty = true;
        self
    }
}

impl<S: Into<ExclusiveSystemDescriptor>> From<S> for SystemStage {
    fn from(descriptor: S) -> Self {
        SystemStage::single(descriptor.into())
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

    fn apply_buffers(&mut self, _world: &mut World, _resources: &mut Resources) {}

    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}
