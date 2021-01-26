use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};
use std::ptr::NonNull;

use super::{
    ExclusiveSystemContainer, ParallelExecutor, ParallelSystemContainer, ParallelSystemExecutor,
    SingleThreadedExecutor, SystemContainer,
};
use crate::{
    topological_sorting, ExclusiveSystem, ExclusiveSystemDescriptor, InsertionPoint,
    ParallelSystemDescriptor, Resources, RunCriteria,
    ShouldRun::{self, *},
    SortingResult, System, SystemId, SystemSet, World,
};

// TODO make use of?
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

struct VirtualSystemSet {
    run_criteria: RunCriteria,
    should_run: ShouldRun,
}

enum SystemKind {
    Parallel,
    ExclusiveAtStart,
    ExclusiveBeforeCommands,
    ExclusiveAtEnd,
}

pub struct SystemStage {
    /// Instance of a scheduling algorithm for running the systems.
    executor: Box<dyn ParallelSystemExecutor>,
    /// Groups of systems; each set has its own run criterion.
    system_sets: Vec<VirtualSystemSet>,
    /// Topologically sorted exclusive systems that want to be ran at the start of the stage.
    exclusive_at_start: Vec<ExclusiveSystemContainer>,
    /// Topologically sorted exclusive systems that want to be ran after parallel systems but
    /// before the application of their command buffers.
    exclusive_before_commands: Vec<ExclusiveSystemContainer>,
    /// Topologically sorted exclusive systems that want to be ran at the end of the stage.
    exclusive_at_end: Vec<ExclusiveSystemContainer>,
    /// Topologically sorted parallel systems.
    parallel: Vec<ParallelSystemContainer>,
    /// Determines if the stage was modified and needs to rebuild its graphs and orders.
    systems_modified: bool,
    /// Determines if the stage's executor was changed.
    executor_modified: bool,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_systems: Vec<(usize, SystemKind)>,
}

