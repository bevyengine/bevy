use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};

use super::{ParallelSystemStageExecutor, SerialSystemStageExecutor, SystemStageExecutor};
use crate::{
    topological_sorting, ExclusiveSystem, ExclusiveSystemDescriptor, InsertionPoint,
    ParallelSystemDescriptor, Resources, RunCriteria, ShouldRun, SortingResult, System, SystemId,
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

impl std::fmt::Debug for SystemIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SystemIndex({},{})", self.set, self.system)
    }
}

pub struct SystemStage {
    /// Determines if this stage should run.
    run_criteria: RunCriteria,
    /// Instance of a scheduling algorithm for running the systems.
    executor: Box<dyn SystemStageExecutor>,
    /// Groups of systems; each set has its own run criterion.
    system_sets: Vec<SystemSet>,
    /// Topologically sorted exclusive systems that want to be ran at the start of the stage.
    at_start: Vec<SystemIndex>,
    /// Topologically sorted exclusive systems that want to be ran after parrallel systems and
    /// before the application of their command buffers.
    before_commands: Vec<SystemIndex>,
    /// Topologically sorted exclusive systems that want to be ran at the end of the stage.
    at_end: Vec<SystemIndex>,
    /// Resolved graph of parallel systems and their dependencies. Contains all parallel systems.
    parallel_dependencies: HashMap<SystemIndex, Vec<SystemIndex>>,
    /// Topologically sorted parallel systems.
    parallel_sorted: Vec<SystemIndex>,
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
            parallel_sorted: Default::default(),
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

    pub fn get_executor<T: SystemStageExecutor>(&self) -> Option<&T> {
        self.executor.downcast_ref()
    }

    pub fn get_executor_mut<T: SystemStageExecutor>(&mut self) -> Option<&mut T> {
        self.executor.downcast_mut()
    }

    pub fn set_executor(&mut self, executor: Box<dyn SystemStageExecutor>) {
        self.executor = executor;
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

    // TODO tests
    fn rebuild_orders_and_dependencies(&mut self) {
        // Collect labels.
        let mut parallel_labels_map = HashMap::<Label, SystemIndex>::default();
        let mut exclusive_labels_map = HashMap::<Label, SystemIndex>::default();
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
                    exclusive_labels_map.insert(
                        label,
                        SystemIndex {
                            set: set_index,
                            system: system_index,
                        },
                    );
                }
            }
        }

        // Build dependency graphs.
        self.parallel_dependencies.clear();
        let mut at_start_graph = HashMap::default();
        let mut before_commands_graph = HashMap::default();
        let mut at_end_graph = HashMap::default();
        for (set_index, system_set) in self.system_sets.iter().enumerate() {
            for (system_index, descriptor) in system_set.parallel_systems.iter().enumerate() {
                insert_into_graph(
                    SystemIndex {
                        set: set_index,
                        system: system_index,
                    },
                    &descriptor.after,
                    &descriptor.before,
                    &mut self.parallel_dependencies,
                    &parallel_labels_map,
                );
            }
            for (system_index, descriptor) in system_set.exclusive_systems.iter().enumerate() {
                let tree = match descriptor.insertion_point {
                    InsertionPoint::AtStart => &mut at_start_graph,
                    InsertionPoint::BeforeCommands => &mut before_commands_graph,
                    InsertionPoint::AtEnd => &mut at_end_graph,
                };
                insert_into_graph(
                    SystemIndex {
                        set: set_index,
                        system: system_index,
                    },
                    &descriptor.after,
                    &descriptor.before,
                    tree,
                    &exclusive_labels_map,
                )
            }
        }

        // Generate topological order for parallel systems.
        self.parallel_sorted = match topological_sorting(&self.parallel_dependencies) {
            SortingResult::Sorted(sorted) => sorted,
            // TODO better error
            SortingResult::FoundCycle(cycle) => panic!(
                "found cycle: {:?}",
                cycle
                    .iter()
                    .map(
                        |index| self.system_sets[index.set].parallel_systems[index.system]
                            .label
                            .unwrap_or("<unlabeled>")
                    )
                    .collect::<Vec<_>>()
            ),
        };

        // Generate topological orders for exclusive systems.
        let system_sets = &self.system_sets;
        let try_sort = |graph: &HashMap<SystemIndex, _>| {
            match topological_sorting(graph) {
                SortingResult::Sorted(sorted) => sorted,
                // TODO better error
                SortingResult::FoundCycle(cycle) => panic!(
                    "found cycle: {:?}",
                    cycle
                        .iter()
                        .map(
                            |index| system_sets[index.set].exclusive_systems[index.system]
                                .label
                                .unwrap_or("<unlabeled>")
                        )
                        .collect::<Vec<_>>()
                ),
            }
        };
        self.at_start = try_sort(&at_start_graph);
        self.before_commands = try_sort(&before_commands_graph);
        self.at_end = try_sort(&at_end_graph);
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
            &self.parallel_sorted,
            world,
            resources,
        );
        for system_set in &mut self.system_sets {
            system_set.is_dirty = false;
        }
    }
}

