use bevy_utils::{tracing::info, HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;
use std::borrow::Cow;

use super::{
    ExclusiveSystemContainer, ParallelExecutor, ParallelSystemContainer, ParallelSystemExecutor,
    SingleThreadedExecutor, SystemContainer,
};
use crate::{
    InsertionPoint, Resources, RunCriteria,
    ShouldRun::{self, *},
    System, SystemDescriptor, SystemSet, World,
};

pub trait Stage: Downcast + Send + Sync {
    /// Runs the stage; this happens once per update.
    /// Implementors must initialize all of their state and systems before running the first time.
    fn run(&mut self, world: &mut World, resources: &mut Resources);
}

impl_downcast!(Stage);

/// When this resource is present in the `AppBuilder`'s `Resources`,
/// each `SystemStage` will log a report containing
/// pairs of systems with ambiguous execution order.
///
/// Systems that access the same Component or Resource within the same stage
/// risk an ambiguous order that could result in logic bugs, unless they have an
/// explicit execution ordering constraint between them.
///
/// This occurs because, in the absence of explicit constraints, systems are executed in
/// an unstable, arbitrary order within each stage that may vary between runs and frames.
///
/// Some ambiguities reported by the ambiguity checker may be warranted (to allow two systems to run without blocking each other)
/// or spurious, as the exact combination of archetypes used may prevent them from ever conflicting during actual gameplay.
/// You can resolve the warnings produced by the ambiguity checker by adding `.before` or `.after` to one of the conflicting systems
/// referencing the other system to force a specific ordering.
///
/// The checker may report a system more times than the amount of constraints it would actually need to have
/// unambiguous order with regards to a group of already-constrained systems.
pub struct ReportExecutionOrderAmbiguities;

struct VirtualSystemSet {
    run_criteria: RunCriteria,
    should_run: ShouldRun,
}

/// Stores and executes systems. Execution order is not defined unless explicitly specified;
/// see `SystemDescriptor` documentation.
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
    uninitialized_at_start: Vec<usize>,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_before_commands: Vec<usize>,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_at_end: Vec<usize>,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_parallel: Vec<usize>,
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
            uninitialized_parallel: vec![],
            uninitialized_at_start: vec![],
            uninitialized_before_commands: vec![],
            uninitialized_at_end: vec![],
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

    // TODO: consider exposing
    fn add_system_to_set(&mut self, system: impl Into<SystemDescriptor>, set: usize) -> &mut Self {
        self.systems_modified = true;
        match system.into() {
            SystemDescriptor::Exclusive(descriptor) => {
                let insertion_point = descriptor.insertion_point;
                let container = ExclusiveSystemContainer::from_descriptor(descriptor, set);
                match insertion_point {
                    InsertionPoint::AtStart => {
                        let index = self.exclusive_at_start.len();
                        self.uninitialized_at_start.push(index);
                        self.exclusive_at_start.push(container);
                    }
                    InsertionPoint::BeforeCommands => {
                        let index = self.exclusive_before_commands.len();
                        self.uninitialized_before_commands.push(index);
                        self.exclusive_before_commands.push(container);
                    }
                    InsertionPoint::AtEnd => {
                        let index = self.exclusive_at_end.len();
                        self.uninitialized_at_end.push(index);
                        self.exclusive_at_end.push(container);
                    }
                }
            }
            SystemDescriptor::Parallel(descriptor) => {
                self.uninitialized_parallel.push(self.parallel.len());
                self.parallel
                    .push(ParallelSystemContainer::from_descriptor(descriptor, set));
            }
        }
        self
    }

    fn initialize_systems(&mut self, world: &mut World, resources: &mut Resources) {
        for index in self.uninitialized_at_start.drain(..) {
            self.exclusive_at_start[index]
                .system_mut()
                .initialize(world, resources);
        }
        for index in self.uninitialized_before_commands.drain(..) {
            self.exclusive_before_commands[index]
                .system_mut()
                .initialize(world, resources);
        }
        for index in self.uninitialized_at_end.drain(..) {
            self.exclusive_at_end[index]
                .system_mut()
                .initialize(world, resources);
        }
        for index in self.uninitialized_parallel.drain(..) {
            self.parallel[index]
                .system_mut()
                .initialize(world, resources);
        }
    }

    /// Rearranges all systems in topological orders. Systems must be initialized.
    fn rebuild_orders_and_dependencies(&mut self) {
        debug_assert!(
            self.uninitialized_parallel.is_empty()
                && self.uninitialized_at_start.is_empty()
                && self.uninitialized_before_commands.is_empty()
                && self.uninitialized_at_end.is_empty()
        );
        use DependencyGraphError::*;
        match sort_systems(&mut self.parallel) {
            Ok(()) => (),
            Err(LabelNotFound(label)) => {
                panic!("No parallel system with label {:?} in stage.", label)
            }
            Err(DuplicateLabel(label)) => {
                panic!("Label {:?} already used by a parallel system.", label)
            }
            Err(GraphCycles(labels)) => {
                panic!(
                    "Found a dependency cycle in parallel systems: {:?}.",
                    labels
                )
            }
        }
        match sort_systems(&mut self.exclusive_at_start) {
            Ok(()) => (),
            Err(LabelNotFound(label)) => {
                panic!(
                    "No exclusive system with label {:?} at start of stage.",
                    label
                )
            }
            Err(DuplicateLabel(label)) => {
                panic!(
                    "Label {:?} already used by an exclusive system at start of stage.",
                    label
                )
            }
            Err(GraphCycles(labels)) => {
                panic!(
                    "Found a dependency cycle in exclusive systems at start of stage: {:?}.",
                    labels
                )
            }
        }
        match sort_systems(&mut self.exclusive_before_commands) {
            Ok(()) => (),
            Err(LabelNotFound(label)) => {
                panic!(
                    "No exclusive system with label {:?} before commands of stage.",
                    label
                )
            }
            Err(DuplicateLabel(label)) => {
                panic!(
                    "Label {:?} already used by an exclusive system before commands of stage.",
                    label
                )
            }
            Err(GraphCycles(labels)) => {
                panic!(
                    "Found a dependency cycle in exclusive systems before commands of stage: {:?}.",
                    labels
                )
            }
        }
        match sort_systems(&mut self.exclusive_at_end) {
            Ok(()) => (),
            Err(LabelNotFound(label)) => {
                panic!(
                    "No exclusive system with label {:?} at end of stage.",
                    label
                )
            }
            Err(DuplicateLabel(label)) => {
                panic!(
                    "Label {:?} already used by an exclusive system at end of stage.",
                    label
                )
            }
            Err(GraphCycles(labels)) => {
                panic!(
                    "Found a dependency cycle in exclusive systems at end of stage: {:?}.",
                    labels
                )
            }
        }
    }

    /// Logs execution order ambiguities between systems. System orders must be fresh.
    fn report_ambiguities(&self) {
        debug_assert!(!self.systems_modified);
        use std::fmt::Write;
        fn write_display_names_of_pairs(
            string: &mut String,
            systems: &[impl SystemContainer],
            mut ambiguities: Vec<(usize, usize)>,
        ) {
            for (index_a, index_b) in ambiguities.drain(..) {
                writeln!(
                    string,
                    " -- {:?} and {:?}",
                    systems[index_a].display_name(),
                    systems[index_b].display_name()
                )
                .unwrap();
            }
        }
        let parallel = find_ambiguities(&self.parallel);
        let at_start = find_ambiguities(&self.exclusive_at_start);
        let before_commands = find_ambiguities(&self.exclusive_before_commands);
        let at_end = find_ambiguities(&self.exclusive_at_end);
        if !(parallel.is_empty()
            && at_start.is_empty()
            && before_commands.is_empty()
            && at_end.is_empty())
        {
            let mut string = "Execution order ambiguities detected, you might want to \
                    add an explicit dependency relation between some these systems:\n"
                .to_owned();
            if !parallel.is_empty() {
                writeln!(string, " * Parallel systems:").unwrap();
                write_display_names_of_pairs(&mut string, &self.parallel, parallel);
            }
            if !at_start.is_empty() {
                writeln!(string, " * Exclusive systems at start of stage:").unwrap();
                write_display_names_of_pairs(&mut string, &self.exclusive_at_start, at_start);
            }
            if !before_commands.is_empty() {
                writeln!(string, " * Exclusive systems before commands of stage:").unwrap();
                write_display_names_of_pairs(
                    &mut string,
                    &self.exclusive_before_commands,
                    before_commands,
                );
            }
            if !at_end.is_empty() {
                writeln!(string, " * Exclusive systems at end of stage:").unwrap();
                write_display_names_of_pairs(&mut string, &self.exclusive_at_end, at_end);
            }
            info!("{}", string);
        }
    }
}

