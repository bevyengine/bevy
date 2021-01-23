use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};

use super::{ParallelExecutor, ParallelSystemExecutor, SingleThreadedExecutor};
use crate::{
    topological_sorting, ExclusiveSystem, ExclusiveSystemDescriptor, InsertionPoint,
    ParallelSystemDescriptor, Resources, RunCriteria,
    ShouldRun::{self, *},
    SortingResult, System, SystemId, SystemSet, World,
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
    executor: Box<dyn ParallelSystemExecutor>,
    /// Groups of systems; each set has its own run criterion.
    system_sets: Vec<SystemSet>,
    /// Cached results of system sets' run criteria evaluation.
    system_set_should_run: Vec<ShouldRun>,
    /// Topologically sorted exclusive systems that want to be ran at the start of the stage.
    exclusive_at_start: Vec<SystemIndex>,
    /// Topologically sorted exclusive systems that want to be ran after parallel systems but
    /// before the application of their command buffers.
    exclusive_before_commands: Vec<SystemIndex>,
    /// Topologically sorted exclusive systems that want to be ran at the end of the stage.
    exclusive_at_end: Vec<SystemIndex>,
    /// Resolved graph of parallel systems and their dependencies. Contains all parallel systems.
    parallel_dependency_graph: HashMap<SystemIndex, Vec<SystemIndex>>,
    /// Topologically sorted parallel systems.
    parallel_topological_order: Vec<SystemIndex>,
}

impl SystemStage {
    pub fn new(executor: Box<dyn ParallelSystemExecutor>) -> Self {
        SystemStage {
            run_criteria: Default::default(),
            executor,
            system_sets: vec![SystemSet::default()],
            system_set_should_run: Default::default(),
            exclusive_at_start: Default::default(),
            exclusive_before_commands: Default::default(),
            exclusive_at_end: Default::default(),
            parallel_dependency_graph: Default::default(),
            parallel_topological_order: Default::default(),
        }
    }

    pub fn single(system: impl Into<ExclusiveSystemDescriptor>) -> Self {
        Self::single_threaded().with_exclusive_system(system)
    }

    pub fn single_threaded() -> Self {
        Self::new(Box::new(SingleThreadedExecutor::default()))
    }

    pub fn parallel() -> Self {
        Self::new(Box::new(ParallelExecutor::default()))
    }

    pub fn get_executor<T: ParallelSystemExecutor>(&self) -> Option<&T> {
        self.executor.downcast_ref()
    }

    pub fn get_executor_mut<T: ParallelSystemExecutor>(&mut self) -> Option<&mut T> {
        self.executor.downcast_mut()
    }