fn insert_into_graph(
    system_index: SystemIndex,
    dependencies: &[Label],
    dependants: &[Label],
    graph: &mut HashMap<SystemIndex, Vec<SystemIndex>>,
    labels: &HashMap<Label, SystemIndex>,
) {
    let resolve_label = |label| {
        // TODO better error message
        labels
            .get(label)
            .unwrap_or_else(|| panic!("no such system"))
    };
    {
        let children = graph.entry(system_index).or_insert_with(Vec::new);
        for dependency in dependencies.iter().map(resolve_label) {
            if !children.contains(dependency) {
                children.push(*dependency);
            }
        }
    }
    for dependant in dependants.iter().map(resolve_label) {
        let children = graph.entry(*dependant).or_insert_with(Vec::new);
        if !children.contains(&system_index) {
            children.push(system_index);
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

#[cfg(test)]
mod tests {
    use crate::{prelude::*, SerialSystemStageExecutor};
    use bevy_tasks::{ComputeTaskPool, TaskPoolBuilder};
    use std::thread::{self, ThreadId};

    fn make_exclusive(tag: usize) -> impl FnMut(&mut Resources) {
        move |resources| resources.get_mut::<Vec<usize>>().unwrap().push(tag)
    }

    // This is silly. https://github.com/bevyengine/bevy/issues/1029
    macro_rules! make_parallel {
        ($tag:expr) => {{
            fn parallel(mut resource: ResMut<Vec<usize>>) {
                resource.push($tag)
            }
            parallel
        }};
    }

    #[test]
    fn insertion_points() {
        let mut world = World::new();
        let mut resources = Resources::default();

        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(0).system().at_start())
            .with_system(make_parallel!(1).system())
            .with_exclusive_system(make_exclusive(2).system().before_commands())
            .with_exclusive_system(make_exclusive(3).system().at_end());
        stage.run(&mut world, &mut resources);
        assert_eq!(*resources.get::<Vec<usize>>().unwrap(), vec![0, 1, 2, 3]);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );

        resources.get_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(2).system().before_commands())
            .with_exclusive_system(make_exclusive(3).system().at_end())
            .with_system(make_parallel!(1).system())
            .with_exclusive_system(make_exclusive(0).system().at_start());
        stage.run(&mut world, &mut resources);
        assert_eq!(*resources.get::<Vec<usize>>().unwrap(), vec![0, 1, 2, 3]);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );
    }

    #[test]
    fn exclusive_after() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(1).system().label("1").after("0"))
            .with_exclusive_system(make_exclusive(2).system().after("1"))
            .with_exclusive_system(make_exclusive(0).system().label("0"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn exclusive_before() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(1).system().label("1").before("2"))
            .with_exclusive_system(make_exclusive(2).system().label("2"))
            .with_exclusive_system(make_exclusive(0).system().before("1"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn exclusive_mixed() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(2).system().label("2"))
            .with_exclusive_system(make_exclusive(1).system().label("1").after("0").before("2"))
            .with_exclusive_system(make_exclusive(0).system().label("0"))
            .with_exclusive_system(make_exclusive(4).system().label("4"))
            .with_exclusive_system(make_exclusive(3).system().label("3").after("2").before("4"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_2() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(0).system().label("0").after("1"))
            .with_exclusive_system(make_exclusive(1).system().label("1").after("0"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_3() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(0).system().label("0"))
            .with_exclusive_system(make_exclusive(1).system().after("0").before("2"))
            .with_exclusive_system(make_exclusive(2).system().label("2").before("0"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    fn parallel_after() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(1).system().after("0").label("1"))
            .with_system(make_parallel!(2).system().after("1"))
            .with_system(make_parallel!(0).system().label("0"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn parallel_before() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(1).system().label("1").before("2"))
            .with_system(make_parallel!(2).system().label("2"))
            .with_system(make_parallel!(0).system().before("1"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn parallel_mixed() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(2).system().label("2"))
            .with_system(make_parallel!(1).system().label("1").after("0").before("2"))
            .with_system(make_parallel!(0).system().label("0"))
            .with_system(make_parallel!(4).system().label("4"))
            .with_system(make_parallel!(3).system().label("3").after("2").before("4"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_2() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(0).system().label("0").after("1"))
            .with_system(make_parallel!(1).system().label("1").after("0"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_3() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(0).system().label("0"))
            .with_system(make_parallel!(1).system().after("0").before("2"))
            .with_system(make_parallel!(2).system().label("2").before("0"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    fn thread_local_resource_system() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(ComputeTaskPool(
            TaskPoolBuilder::new().num_threads(4).build(),
        ));
        resources.insert_thread_local(thread::current().id());

        fn wants_thread_local(thread_id: ThreadLocal<ThreadId>) {
            assert_eq!(thread::current().id(), *thread_id);
        }

        let mut stage = SystemStage::parallel()
            .with_system(wants_thread_local.system())
            .with_system(wants_thread_local.system())
            .with_system(wants_thread_local.system())
            .with_system(wants_thread_local.system());
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SerialSystemStageExecutor::default()));
        stage.run(&mut world, &mut resources);
    }
}