impl SystemStage {
    pub fn new(executor: Box<dyn ParallelSystemExecutor>) -> Self {
        let set = VirtualSystemSet {
            run_criteria: Default::default(),
            should_run: ShouldRun::Yes,
        };
        SystemStage {
            executor,
            system_sets: vec![set],
            exclusive_at_start: Default::default(),
            exclusive_before_commands: Default::default(),
            exclusive_at_end: Default::default(),
            parallel: vec![],
            systems_modified: true,
            executor_modified: true,
            uninitialized_systems: vec![],
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
        self.executor_modified = true;
        self.executor.downcast_mut()
    }

    pub fn set_executor(&mut self, executor: Box<dyn ParallelSystemExecutor>) {
        self.executor_modified = true;
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
        self.system_sets[0].run_criteria.set(Box::new(system));
        self
    }

    pub fn add_system_set(&mut self, system_set: SystemSet) -> &mut Self {
        self.systems_modified = true;
        let SystemSet {
            run_criteria,
            mut exclusive_descriptors,
            mut parallel_descriptors,
        } = system_set;
        let set = self.system_sets.len();
        self.system_sets.push(VirtualSystemSet {
            run_criteria,
            should_run: ShouldRun::No,
        });
        for system in exclusive_descriptors.drain(..) {
            self.add_exclusive_system_to_set(system, set);
        }
        for system in parallel_descriptors.drain(..) {
            self.add_system_to_set(system, set);
        }
        self
    }

    // TODO consider exposing
    fn add_system_to_set(
        &mut self,
        system: impl Into<ParallelSystemDescriptor>,
        set: usize,
    ) -> &mut Self {
        self.systems_modified = true;
        let descriptor = system.into();
        self.uninitialized_systems
            .push((self.parallel.len(), SystemKind::Parallel));
        self.parallel.push(ParallelSystemContainer {
            system: unsafe { NonNull::new_unchecked(Box::into_raw(descriptor.system)) },
            should_run: false,
            set,
            dependencies: Vec::new(),
            label: descriptor.label,
            before: descriptor.before,
            after: descriptor.after,
        });
        self
    }

    pub fn add_system(&mut self, system: impl Into<ParallelSystemDescriptor>) -> &mut Self {
        self.add_system_to_set(system, 0)
    }

    // TODO consider exposing
    fn add_exclusive_system_to_set(
        &mut self,
        system: impl Into<ExclusiveSystemDescriptor>,
        set: usize,
    ) -> &mut Self {
        self.systems_modified = true;
        let descriptor = system.into();
        let container = ExclusiveSystemContainer {
            system: descriptor.system,
            set,
            label: descriptor.label,
            before: descriptor.before,
            after: descriptor.after,
        };
        match descriptor.insertion_point {
            InsertionPoint::AtStart => {
                let index = self.exclusive_at_start.len();
                self.uninitialized_systems
                    .push((index, SystemKind::ExclusiveAtStart));
                self.exclusive_at_start.push(container);
            }
            InsertionPoint::BeforeCommands => {
                let index = self.exclusive_before_commands.len();
                self.uninitialized_systems
                    .push((index, SystemKind::ExclusiveBeforeCommands));
                self.exclusive_before_commands.push(container);
            }
            InsertionPoint::AtEnd => {
                let index = self.exclusive_at_end.len();
                self.uninitialized_systems
                    .push((index, SystemKind::ExclusiveAtEnd));
                self.exclusive_at_end.push(container);
            }
        }
        self
    }

    pub fn add_exclusive_system(
        &mut self,
        system: impl Into<ExclusiveSystemDescriptor>,
    ) -> &mut Self {
        self.add_exclusive_system_to_set(system, 0)
    }

    fn initialize_systems(&mut self, world: &mut World, resources: &mut Resources) {
        for (index, kind) in self.uninitialized_systems.drain(..) {
            use SystemKind::*;
            match kind {
                Parallel => self.parallel[index]
                    .system_mut()
                    .initialize(world, resources),
                ExclusiveAtStart => self.exclusive_at_start[index]
                    .system
                    .initialize(world, resources),
                ExclusiveBeforeCommands => self.exclusive_before_commands[index]
                    .system
                    .initialize(world, resources),
                ExclusiveAtEnd => self.exclusive_at_end[index]
                    .system
                    .initialize(world, resources),
            }
        }
    }

    fn rebuild_orders_and_dependencies(&mut self) {
        let mut graph = build_dependency_graph(&self.parallel);
        let order = topological_order_unwrap(&self.parallel, &graph);
        let mut order_inverted = order.iter().enumerate().collect::<Vec<_>>();
        order_inverted.sort_unstable_by_key(|(_, &key)| key);
        for (index, container) in self.parallel.iter_mut().enumerate() {
            container.dependencies.clear();
            container.dependencies.extend(
                graph
                    .get_mut(&index)
                    .unwrap()
                    .drain(..)
                    .map(|index| order_inverted[index].0),
            );
        }
        rearrange_to_order(&mut self.parallel, &order);

        let sort_exclusive = |systems: &mut Vec<ExclusiveSystemContainer>| {
            let graph = build_dependency_graph(systems);
            let order = topological_order_unwrap(systems, &graph);
            rearrange_to_order(systems, &order);
        };
        sort_exclusive(&mut self.exclusive_at_start);
        sort_exclusive(&mut self.exclusive_before_commands);
        sort_exclusive(&mut self.exclusive_before_commands);
    }
}

fn build_dependency_graph(systems: &[impl SystemContainer]) -> HashMap<usize, Vec<usize>> {
    let labels = systems
        .iter()
        .enumerate()
        .filter_map(|(index, container)| container.label().map(|label| (label, index)))
        .collect::<HashMap<Label, usize>>();
    let mut graph = HashMap::default();
    for (system_index, container) in systems.iter().enumerate() {
        let resolve_label = |label| {
            labels
                .get(label)
                // TODO better error message
                .unwrap_or_else(|| panic!("no such system"))
        };
        let children = graph.entry(system_index).or_insert_with(Vec::new);
        for dependency in container.after().iter().map(resolve_label) {
            if !children.contains(dependency) {
                children.push(*dependency);
            }
        }
        for dependant in container.before().iter().map(resolve_label) {
            let children = graph.entry(*dependant).or_insert_with(Vec::new);
            if !children.contains(&system_index) {
                children.push(system_index);
            }
        }
    }
    graph
}

fn topological_order_unwrap(
    systems: &[impl SystemContainer],
    graph: &HashMap<usize, Vec<usize>>,
) -> Vec<usize> {
    match topological_sorting(graph) {
        SortingResult::Sorted(sorted) => sorted,
        // TODO better error
        SortingResult::FoundCycle(cycle) => panic!(
            "found cycle: {:?}",
            cycle
                .iter()
                .map(|index| systems[*index].display_name())
                .collect::<Vec<_>>()
        ),
    }
}

fn rearrange_to_order(systems: &mut Vec<impl SystemContainer>, order: &[usize]) {
    let mut temp = systems.drain(..).map(Some).collect::<Vec<_>>();
    for index in order {
        systems.push(temp[*index].take().unwrap());
    }
}

impl Stage for SystemStage {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        // Evaluate sets' run criteria, initialize sets as needed, detect if any sets were changed.
        let mut has_any_work = false;
        let mut has_doable_work = false;
        for system_set in self.system_sets.iter_mut() {
            let result = system_set.run_criteria.should_run(world, resources);
            match result {
                Yes | YesAndLoop => {
                    has_doable_work = true;
                    has_any_work = true;
                }
                NoAndLoop => has_any_work = true,
                No => (),
            }
            system_set.should_run = result;
        }
        // TODO a real error message
        assert!(!has_any_work || has_doable_work);
        if !has_doable_work {
            return;
        }