    pub fn set_executor(&mut self, executor: Box<dyn ParallelSystemExecutor>) {
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
            for (system_index, descriptor) in system_set.parallel_descriptors.iter().enumerate() {
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
            for (system_index, descriptor) in system_set.exclusive_descriptors.iter().enumerate() {
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
        self.parallel_dependency_graph.clear();
        let mut at_start_graph = HashMap::default();
        let mut before_commands_graph = HashMap::default();
        let mut at_end_graph = HashMap::default();
        for (set_index, system_set) in self.system_sets.iter().enumerate() {
            for (system_index, descriptor) in system_set.parallel_descriptors.iter().enumerate() {
                insert_into_graph(
                    SystemIndex {
                        set: set_index,
                        system: system_index,
                    },
                    &descriptor.after,
                    &descriptor.before,
                    &mut self.parallel_dependency_graph,
                    &parallel_labels_map,
                );
            }
            for (system_index, descriptor) in system_set.exclusive_descriptors.iter().enumerate() {
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
        self.parallel_topological_order = match topological_sorting(&self.parallel_dependency_graph)
        {
            SortingResult::Sorted(sorted) => sorted,
            // TODO better error
            SortingResult::FoundCycle(cycle) => panic!(
                "found cycle: {:?}",
                cycle
                    .iter()
                    .map(|index| {
                        let system =
                            &self.system_sets[index.set].parallel_descriptors[index.system];
                        system
                            .label
                            .map(|label| label.into())
                            .unwrap_or_else(|| system.system().name())
                    })
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
                        .map(|index| {
                            let system =
                                &system_sets[index.set].exclusive_descriptors[index.system];
                            system
                                .label
                                .map(|label| label.into())
                                .unwrap_or_else(|| system.system.name())
                        })
                        .collect::<Vec<_>>()
                ),
            }
        };
        self.exclusive_at_start = try_sort(&at_start_graph);
        self.exclusive_before_commands = try_sort(&before_commands_graph);
        self.exclusive_at_end = try_sort(&at_end_graph);
    }

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        let mut is_dirty = false;
        let mut has_any_work = false;
        let mut has_doable_work = false;
        self.system_set_should_run.clear();

        // Evaluate sets' run criteria, initialize sets as needed, detect if any sets were changed.
        for system_set in &mut self.system_sets {
            if system_set.is_dirty() {
                is_dirty = true;
                system_set.initialize(world, resources);
            }
            let result = system_set.should_run(world, resources);
            match result {
                Yes | YesAndLoop => {
                    has_doable_work = true;
                    has_any_work = true;
                }
                NoAndLoop => has_any_work = true,
                No => (),
            }
            self.system_set_should_run.push(result);
        }
        // TODO a real error message
        assert!(!has_any_work || has_doable_work);
        if !has_doable_work {
            return;
        }

        if is_dirty {
            self.rebuild_orders_and_dependencies();
        }

        while has_doable_work {
            // Run systems that want to be at the start of stage.
            for index in &self.exclusive_at_start {
                if let Yes | YesAndLoop = self.system_set_should_run[index.set] {
                    self.system_sets[index.set]
                        .exclusive_system_mut(index.system)
                        .run(world, resources);
                }
            }

            // Run parallel systems using the executor.
            self.executor.run_systems(
                &mut self.system_sets,
                &self.system_set_should_run,
                &self.parallel_dependency_graph,
                &self.parallel_topological_order,
                world,
                resources,
            );

            // Run systems that want to be between parallel systems and their command buffers.
            for index in &self.exclusive_before_commands {
                if let Yes | YesAndLoop = self.system_set_should_run[index.set] {
                    self.system_sets[index.set]
                        .exclusive_system_mut(index.system)
                        .run(world, resources);
                }
            }

            // Apply parallel systems' buffers.
            for index in &self.parallel_topological_order {
                if let Yes | YesAndLoop = self.system_set_should_run[index.set] {
                    self.system_sets[index.set]
                        .parallel_system_mut(index.system)
                        .apply_buffers(world, resources);
                }
            }

            // Run systems that want to be at the end of stage.
            for index in &self.exclusive_at_end {
                if let Yes | YesAndLoop = self.system_set_should_run[index.set] {
                    self.system_sets[index.set]
                        .exclusive_system_mut(index.system)
                        .run(world, resources);
                }
            }

            // Reevaluate system sets' run criteria.
            has_any_work = false;
            has_doable_work = false;
            for (index, result) in self.system_set_should_run.iter_mut().enumerate() {
                match result {
                    No => (),
                    Yes => *result = No,
                    YesAndLoop | NoAndLoop => {
                        let new_result = self.system_sets[index].should_run(world, resources);
                        match new_result {
                            Yes | YesAndLoop => {
                                has_doable_work = true;
                                has_any_work = true;
                            }
                            NoAndLoop => has_any_work = true,
                            No => (),
                        }
                        *result = new_result;
                    }
                }
            }
            // TODO a real error message
            assert!(!has_any_work || has_doable_work);
        }
        for system_set in &mut self.system_sets {
            system_set.reset_dirty();
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
                No => return,
                Yes => {
                    self.run_once(world, resources);
                    return;
                }
                YesAndLoop => {
                    self.run_once(world, resources);
                }
                NoAndLoop => {
                    panic!("`NoAndLoop` run criteria would loop infinitely in this situation.")
                }
            }
        }
    }
}

// TODO does this even work
impl<S: Into<ExclusiveSystemDescriptor>> From<S> for SystemStage {
    fn from(descriptor: S) -> Self {
        SystemStage::single(descriptor.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, SingleThreadedExecutor};
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

    fn resettable_run_once(mut has_ran: ResMut<bool>) -> ShouldRun {
        if !*has_ran {
            *has_ran = true;
            return ShouldRun::Yes;
        }
        ShouldRun::No
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
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
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
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
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
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
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
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
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
            .with_exclusive_system(make_exclusive(1).system().after("0").before("2"))
            .with_exclusive_system(make_exclusive(0).system().label("0"))
            .with_exclusive_system(make_exclusive(4).system().label("4"))
            .with_exclusive_system(make_exclusive(3).system().after("2").before("4"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_mixed_across_sets() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(2).system().label("2"))
            .with_system_set(
                SystemSet::new()
                    .with_exclusive_system(make_exclusive(0).system().label("0"))
                    .with_exclusive_system(make_exclusive(4).system().label("4"))
                    .with_exclusive_system(make_exclusive(3).system().after("2").before("4")),
            )
            .with_exclusive_system(make_exclusive(1).system().after("0").before("2"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_run_criteria() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        resources.insert(false);
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(0).system().before("1"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(resettable_run_once.system())
                    .with_exclusive_system(make_exclusive(1).system().label("1")),
            )
            .with_exclusive_system(make_exclusive(2).system().after("1"));
        stage.run(&mut world, &mut resources);
        stage.run(&mut world, &mut resources);
        *resources.get_mut::<bool>().unwrap() = false;
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 2, 0, 1, 2, 0, 2]
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
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
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
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
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
            .with_system(make_parallel!(1).system().after("0").before("2"))
            .with_system(make_parallel!(0).system().label("0"))
            .with_system(make_parallel!(4).system().label("4"))
            .with_system(make_parallel!(3).system().after("2").before("4"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_mixed_across_sets() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(2).system().label("2"))
            .with_system_set(
                SystemSet::new()
                    .with_system(make_parallel!(0).system().label("0"))
                    .with_system(make_parallel!(4).system().label("4"))
                    .with_system(make_parallel!(3).system().after("2").before("4")),
            )
            .with_system(make_parallel!(1).system().after("0").before("2"));
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_run_criteria() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        resources.insert(false);
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(0).system().before("1"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(resettable_run_once.system())
                    .with_system(make_parallel!(1).system().label("1")),
            )
            .with_system(make_parallel!(2).system().after("1"));
        stage.run(&mut world, &mut resources);
        stage.run(&mut world, &mut resources);
        *resources.get_mut::<bool>().unwrap() = false;
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
        stage.run(&mut world, &mut resources);
        assert_eq!(
            *resources.get::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 2, 0, 1, 2, 0, 2]
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
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
    }
}
