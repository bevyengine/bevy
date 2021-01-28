use bevy_utils::{HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;
use std::{borrow::Cow, iter::FromIterator, ptr::NonNull};

use super::{
    ExclusiveSystemContainer, ParallelExecutor, ParallelSystemContainer, ParallelSystemExecutor,
    SingleThreadedExecutor, SystemContainer,
};
use crate::{
    InsertionPoint, Resources, RunCriteria,
    ShouldRun::{self, *},
    System, SystemDescriptor, SystemId, SystemSet, World,
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

    pub fn single(system: impl Into<SystemDescriptor>) -> Self {
        Self::single_threaded().with_system(system)
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

    pub fn with_system(mut self, system: impl Into<SystemDescriptor>) -> Self {
        self.add_system(system);
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
            mut descriptors,
        } = system_set;
        let set = self.system_sets.len();
        self.system_sets.push(VirtualSystemSet {
            run_criteria,
            should_run: ShouldRun::No,
        });
        for system in descriptors.drain(..) {
            self.add_system_to_set(system, set);
        }
        self
    }

    pub fn add_system(&mut self, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.add_system_to_set(system, 0)
    }

    // TODO consider exposing
    fn add_system_to_set(&mut self, system: impl Into<SystemDescriptor>, set: usize) -> &mut Self {
        self.systems_modified = true;
        match system.into() {
            SystemDescriptor::Exclusive(descriptor) => {
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
            }
            SystemDescriptor::Parallel(descriptor) => {
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
            }
        }
        self
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

    /// Rearranges all systems in topological order, repopulates dependencies of parallel systems
    /// to match the new order, returns pairs of names of systems with ambiguous execution order.
    fn rebuild_orders_and_dependencies(&mut self) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
        let mut ambiguities = Vec::new();
        let mut all_ambiguities = Vec::new();
        let mut all_relations = HashMap::<usize, FixedBitSet>::default();

        let mut graph = build_dependency_graph(&self.parallel);
        let order = topological_order(&self.parallel, &graph);
        populate_relations(&graph, &mut all_relations);
        let full_bitset = FixedBitSet::from_iter(0..self.parallel.len());
        for (index_a, relations) in all_relations.drain() {
            let difference = full_bitset.difference(&relations);
            for index_b in difference {
                let system_a = self.parallel[index_a].system();
                let system_b = self.parallel[index_b].system();
                if !(system_a
                    .component_access()
                    .is_compatible(system_b.component_access())
                    && system_a
                        .resource_access()
                        .is_compatible(system_b.resource_access())
                    || ambiguities.contains(&(index_b, index_a)))
                {
                    ambiguities.push((index_a, index_b));
                }
            }
        }
        all_ambiguities.extend(ambiguities.drain(..).map(|(index_a, index_b)| {
            (
                self.parallel[index_a].display_name(),
                self.parallel[index_b].display_name(),
            )
        }));
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

        let mut sort_exclusive = |systems: &mut Vec<ExclusiveSystemContainer>| {
            let graph = build_dependency_graph(systems);
            let order = topological_order(systems, &graph);
            populate_relations(&graph, &mut all_relations);
            let full_bitset = FixedBitSet::from_iter(0..systems.len());
            for (index_a, relations) in all_relations.drain() {
                let difference = full_bitset.difference(&relations);
                for index_b in difference {
                    if !(ambiguities.contains(&(index_b, index_a))) {
                        ambiguities.push((index_a, index_b));
                    }
                }
            }
            all_ambiguities.extend(ambiguities.drain(..).map(|(index_a, index_b)| {
                (
                    systems[index_a].display_name(),
                    systems[index_b].display_name(),
                )
            }));
            rearrange_to_order(systems, &order);
        };
        sort_exclusive(&mut self.exclusive_at_start);
        sort_exclusive(&mut self.exclusive_before_commands);
        sort_exclusive(&mut self.exclusive_at_end);
        all_ambiguities
    }
}

/// Constructs a dependency graph of given system containers.
fn build_dependency_graph(systems: &[impl SystemContainer]) -> HashMap<usize, Vec<usize>> {
    let labels = systems
        .iter()
        .enumerate()
        .filter_map(|(index, container)| container.label().map(|label| (label, index)))
        .collect::<HashMap<Label, usize>>();
    let resolve_label = |label| {
        labels
            .get(label)
            // TODO better error message
            .unwrap_or_else(|| panic!("no such system"))
    };
    let mut graph = HashMap::default();
    for (system_index, container) in systems.iter().enumerate() {
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

/// Generates a topological order for the given graph; panics if the graph cycles,
/// using given list of system containers to find and print system display names.
fn topological_order(
    systems: &[impl SystemContainer],
    graph: &HashMap<usize, Vec<usize>>,
) -> Vec<usize> {
    fn check_if_cycles_and_visit(
        node: &usize,
        graph: &HashMap<usize, Vec<usize>>,
        sorted: &mut Vec<usize>,
        unvisited: &mut HashSet<usize>,
        current: &mut HashSet<usize>,
    ) -> bool {
        if current.contains(node) {
            return true;
        } else if !unvisited.remove(node) {
            return false;
        }
        current.insert(node.clone());
        for node in graph.get(node).unwrap() {
            if check_if_cycles_and_visit(node, &graph, sorted, unvisited, current) {
                return true;
            }
        }
        sorted.push(node.clone());
        current.remove(node);
        false
    }
    let mut sorted = Vec::with_capacity(graph.len());
    let mut current = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    let mut unvisited = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    unvisited.extend(graph.keys().cloned());
    while let Some(node) = unvisited.iter().next().cloned() {
        if check_if_cycles_and_visit(&node, graph, &mut sorted, &mut unvisited, &mut current) {
            panic!(
                "found cycle: {:?}",
                current
                    .iter()
                    .map(|index| systems[*index].display_name())
                    .collect::<Vec<_>>()
            )
        }
    }
    sorted
}

/// Rearranges given vector of system containers to match the order of the given indices.
fn rearrange_to_order(systems: &mut Vec<impl SystemContainer>, order: &[usize]) {
    let mut temp = systems.drain(..).map(Some).collect::<Vec<_>>();
    for index in order {
        systems.push(temp[*index].take().unwrap());
    }
}

/// Populates the map of all ascendants and descendants using the given graph.
fn populate_relations(
    graph: &HashMap<usize, Vec<usize>>,
    all_relations: &mut HashMap<usize, FixedBitSet>,
) {
    fn add_relations(
        index: usize,
        current_descendant: usize,
        graph: &HashMap<usize, Vec<usize>>,
        all_relations: &mut HashMap<usize, FixedBitSet>,
    ) {
        for &descendant in graph.get(&current_descendant).unwrap() {
            all_relations.get_mut(&index).unwrap().insert(descendant);
            all_relations
                .entry(descendant)
                .or_insert_with(|| FixedBitSet::with_capacity(graph.len()))
                .insert(index);
            add_relations(index, descendant, graph, all_relations);
        }
    }
    all_relations.reserve(graph.len());
    for index in 0..graph.len() {
        all_relations
            .entry(index)
            .or_insert_with(|| FixedBitSet::with_capacity(graph.len()))
            .insert(index);
        add_relations(index, index, graph, all_relations);
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
            let mut ambiguities = self.rebuild_orders_and_dependencies();
            self.systems_modified = false;
            self.executor.rebuild_cached_data(&mut self.parallel, world);
            self.executor_modified = false;
            if !ambiguities.is_empty() {
                println!(
                    "Execution order ambiguities detected, you might want to add an explicit \
                    dependency relation between some these systems:"
                );
                for (system_a, system_b) in ambiguities.drain(..) {
                    println!(" - {:?} and {:?}", system_a, system_b);
                }
            }
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

#[cfg(test)]
mod tests {
    use crate::{prelude::*, SingleThreadedExecutor};

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
            .with_system(make_exclusive(0).exclusive_system().at_start())
            .with_system(make_parallel!(1).system())
            .with_system(make_exclusive(2).exclusive_system().before_commands())
            .with_system(make_exclusive(3).exclusive_system().at_end());
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
            .with_system(make_exclusive(2).exclusive_system().before_commands())
            .with_system(make_exclusive(3).exclusive_system().at_end())
            .with_system(make_parallel!(1).system())
            .with_system(make_exclusive(0).exclusive_system().at_start());
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
            .with_system(make_parallel!(2).exclusive_system().before_commands())
            .with_system(make_parallel!(3).exclusive_system().at_end())
            .with_system(make_parallel!(1).system())
            .with_system(make_parallel!(0).exclusive_system().at_start());
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
            .with_system(make_exclusive(1).exclusive_system().label("1").after("0"))
            .with_system(make_exclusive(2).exclusive_system().after("1"))
            .with_system(make_exclusive(0).exclusive_system().label("0"));
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
            .with_system(make_exclusive(1).exclusive_system().label("1").before("2"))
            .with_system(make_exclusive(2).exclusive_system().label("2"))
            .with_system(make_exclusive(0).exclusive_system().before("1"));
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
            .with_system(make_exclusive(2).exclusive_system().label("2"))
            .with_system(make_exclusive(1).exclusive_system().after("0").before("2"))
            .with_system(make_exclusive(0).exclusive_system().label("0"))
            .with_system(make_exclusive(4).exclusive_system().label("4"))
            .with_system(make_exclusive(3).exclusive_system().after("2").before("4"));
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
            .with_system(
                make_exclusive(2)
                    .exclusive_system()
                    .label("2")
                    .after("1")
                    .before("3")
                    .before("3"),
            )
            .with_system(
                make_exclusive(1)
                    .exclusive_system()
                    .label("1")
                    .after("0")
                    .after("0")
                    .before("2"),
            )
            .with_system(make_exclusive(0).exclusive_system().label("0").before("1"))
            .with_system(make_exclusive(4).exclusive_system().label("4").after("3"))
            .with_system(
                make_exclusive(3)
                    .exclusive_system()
                    .label("3")
                    .after("2")
                    .before("4"),
            );
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
            .with_system(make_exclusive(2).exclusive_system().label("2"))
            .with_system_set(
                SystemSet::new()
                    .with_system(make_exclusive(0).exclusive_system().label("0"))
                    .with_system(make_exclusive(4).exclusive_system().label("4"))
                    .with_system(make_exclusive(3).exclusive_system().after("2").before("4")),
            )
            .with_system(make_exclusive(1).exclusive_system().after("0").before("2"));
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
            .with_system(make_exclusive(0).exclusive_system().before("1"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(resettable_run_once.system())
                    .with_system(make_exclusive(1).exclusive_system().label("1")),
            )
            .with_system(make_exclusive(2).exclusive_system().after("1"));
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
            .with_system(make_exclusive(0).exclusive_system().label("0").after("0"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_2() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().label("0").after("1"))
            .with_system(make_exclusive(1).exclusive_system().label("1").after("0"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_3() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().label("0"))
            .with_system(make_exclusive(1).exclusive_system().after("0").before("2"))
            .with_system(make_exclusive(2).exclusive_system().label("2").before("0"));
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
    fn ambiguity_detection() {
        fn empty() {}
        fn resource(_: ResMut<usize>) {}
        fn component(_: Query<&mut f32>) {}
        let mut world = World::new();
        let mut resources = Resources::default();

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(empty.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().after("2").before("4"))
            .with_system(empty.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        assert_eq!(stage.rebuild_orders_and_dependencies().len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(component.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().after("2").before("4"))
            .with_system(component.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(stage.rebuild_orders_and_dependencies().len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().after("2").before("4"))
            .with_system(resource.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        assert_eq!(stage.rebuild_orders_and_dependencies().len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().after("2").before("4"))
            .with_system(component.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        assert_eq!(stage.rebuild_orders_and_dependencies().len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(component.system().after("2").before("4"))
            .with_system(resource.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        assert_eq!(stage.rebuild_orders_and_dependencies().len(), 2);

        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().label("0"))
            .with_system(empty.exclusive_system().label("1").after("0"))
            .with_system(empty.exclusive_system().label("2"))
            .with_system(empty.exclusive_system().after("2").before("4"))
            .with_system(empty.exclusive_system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        assert_eq!(stage.rebuild_orders_and_dependencies().len(), 6);
    }
}