        if self.systems_modified {
            self.initialize_systems(world, resources);
            self.rebuild_orders_and_dependencies();
            self.systems_modified = false;
            self.executor.rebuild_cached_data(&mut self.parallel, world);
            self.executor_modified = false;
        } else if self.executor_modified {
            self.executor.rebuild_cached_data(&mut self.parallel, world);
            self.executor_modified = false;
        }

        while has_doable_work {
            // Run systems that want to be at the start of stage.
            for container in &mut self.exclusive_at_start {
                if let Yes | YesAndLoop = self.system_sets[container.set].should_run {
                    container.system.run(world, resources);
                }
            }

            // Run parallel systems using the executor.
            // TODO hard dependencies, nested sets, whatever... should be evaluated here.
            for container in &mut self.parallel {
                match self.system_sets[container.set].should_run {
                    Yes | YesAndLoop => container.should_run = true,
                    No | NoAndLoop => container.should_run = false,
                }
            }
            self.executor
                .run_systems(&mut self.parallel, world, resources);

            // Run systems that want to be between parallel systems and their command buffers.
            for container in &mut self.exclusive_before_commands {
                if let Yes | YesAndLoop = self.system_sets[container.set].should_run {
                    container.system.run(world, resources);
                }
            }

            // Apply parallel systems' buffers.
            for container in &mut self.parallel {
                if container.should_run {
                    container.system_mut().apply_buffers(world, resources);
                }
            }

            // Run systems that want to be at the end of stage.
            for container in &mut self.exclusive_at_end {
                if let Yes | YesAndLoop = self.system_sets[container.set].should_run {
                    container.system.run(world, resources);
                }
            }

            // Reevaluate system sets' run criteria.
            has_any_work = false;
            has_doable_work = false;
            for system_set in self.system_sets.iter_mut() {
                match system_set.should_run {
                    No => (),
                    Yes => system_set.should_run = No,
                    YesAndLoop | NoAndLoop => {
                        let new_result = system_set.run_criteria.should_run(world, resources);
                        match new_result {
                            Yes | YesAndLoop => {
                                has_doable_work = true;
                                has_any_work = true;
                            }
                            NoAndLoop => has_any_work = true,
                            No => (),
                        }
                        system_set.should_run = new_result;
                    }
                }
            }
            // TODO a real error message
            assert!(!has_any_work || has_doable_work);
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
    fn exclusive_redundant_constraints() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(
                make_exclusive(2)
                    .system()
                    .label("2")
                    .after("1")
                    .before("3")
                    .before("3"),
            )
            .with_exclusive_system(
                make_exclusive(1)
                    .system()
                    .label("1")
                    .after("0")
                    .after("0")
                    .before("2"),
            )
            .with_exclusive_system(make_exclusive(0).system().label("0").before("1"))
            .with_exclusive_system(make_exclusive(4).system().label("4").after("3"))
            .with_exclusive_system(make_exclusive(3).system().label("3").after("2").before("4"));
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
    fn exclusive_cycle_1() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(0).system().label("0").after("0"));
        stage.run(&mut world, &mut resources);
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
    fn parallel_redundant_constraints() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(
                make_parallel!(2)
                    .system()
                    .label("2")
                    .after("1")
                    .before("3")
                    .before("3"),
            )
            .with_system(
                make_parallel!(1)
                    .system()
                    .label("1")
                    .after("0")
                    .after("0")
                    .before("2"),
            )
            .with_system(make_parallel!(0).system().label("0").before("1"))
            .with_system(make_parallel!(4).system().label("4").after("3"))
            .with_system(make_parallel!(3).system().label("3").after("2").before("4"));
        stage.run(&mut world, &mut resources);
        for container in stage.parallel.iter() {
            assert!(container.dependencies.len() <= 1);
        }
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
    fn parallel_cycle_1() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage =
            SystemStage::parallel().with_system(make_parallel!(0).system().label("0").after("0"));
        stage.run(&mut world, &mut resources);
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
    fn non_send_resource_system() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(ComputeTaskPool(
            TaskPoolBuilder::new().num_threads(4).build(),
        ));
        resources.insert_non_send(thread::current().id());

        fn wants_non_send(thread_id: NonSend<ThreadId>) {
            assert_eq!(thread::current().id(), *thread_id);
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        let mut stage = SystemStage::parallel()
            .with_system(wants_non_send.system())
            .with_system(wants_non_send.system())
            .with_system(wants_non_send.system())
            .with_system(wants_non_send.system());
        stage.run(&mut world, &mut resources);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
    }
}