enum DependencyGraphError {
    LabelNotFound(Cow<'static, str>),
    DuplicateLabel(Cow<'static, str>),
    GraphCycles(Vec<Cow<'static, str>>),
}

/// Sorts given system containers topologically and populates their resolved dependencies.
fn sort_systems(systems: &mut Vec<impl SystemContainer>) -> Result<(), DependencyGraphError> {
    let mut graph = build_dependency_graph(systems)?;
    let order = topological_order(systems, &graph)?;
    let mut order_inverted = order.iter().enumerate().collect::<Vec<_>>();
    order_inverted.sort_unstable_by_key(|(_, &key)| key);
    for (index, container) in systems.iter_mut().enumerate() {
        container.set_dependencies(
            graph
                .get_mut(&index)
                .unwrap()
                .drain(..)
                .map(|index| order_inverted[index].0),
        );
    }
    let mut temp = systems.drain(..).map(Some).collect::<Vec<_>>();
    for index in order {
        systems.push(temp[index].take().unwrap());
    }
    Ok(())
}

/// Constructs a dependency graph of given system containers.
fn build_dependency_graph(
    systems: &[impl SystemContainer],
) -> Result<HashMap<usize, Vec<usize>>, DependencyGraphError> {
    let mut labels = HashMap::<Cow<'static, str>, usize>::default();
    for (label, index) in systems.iter().enumerate().filter_map(|(index, container)| {
        container
            .label()
            .as_ref()
            .cloned()
            .map(|label| (label, index))
    }) {
        if labels.contains_key(&label) {
            return Err(DependencyGraphError::DuplicateLabel(label));
        }
        labels.insert(label, index);
    }
    let mut graph = HashMap::default();
    for (system_index, container) in systems.iter().enumerate() {
        let dependencies = graph.entry(system_index).or_insert_with(Vec::new);
        for label in container.after() {
            match labels.get(label) {
                Some(dependency) => {
                    if !dependencies.contains(dependency) {
                        dependencies.push(*dependency);
                    }
                }
                None => return Err(DependencyGraphError::LabelNotFound(label.clone())),
            }
        }
        for label in container.before() {
            match labels.get(label) {
                Some(dependant) => {
                    let dependencies = graph.entry(*dependant).or_insert_with(Vec::new);
                    if !dependencies.contains(&system_index) {
                        dependencies.push(system_index);
                    }
                }
                None => return Err(DependencyGraphError::LabelNotFound(label.clone())),
            }
        }
    }
    Ok(graph)
}

/// Generates a topological order for the given graph.
fn topological_order(
    systems: &[impl SystemContainer],
    graph: &HashMap<usize, Vec<usize>>,
) -> Result<Vec<usize>, DependencyGraphError> {
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
        current.insert(*node);
        for dependency in graph.get(node).unwrap() {
            if check_if_cycles_and_visit(dependency, &graph, sorted, unvisited, current) {
                return true;
            }
        }
        sorted.push(*node);
        current.remove(node);
        false
    }
    let mut sorted = Vec::with_capacity(graph.len());
    let mut current = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    let mut unvisited = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    unvisited.extend(graph.keys().cloned());
    while let Some(node) = unvisited.iter().next().cloned() {
        if check_if_cycles_and_visit(&node, graph, &mut sorted, &mut unvisited, &mut current) {
            return Err(DependencyGraphError::GraphCycles(
                current
                    .iter()
                    .map(|index| systems[*index].display_name())
                    .collect::<Vec<_>>(),
            ));
        }
    }
    Ok(sorted)
}

/// Returns vector containing all pairs of indices of systems with ambiguous execution order.
/// Systems must be topologically sorted beforehand.
fn find_ambiguities(systems: &[impl SystemContainer]) -> Vec<(usize, usize)> {
    let mut all_dependencies = Vec::<FixedBitSet>::with_capacity(systems.len());
    let mut all_dependants = Vec::<FixedBitSet>::with_capacity(systems.len());
    for (index, container) in systems.iter().enumerate() {
        let mut dependencies = FixedBitSet::with_capacity(systems.len());
        for &dependency in container.dependencies() {
            dependencies.union_with(&all_dependencies[dependency]);
            dependencies.insert(dependency);
            all_dependants[dependency].insert(index);
        }
        all_dependants.push(FixedBitSet::with_capacity(systems.len()));
        all_dependencies.push(dependencies);
    }
    for index in (0..systems.len()).rev() {
        let mut dependants = FixedBitSet::with_capacity(systems.len());
        for dependant in all_dependants[index].ones() {
            dependants.union_with(&all_dependants[dependant]);
            dependants.insert(dependant);
        }
        all_dependants[index] = dependants;
    }
    let mut all_relations = all_dependencies
        .drain(..)
        .zip(all_dependants.drain(..))
        .enumerate()
        .map(|(index, (dependencies, dependants))| {
            let mut relations = FixedBitSet::with_capacity(systems.len());
            relations.union_with(&dependencies);
            relations.union_with(&dependants);
            relations.insert(index);
            relations
        })
        .collect::<Vec<FixedBitSet>>();
    let mut ambiguities = Vec::new();
    let full_bitset: FixedBitSet = (0..systems.len()).collect();
    let mut processed = FixedBitSet::with_capacity(systems.len());
    for (index_a, relations) in all_relations.drain(..).enumerate() {
        // TODO: prove that `.take(index_a)` would be correct here, and uncomment it if so.
        for index_b in full_bitset.difference(&relations)
        /*.take(index_a)*/
        {
            if !processed.contains(index_b) && !systems[index_a].is_compatible(&systems[index_b]) {
                ambiguities.push((index_a, index_b));
            }
        }
        processed.insert(index_a);
    }
    ambiguities
}

impl Stage for SystemStage {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        // Evaluate sets' run criteria, initialize sets as needed, detect if any sets were changed.
        let mut has_work = false;
        for system_set in self.system_sets.iter_mut() {
            let result = system_set.run_criteria.should_run(world, resources);
            match result {
                Yes | YesAndCheckAgain => has_work = true,
                No | NoAndCheckAgain => (),
            }
            system_set.should_run = result;
        }

        if self.systems_modified {
            self.initialize_systems(world, resources);
            self.rebuild_orders_and_dependencies();
            self.systems_modified = false;
            self.executor.rebuild_cached_data(&mut self.parallel, world);
            self.executor_modified = false;
            if resources.contains::<ReportExecutionOrderAmbiguities>() {
                self.report_ambiguities();
            }
        } else if self.executor_modified {
            self.executor.rebuild_cached_data(&mut self.parallel, world);
            self.executor_modified = false;
        }

        while has_work {
            // Run systems that want to be at the start of stage.
            for container in &mut self.exclusive_at_start {
                if let Yes | YesAndCheckAgain = self.system_sets[container.system_set()].should_run
                {
                    container.system_mut().run(world, resources);
                }
            }

            // Run parallel systems using the executor.
            // TODO: hard dependencies, nested sets, whatever... should be evaluated here.
            for container in &mut self.parallel {
                match self.system_sets[container.system_set()].should_run {
                    Yes | YesAndCheckAgain => container.should_run = true,
                    No | NoAndCheckAgain => container.should_run = false,
                }
            }
            self.executor
                .run_systems(&mut self.parallel, world, resources);

            // Run systems that want to be between parallel systems and their command buffers.
            for container in &mut self.exclusive_before_commands {
                if let Yes | YesAndCheckAgain = self.system_sets[container.system_set()].should_run
                {
                    container.system_mut().run(world, resources);
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
                if let Yes | YesAndCheckAgain = self.system_sets[container.system_set()].should_run
                {
                    container.system_mut().run(world, resources);
                }
            }

            // Reevaluate system sets' run criteria.
            has_work = false;
            for system_set in self.system_sets.iter_mut() {
                match system_set.should_run {
                    No => (),
                    Yes => system_set.should_run = No,
                    YesAndCheckAgain | NoAndCheckAgain => {
                        let new_result = system_set.run_criteria.should_run(world, resources);
                        match new_result {
                            Yes | YesAndCheckAgain => has_work = true,
                            No | NoAndCheckAgain => (),
                        }
                        system_set.should_run = new_result;
                    }
                }
            }
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

    fn empty() {}

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
    #[should_panic(expected = "No exclusive system with label \"empty\" at start of stage.")]
    fn exclusive_unknown_label() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().at_end().label("empty"))
            .with_system(empty.exclusive_system().after("empty"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic(
        expected = "Label \"empty\" already used by an exclusive system at start of stage."
    )]
    fn exclusive_duplicate_label() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().at_end().label("empty"))
            .with_system(empty.exclusive_system().before_commands().label("empty"));
        stage.run(&mut world, &mut resources);
        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().label("empty"))
            .with_system(empty.exclusive_system().label("empty"));
        stage.run(&mut world, &mut resources);
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
    #[should_panic(expected = "No parallel system with label \"empty\" in stage.")]
    fn parallel_unknown_label() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(empty.system())
            .with_system(empty.system().after("empty"));
        stage.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic(expected = "Label \"empty\" already used by a parallel system.")]
    fn parallel_duplicate_label() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("empty"))
            .with_system(empty.system().label("empty"));
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
            assert!(container.dependencies().len() <= 1);
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
        use super::{find_ambiguities, SystemContainer};
        use std::borrow::Cow;

        fn find_ambiguities_labels(
            systems: &[impl SystemContainer],
        ) -> Vec<(Cow<'static, str>, Cow<'static, str>)> {
            find_ambiguities(systems)
                .drain(..)
                .map(|(index_a, index_b)| {
                    (
                        systems[index_a].display_name(),
                        systems[index_b].display_name(),
                    )
                })
                .collect()
        }

        fn empty() {}
        fn resource(_: ResMut<usize>) {}
        fn component(_: Query<&mut f32>) {}

        let mut world = World::new();
        let mut resources = Resources::default();

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(empty.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(empty.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(find_ambiguities(&stage.parallel).len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(component.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(component.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&("1".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "1".into()))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(resource.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&("1".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "1".into()))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(component.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(find_ambiguities(&stage.parallel).len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(component.system().label("3").after("2").before("4"))
            .with_system(resource.system().label("4"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&("0".into(), "3".into()))
                || ambiguities.contains(&("3".into(), "0".into()))
        );
        assert!(
            ambiguities.contains(&("1".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "1".into()))
        );
        assert_eq!(ambiguities.len(), 2);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0").before("2"))
            .with_system(component.system().label("1").before("2"))
            .with_system(component.system().label("2"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&("0".into(), "1".into()))
                || ambiguities.contains(&("1".into(), "0".into()))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0"))
            .with_system(component.system().label("1").after("0"))
            .with_system(component.system().label("2").after("0"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&("1".into(), "2".into()))
                || ambiguities.contains(&("2".into(), "1".into()))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0").before("1").before("2"))
            .with_system(component.system().label("1"))
            .with_system(component.system().label("2"))
            .with_system(component.system().label("3").after("1").after("2"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&("1".into(), "2".into()))
                || ambiguities.contains(&("2".into(), "1".into()))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(
                component
                    .system()
                    .label("0")
                    .before("1")
                    .before("2")
                    .before("3")
                    .before("4"),
            )
            .with_system(component.system().label("1"))
            .with_system(component.system().label("2"))
            .with_system(component.system().label("3"))
            .with_system(component.system().label("4"))
            .with_system(
                component
                    .system()
                    .label("5")
                    .after("1")
                    .after("2")
                    .after("3")
                    .after("4"),
            );
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&("1".into(), "2".into()))
                || ambiguities.contains(&("2".into(), "1".into()))
        );
        assert!(
            ambiguities.contains(&("1".into(), "3".into()))
                || ambiguities.contains(&("3".into(), "1".into()))
        );
        assert!(
            ambiguities.contains(&("1".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "1".into()))
        );
        assert!(
            ambiguities.contains(&("2".into(), "3".into()))
                || ambiguities.contains(&("3".into(), "2".into()))
        );
        assert!(
            ambiguities.contains(&("2".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "2".into()))
        );
        assert!(
            ambiguities.contains(&("3".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "3".into()))
        );
        assert_eq!(ambiguities.len(), 6);

        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().label("0"))
            .with_system(empty.exclusive_system().label("1").after("0"))
            .with_system(empty.exclusive_system().label("2").after("1"))
            .with_system(empty.exclusive_system().label("3").after("2"))
            .with_system(empty.exclusive_system().label("4").after("3"))
            .with_system(empty.exclusive_system().label("5").after("4"))
            .with_system(empty.exclusive_system().label("6").after("5"))
            .with_system(empty.exclusive_system().label("7").after("6"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(find_ambiguities(&stage.exclusive_at_start).len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().label("0").before("1").before("3"))
            .with_system(empty.exclusive_system().label("1"))
            .with_system(empty.exclusive_system().label("2").after("1"))
            .with_system(empty.exclusive_system().label("3"))
            .with_system(empty.exclusive_system().label("4").after("3").before("5"))
            .with_system(empty.exclusive_system().label("5"))
            .with_system(empty.exclusive_system().label("6").after("2").after("5"));
        stage.initialize_systems(&mut world, &mut resources);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_labels(&stage.exclusive_at_start);
        assert!(
            ambiguities.contains(&("1".into(), "3".into()))
                || ambiguities.contains(&("3".into(), "1".into()))
        );
        assert!(
            ambiguities.contains(&("2".into(), "3".into()))
                || ambiguities.contains(&("3".into(), "2".into()))
        );
        assert!(
            ambiguities.contains(&("1".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "1".into()))
        );
        assert!(
            ambiguities.contains(&("2".into(), "4".into()))
                || ambiguities.contains(&("4".into(), "2".into()))
        );
        assert!(
            ambiguities.contains(&("1".into(), "5".into()))
                || ambiguities.contains(&("5".into(), "1".into()))
        );
        assert!(
            ambiguities.contains(&("2".into(), "5".into()))
                || ambiguities.contains(&("5".into(), "2".into()))
        );
        assert_eq!(ambiguities.len(), 6);
    }
}
