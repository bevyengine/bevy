use crate::{
    component::ComponentId,
    schedule::{
        BoxedSystemLabel, ExclusiveSystemContainer, InsertionPoint, ParallelExecutor,
        ParallelSystemContainer, ParallelSystemExecutor, RunCriteria, ShouldRun,
        SingleThreadedExecutor, SystemContainer, SystemDescriptor, SystemSet,
    },
    system::System,
    world::{World, WorldId},
};
use bevy_utils::{
    tracing::{info, warn},
    HashMap, HashSet,
};
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;
use std::borrow::Cow;

pub trait Stage: Downcast + Send + Sync {
    /// Runs the stage; this happens once per update.
    /// Implementors must initialize all of their state and systems before running the first time.
    fn run(&mut self, world: &mut World);
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
/// Some ambiguities reported by the ambiguity checker may be warranted (to allow two systems to run
/// without blocking each other) or spurious, as the exact combination of archetypes used may
/// prevent them from ever conflicting during actual gameplay. You can resolve the warnings produced
/// by the ambiguity checker by adding `.before` or `.after` to one of the conflicting systems
/// referencing the other system to force a specific ordering.
///
/// The checker may report a system more times than the amount of constraints it would actually need
/// to have unambiguous order with regards to a group of already-constrained systems.
pub struct ReportExecutionOrderAmbiguities;

struct VirtualSystemSet {
    run_criteria: RunCriteria,
    should_run: ShouldRun,
}

/// Stores and executes systems. Execution order is not defined unless explicitly specified;
/// see `SystemDescriptor` documentation.
pub struct SystemStage {
    /// The WorldId this stage was last run on.
    world_id: Option<WorldId>,
    /// Instance of a scheduling algorithm for running the systems.
    executor: Box<dyn ParallelSystemExecutor>,
    /// Groups of systems; each set has its own run criterion.
    system_sets: Vec<VirtualSystemSet>,
    /// Topologically sorted exclusive systems that want to be run at the start of the stage.
    exclusive_at_start: Vec<ExclusiveSystemContainer>,
    /// Topologically sorted exclusive systems that want to be run after parallel systems but
    /// before the application of their command buffers.
    exclusive_before_commands: Vec<ExclusiveSystemContainer>,
    /// Topologically sorted exclusive systems that want to be run at the end of the stage.
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
            world_id: None,
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

    /// Topologically sorted parallel systems.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn parallel_systems(&self) -> &[impl SystemContainer] {
        &self.parallel
    }
    /// Topologically sorted exclusive systems that want to be run at the start of the stage.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn exclusive_at_start_systems(&self) -> &[impl SystemContainer] {
        &self.exclusive_at_start
    }
    /// Topologically sorted exclusive systems that want to be run at the end of the stage.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn exclusive_at_end_systems(&self) -> &[impl SystemContainer] {
        &self.exclusive_at_end
    }
    /// Topologically sorted exclusive systems that want to be run after parallel systems but
    /// before the application of their command buffers.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn exclusive_before_commands_systems(&self) -> &[impl SystemContainer] {
        &self.exclusive_before_commands
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

    fn initialize_systems(&mut self, world: &mut World) {
        for index in self.uninitialized_at_start.drain(..) {
            self.exclusive_at_start[index]
                .system_mut()
                .initialize(world);
        }
        for index in self.uninitialized_before_commands.drain(..) {
            self.exclusive_before_commands[index]
                .system_mut()
                .initialize(world);
        }
        for index in self.uninitialized_at_end.drain(..) {
            self.exclusive_at_end[index].system_mut().initialize(world);
        }
        for index in self.uninitialized_parallel.drain(..) {
            self.parallel[index].system_mut().initialize(world);
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
        fn sort_systems_unwrap(
            systems: &mut Vec<impl SystemContainer>,
            systems_description: &'static str,
        ) {
            if let Err(DependencyGraphError::GraphCycles(cycle)) = sort_systems(systems) {
                use std::fmt::Write;
                let mut message = format!("Found a dependency cycle in {}:", systems_description);
                writeln!(message).unwrap();
                for (name, labels) in &cycle {
                    writeln!(message, " - {}", name).unwrap();
                    writeln!(
                        message,
                        "    wants to be after (because of labels {:?})",
                        labels
                    )
                    .unwrap();
                }
                writeln!(message, " - {}", cycle[0].0).unwrap();
                panic!("{}", message);
            }
        }
        sort_systems_unwrap(&mut self.parallel, "parallel systems");
        sort_systems_unwrap(
            &mut self.exclusive_at_start,
            "exclusive systems at start of stage",
        );
        sort_systems_unwrap(
            &mut self.exclusive_before_commands,
            "exclusive systems before commands of stage",
        );
        sort_systems_unwrap(
            &mut self.exclusive_at_end,
            "exclusive systems at end of stage",
        );
    }

    /// Logs execution order ambiguities between systems. System orders must be fresh.
    fn report_ambiguities(&self, world: &World) {
        debug_assert!(!self.systems_modified);
        use std::fmt::Write;
        fn write_display_names_of_pairs(
            string: &mut String,
            systems: &[impl SystemContainer],
            mut ambiguities: Vec<(usize, usize, Vec<ComponentId>)>,
            world: &World,
        ) {
            for (index_a, index_b, conflicts) in ambiguities.drain(..) {
                writeln!(
                    string,
                    " -- {:?} and {:?}",
                    systems[index_a].name(),
                    systems[index_b].name()
                )
                .unwrap();
                if !conflicts.is_empty() {
                    let names = conflicts
                        .iter()
                        .map(|id| world.components().get_info(*id).unwrap().name())
                        .collect::<Vec<_>>();
                    writeln!(string, "    conflicts: {:?}", names).unwrap();
                }
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
                write_display_names_of_pairs(&mut string, &self.parallel, parallel, world);
            }
            if !at_start.is_empty() {
                writeln!(string, " * Exclusive systems at start of stage:").unwrap();
                write_display_names_of_pairs(
                    &mut string,
                    &self.exclusive_at_start,
                    at_start,
                    world,
                );
            }
            if !before_commands.is_empty() {
                writeln!(string, " * Exclusive systems before commands of stage:").unwrap();
                write_display_names_of_pairs(
                    &mut string,
                    &self.exclusive_before_commands,
                    before_commands,
                    world,
                );
            }
            if !at_end.is_empty() {
                writeln!(string, " * Exclusive systems at end of stage:").unwrap();
                write_display_names_of_pairs(&mut string, &self.exclusive_at_end, at_end, world);
            }
            info!("{}", string);
        }
    }
}

enum DependencyGraphError {
    GraphCycles(Vec<(Cow<'static, str>, Vec<BoxedSystemLabel>)>),
}

/// Sorts given system containers topologically and populates their resolved dependencies.
fn sort_systems(systems: &mut Vec<impl SystemContainer>) -> Result<(), DependencyGraphError> {
    let mut graph = build_dependency_graph(systems);
    let order = topological_order(systems, &graph)?;
    let mut order_inverted = order.iter().enumerate().collect::<Vec<_>>();
    order_inverted.sort_unstable_by_key(|(_, &key)| key);
    for (index, container) in systems.iter_mut().enumerate() {
        container.set_dependencies(
            graph
                .get_mut(&index)
                .unwrap()
                .drain()
                .map(|(index, _)| order_inverted[index].0),
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
) -> HashMap<usize, HashMap<usize, HashSet<BoxedSystemLabel>>> {
    let mut labelled_systems = HashMap::<BoxedSystemLabel, FixedBitSet>::default();
    for (label, index) in systems.iter().enumerate().flat_map(|(index, container)| {
        container
            .labels()
            .iter()
            .cloned()
            .map(move |label| (label, index))
    }) {
        labelled_systems
            .entry(label)
            .or_insert_with(|| FixedBitSet::with_capacity(systems.len()))
            .insert(index);
    }
    let mut graph = HashMap::with_capacity_and_hasher(systems.len(), Default::default());
    for (system_index, container) in systems.iter().enumerate() {
        let dependencies = graph.entry(system_index).or_insert_with(HashMap::default);
        for label in container.after() {
            match labelled_systems.get(label) {
                Some(new_dependencies) => {
                    for dependency in new_dependencies.ones() {
                        dependencies
                            .entry(dependency)
                            .or_insert_with(HashSet::default)
                            .insert(label.clone());
                    }
                }
                None => warn!(
                    "System {} wants to be after unknown system label: {:?}",
                    systems[system_index].name(),
                    label
                ),
            }
        }
        for label in container.before() {
            match labelled_systems.get(label) {
                Some(dependants) => {
                    for dependant in dependants.ones() {
                        graph
                            .entry(dependant)
                            .or_insert_with(HashMap::default)
                            .entry(system_index)
                            .or_insert_with(HashSet::default)
                            .insert(label.clone());
                    }
                }
                None => warn!(
                    "System {} wants to be before unknown system label: {:?}",
                    systems[system_index].name(),
                    label
                ),
            }
        }
    }
    graph
}

/// Generates a topological order for the given graph.
fn topological_order(
    systems: &[impl SystemContainer],
    graph: &HashMap<usize, HashMap<usize, HashSet<BoxedSystemLabel>>>,
) -> Result<Vec<usize>, DependencyGraphError> {
    fn check_if_cycles_and_visit(
        node: &usize,
        graph: &HashMap<usize, HashMap<usize, HashSet<BoxedSystemLabel>>>,
        sorted: &mut Vec<usize>,
        unvisited: &mut HashSet<usize>,
        current: &mut Vec<usize>,
    ) -> bool {
        if current.contains(node) {
            return true;
        } else if !unvisited.remove(node) {
            return false;
        }
        current.push(*node);
        for dependency in graph.get(node).unwrap().keys() {
            if check_if_cycles_and_visit(dependency, &graph, sorted, unvisited, current) {
                return true;
            }
        }
        sorted.push(*node);
        current.pop();
        false
    }
    let mut sorted = Vec::with_capacity(graph.len());
    let mut current = Vec::with_capacity(graph.len());
    let mut unvisited = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    unvisited.extend(graph.keys().cloned());
    while let Some(node) = unvisited.iter().next().cloned() {
        if check_if_cycles_and_visit(&node, graph, &mut sorted, &mut unvisited, &mut current) {
            let mut cycle = Vec::new();
            let last_window = [*current.last().unwrap(), current[0]];
            let mut windows = current
                .windows(2)
                .chain(std::iter::once(&last_window as &[usize]));
            while let Some(&[dependant, dependency]) = windows.next() {
                cycle.push((
                    systems[dependant].name(),
                    graph[&dependant][&dependency].iter().cloned().collect(),
                ));
            }
            return Err(DependencyGraphError::GraphCycles(cycle));
        }
    }
    Ok(sorted)
}

/// Returns vector containing all pairs of indices of systems with ambiguous execution order,
/// along with specific components that have triggered the warning.
/// Systems must be topologically sorted beforehand.
fn find_ambiguities(systems: &[impl SystemContainer]) -> Vec<(usize, usize, Vec<ComponentId>)> {
    let mut ambiguity_set_labels = HashMap::default();
    for set in systems.iter().flat_map(|c| c.ambiguity_sets()) {
        let len = ambiguity_set_labels.len();
        ambiguity_set_labels.entry(set).or_insert(len);
    }
    let mut all_ambiguity_sets = Vec::<FixedBitSet>::with_capacity(systems.len());
    let mut all_dependencies = Vec::<FixedBitSet>::with_capacity(systems.len());
    let mut all_dependants = Vec::<FixedBitSet>::with_capacity(systems.len());
    for (index, container) in systems.iter().enumerate() {
        let mut ambiguity_sets = FixedBitSet::with_capacity(ambiguity_set_labels.len());
        for set in container.ambiguity_sets() {
            ambiguity_sets.insert(ambiguity_set_labels[set]);
        }
        all_ambiguity_sets.push(ambiguity_sets);
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
        // .take(index_a)
        {
            if !processed.contains(index_b)
                && all_ambiguity_sets[index_a].is_disjoint(&all_ambiguity_sets[index_b])
            {
                let a_access = systems[index_a].component_access();
                let b_access = systems[index_b].component_access();
                if let (Some(a), Some(b)) = (a_access, b_access) {
                    let conflicts = a.get_conflicts(b);
                    if !conflicts.is_empty() {
                        ambiguities.push((index_a, index_b, conflicts))
                    }
                } else {
                    ambiguities.push((index_a, index_b, Vec::new()));
                }
            }
        }
        processed.insert(index_a);
    }
    ambiguities
}

impl Stage for SystemStage {
    fn run(&mut self, world: &mut World) {
        if let Some(world_id) = self.world_id {
            assert!(
                world.id() == world_id,
                "Cannot run SystemStage on two different Worlds"
            );
        } else {
            self.world_id = Some(world.id());
        }
        // Evaluate sets' run criteria, initialize sets as needed, detect if any sets were changed.
        let mut has_work = false;
        for system_set in self.system_sets.iter_mut() {
            let result = system_set.run_criteria.should_run(world);
            match result {
                ShouldRun::Yes | ShouldRun::YesAndCheckAgain => has_work = true,
                ShouldRun::No | ShouldRun::NoAndCheckAgain => (),
            }
            system_set.should_run = result;
        }

        if self.systems_modified {
            self.initialize_systems(world);
            self.rebuild_orders_and_dependencies();
            self.systems_modified = false;
            self.executor.rebuild_cached_data(&self.parallel);
            self.executor_modified = false;
            if world.contains_resource::<ReportExecutionOrderAmbiguities>() {
                self.report_ambiguities(world);
            }
        } else if self.executor_modified {
            self.executor.rebuild_cached_data(&self.parallel);
            self.executor_modified = false;
        }

        while has_work {
            // Run systems that want to be at the start of stage.
            for container in &mut self.exclusive_at_start {
                if let ShouldRun::Yes | ShouldRun::YesAndCheckAgain =
                    self.system_sets[container.system_set()].should_run
                {
                    container.system_mut().run(world);
                }
            }

            // Run parallel systems using the executor.
            // TODO: hard dependencies, nested sets, whatever... should be evaluated here.
            for container in &mut self.parallel {
                match self.system_sets[container.system_set()].should_run {
                    ShouldRun::Yes | ShouldRun::YesAndCheckAgain => container.should_run = true,
                    ShouldRun::No | ShouldRun::NoAndCheckAgain => container.should_run = false,
                }
            }
            self.executor.run_systems(&mut self.parallel, world);

            // Run systems that want to be between parallel systems and their command buffers.
            for container in &mut self.exclusive_before_commands {
                if let ShouldRun::Yes | ShouldRun::YesAndCheckAgain =
                    self.system_sets[container.system_set()].should_run
                {
                    container.system_mut().run(world);
                }
            }

            // Apply parallel systems' buffers.
            for container in &mut self.parallel {
                if container.should_run {
                    container.system_mut().apply_buffers(world);
                }
            }

            // Run systems that want to be at the end of stage.
            for container in &mut self.exclusive_at_end {
                if let ShouldRun::Yes | ShouldRun::YesAndCheckAgain =
                    self.system_sets[container.system_set()].should_run
                {
                    container.system_mut().run(world);
                }
            }

            // Reevaluate system sets' run criteria.
            has_work = false;
            for system_set in self.system_sets.iter_mut() {
                match system_set.should_run {
                    ShouldRun::No => (),
                    ShouldRun::Yes => system_set.should_run = ShouldRun::No,
                    ShouldRun::YesAndCheckAgain | ShouldRun::NoAndCheckAgain => {
                        let new_result = system_set.run_criteria.should_run(world);
                        match new_result {
                            ShouldRun::Yes | ShouldRun::YesAndCheckAgain => has_work = true,
                            ShouldRun::No | ShouldRun::NoAndCheckAgain => (),
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
    use crate::{
        schedule::{
            BoxedSystemLabel, ExclusiveSystemDescriptorCoercion, ParallelSystemDescriptorCoercion,
            ShouldRun, SingleThreadedExecutor, Stage, SystemSet, SystemStage,
        },
        system::{IntoExclusiveSystem, IntoSystem, Query, ResMut},
        world::World,
    };

    fn make_exclusive(tag: usize) -> impl FnMut(&mut World) {
        move |world| world.get_resource_mut::<Vec<usize>>().unwrap().push(tag)
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
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().at_start())
            .with_system(make_parallel!(1).system())
            .with_system(make_exclusive(2).exclusive_system().before_commands())
            .with_system(make_exclusive(3).exclusive_system().at_end());
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource_mut::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3]
        );
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).exclusive_system().before_commands())
            .with_system(make_exclusive(3).exclusive_system().at_end())
            .with_system(make_parallel!(1).system())
            .with_system(make_exclusive(0).exclusive_system().at_start());
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3]
        );
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(2).exclusive_system().before_commands())
            .with_system(make_parallel!(3).exclusive_system().at_end())
            .with_system(make_parallel!(1).system())
            .with_system(make_parallel!(0).exclusive_system().at_start());
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3]
        );
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );
    }

    #[test]
    fn exclusive_after() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(1).exclusive_system().label("1").after("0"))
            .with_system(make_exclusive(2).exclusive_system().after("1"))
            .with_system(make_exclusive(0).exclusive_system().label("0"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn exclusive_before() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(1).exclusive_system().label("1").before("2"))
            .with_system(make_exclusive(2).exclusive_system().label("2"))
            .with_system(make_exclusive(0).exclusive_system().before("1"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn exclusive_mixed() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).exclusive_system().label("2"))
            .with_system(make_exclusive(1).exclusive_system().after("0").before("2"))
            .with_system(make_exclusive(0).exclusive_system().label("0"))
            .with_system(make_exclusive(4).exclusive_system().label("4"))
            .with_system(make_exclusive(3).exclusive_system().after("2").before("4"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_multiple_labels() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(
                make_exclusive(1)
                    .exclusive_system()
                    .label("first")
                    .after("0"),
            )
            .with_system(make_exclusive(2).exclusive_system().after("first"))
            .with_system(
                make_exclusive(0)
                    .exclusive_system()
                    .label("first")
                    .label("0"),
            );
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).exclusive_system().after("01").label("2"))
            .with_system(make_exclusive(1).exclusive_system().label("01").after("0"))
            .with_system(make_exclusive(0).exclusive_system().label("01").label("0"))
            .with_system(make_exclusive(4).exclusive_system().label("4"))
            .with_system(make_exclusive(3).exclusive_system().after("2").before("4"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).exclusive_system().label("234").label("2"))
            .with_system(
                make_exclusive(1)
                    .exclusive_system()
                    .before("234")
                    .after("0"),
            )
            .with_system(make_exclusive(0).exclusive_system().label("0"))
            .with_system(make_exclusive(4).exclusive_system().label("234").label("4"))
            .with_system(
                make_exclusive(3)
                    .exclusive_system()
                    .label("234")
                    .after("2")
                    .before("4"),
            );
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_redundant_constraints() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
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
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_mixed_across_sets() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).exclusive_system().label("2"))
            .with_system_set(
                SystemSet::new()
                    .with_system(make_exclusive(0).exclusive_system().label("0"))
                    .with_system(make_exclusive(4).exclusive_system().label("4"))
                    .with_system(make_exclusive(3).exclusive_system().after("2").before("4")),
            )
            .with_system(make_exclusive(1).exclusive_system().after("0").before("2"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_run_criteria() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        world.insert_resource(false);
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().before("1"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(resettable_run_once.system())
                    .with_system(make_exclusive(1).exclusive_system().label("1")),
            )
            .with_system(make_exclusive(2).exclusive_system().after("1"));
        stage.run(&mut world);
        stage.run(&mut world);
        *world.get_resource_mut::<bool>().unwrap() = false;
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 2, 0, 1, 2, 0, 2]
        );
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_1() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().label("0").after("0"));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_2() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().label("0").after("1"))
            .with_system(make_exclusive(1).exclusive_system().label("1").after("0"));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_3() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().label("0"))
            .with_system(make_exclusive(1).exclusive_system().after("0").before("2"))
            .with_system(make_exclusive(2).exclusive_system().label("2").before("0"));
        stage.run(&mut world);
    }

    #[test]
    fn parallel_after() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(1).system().after("0").label("1"))
            .with_system(make_parallel!(2).system().after("1"))
            .with_system(make_parallel!(0).system().label("0"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn parallel_before() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(1).system().label("1").before("2"))
            .with_system(make_parallel!(2).system().label("2"))
            .with_system(make_parallel!(0).system().before("1"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );
    }

    #[test]
    fn parallel_mixed() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(2).system().label("2"))
            .with_system(make_parallel!(1).system().after("0").before("2"))
            .with_system(make_parallel!(0).system().label("0"))
            .with_system(make_parallel!(4).system().label("4"))
            .with_system(make_parallel!(3).system().after("2").before("4"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_multiple_labels() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(1).system().label("first").after("0"))
            .with_system(make_parallel!(2).system().after("first"))
            .with_system(make_parallel!(0).system().label("first").label("0"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(2).system().after("01").label("2"))
            .with_system(make_parallel!(1).system().label("01").after("0"))
            .with_system(make_parallel!(0).system().label("01").label("0"))
            .with_system(make_parallel!(4).system().label("4"))
            .with_system(make_parallel!(3).system().after("2").before("4"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(2).system().label("234").label("2"))
            .with_system(make_parallel!(1).system().before("234").after("0"))
            .with_system(make_parallel!(0).system().label("0"))
            .with_system(make_parallel!(4).system().label("234").label("4"))
            .with_system(
                make_parallel!(3)
                    .system()
                    .label("234")
                    .after("2")
                    .before("4"),
            );
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_redundant_constraints() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
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
        stage.run(&mut world);
        for container in stage.parallel.iter() {
            assert!(container.dependencies().len() <= 1);
        }
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_mixed_across_sets() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(2).system().label("2"))
            .with_system_set(
                SystemSet::new()
                    .with_system(make_parallel!(0).system().label("0"))
                    .with_system(make_parallel!(4).system().label("4"))
                    .with_system(make_parallel!(3).system().after("2").before("4")),
            )
            .with_system(make_parallel!(1).system().after("0").before("2"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_run_criteria() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        world.insert_resource(false);
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(0).system().before("1"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(resettable_run_once.system())
                    .with_system(make_parallel!(1).system().label("1")),
            )
            .with_system(make_parallel!(2).system().after("1"));
        stage.run(&mut world);
        stage.run(&mut world);
        *world.get_resource_mut::<bool>().unwrap() = false;
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 2, 0, 1, 2, 0, 2]
        );
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_1() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage =
            SystemStage::parallel().with_system(make_parallel!(0).system().label("0").after("0"));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_2() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(0).system().label("0").after("1"))
            .with_system(make_parallel!(1).system().label("1").after("0"));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_3() {
        let mut world = World::new();

        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel!(0).system().label("0"))
            .with_system(make_parallel!(1).system().after("0").before("2"))
            .with_system(make_parallel!(2).system().label("2").before("0"));
        stage.run(&mut world);
    }

    #[test]
    fn ambiguity_detection() {
        use super::{find_ambiguities, SystemContainer};

        fn find_ambiguities_first_labels(
            systems: &[impl SystemContainer],
        ) -> Vec<(BoxedSystemLabel, BoxedSystemLabel)> {
            find_ambiguities(systems)
                .drain(..)
                .map(|(index_a, index_b, _conflicts)| {
                    (
                        systems[index_a].labels()[0].clone(),
                        systems[index_b].labels()[0].clone(),
                    )
                })
                .collect()
        }

        fn empty() {}
        fn resource(_: ResMut<usize>) {}
        fn component(_: Query<&mut f32>) {}

        let mut world = World::new();

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(empty.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(empty.system().label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(find_ambiguities(&stage.parallel).len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(component.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(component.system().label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(resource.system().label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(empty.system().label("3").after("2").before("4"))
            .with_system(component.system().label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(find_ambiguities(&stage.parallel).len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0"))
            .with_system(resource.system().label("1").after("0"))
            .with_system(empty.system().label("2"))
            .with_system(component.system().label("3").after("2").before("4"))
            .with_system(resource.system().label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("0"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("0")))
        );
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 2);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0"))
            .with_system(
                resource
                    .system()
                    .label("1")
                    .after("0")
                    .in_ambiguity_set("a"),
            )
            .with_system(empty.system().label("2"))
            .with_system(component.system().label("3").after("2").before("4"))
            .with_system(resource.system().label("4").in_ambiguity_set("a"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("0"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("0")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0").before("2"))
            .with_system(component.system().label("1").before("2"))
            .with_system(component.system().label("2"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("0"), Box::new("1")))
                || ambiguities.contains(&(Box::new("1"), Box::new("0")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0"))
            .with_system(component.system().label("1").after("0"))
            .with_system(component.system().label("2").after("0"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("2")))
                || ambiguities.contains(&(Box::new("2"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0").before("1").before("2"))
            .with_system(component.system().label("1"))
            .with_system(component.system().label("2"))
            .with_system(component.system().label("3").after("1").after("2"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("2")))
                || ambiguities.contains(&(Box::new("2"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0").before("1").before("2"))
            .with_system(component.system().label("1").in_ambiguity_set("a"))
            .with_system(component.system().label("2").in_ambiguity_set("a"))
            .with_system(component.system().label("3").after("1").after("2"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert_eq!(ambiguities.len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(component.system().label("0").before("1").before("2"))
            .with_system(component.system().label("1").in_ambiguity_set("a"))
            .with_system(component.system().label("2").in_ambiguity_set("b"))
            .with_system(component.system().label("3").after("1").after("2"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("2")))
                || ambiguities.contains(&(Box::new("2"), Box::new("1")))
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
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("2")))
                || ambiguities.contains(&(Box::new("2"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("2")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("2")))
        );
        assert!(
            ambiguities.contains(&(Box::new("3"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("3")))
        );
        assert_eq!(ambiguities.len(), 6);

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
            .with_system(component.system().label("1").in_ambiguity_set("a"))
            .with_system(component.system().label("2").in_ambiguity_set("a"))
            .with_system(component.system().label("3").in_ambiguity_set("a"))
            .with_system(component.system().label("4").in_ambiguity_set("a"))
            .with_system(
                component
                    .system()
                    .label("5")
                    .after("1")
                    .after("2")
                    .after("3")
                    .after("4"),
            );
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert_eq!(ambiguities.len(), 0);

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
            .with_system(component.system().label("1").in_ambiguity_set("a"))
            .with_system(component.system().label("2").in_ambiguity_set("a"))
            .with_system(
                component
                    .system()
                    .label("3")
                    .in_ambiguity_set("a")
                    .in_ambiguity_set("b"),
            )
            .with_system(component.system().label("4").in_ambiguity_set("b"))
            .with_system(
                component
                    .system()
                    .label("5")
                    .after("1")
                    .after("2")
                    .after("3")
                    .after("4"),
            );
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("2")))
        );
        assert_eq!(ambiguities.len(), 2);

        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().label("0"))
            .with_system(empty.exclusive_system().label("1").after("0"))
            .with_system(empty.exclusive_system().label("2").after("1"))
            .with_system(empty.exclusive_system().label("3").after("2"))
            .with_system(empty.exclusive_system().label("4").after("3"))
            .with_system(empty.exclusive_system().label("5").after("4"))
            .with_system(empty.exclusive_system().label("6").after("5"))
            .with_system(empty.exclusive_system().label("7").after("6"));
        stage.initialize_systems(&mut world);
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
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.exclusive_at_start);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("2")))
        );
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("2")))
        );
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("5")))
                || ambiguities.contains(&(Box::new("5"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("5")))
                || ambiguities.contains(&(Box::new("5"), Box::new("2")))
        );
        assert_eq!(ambiguities.len(), 6);

        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().label("0").before("1").before("3"))
            .with_system(empty.exclusive_system().label("1").in_ambiguity_set("a"))
            .with_system(empty.exclusive_system().label("2").after("1"))
            .with_system(empty.exclusive_system().label("3").in_ambiguity_set("a"))
            .with_system(empty.exclusive_system().label("4").after("3").before("5"))
            .with_system(empty.exclusive_system().label("5").in_ambiguity_set("a"))
            .with_system(empty.exclusive_system().label("6").after("2").after("5"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.exclusive_at_start);
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("2")))
        );
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("2")))
        );
        assert!(
            ambiguities.contains(&(Box::new("2"), Box::new("5")))
                || ambiguities.contains(&(Box::new("5"), Box::new("2")))
        );
        assert_eq!(ambiguities.len(), 4);

        let mut stage = SystemStage::parallel()
            .with_system(empty.exclusive_system().label("0").in_ambiguity_set("a"))
            .with_system(empty.exclusive_system().label("1").in_ambiguity_set("a"))
            .with_system(empty.exclusive_system().label("2").in_ambiguity_set("a"))
            .with_system(empty.exclusive_system().label("3").in_ambiguity_set("a"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.exclusive_at_start);
        assert_eq!(ambiguities.len(), 0);
    }

    #[test]
    #[should_panic]
    fn multiple_worlds_same_stage() {
        let mut world_a = World::default();
        let mut world_b = World::default();
        let mut stage = SystemStage::parallel();
        stage.run(&mut world_a);
        stage.run(&mut world_b);
    }

    #[test]
    fn archetype_update_single_executor() {
        fn query_count_system(
            mut entity_count: ResMut<usize>,
            query: Query<crate::entity::Entity>,
        ) {
            *entity_count = query.iter().count();
        }

        let mut world = World::new();
        world.insert_resource(0_usize);
        let mut stage = SystemStage::single(query_count_system.system());

        let entity = world.spawn().insert_bundle(()).id();
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);

        world.get_entity_mut(entity).unwrap().insert(1);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);
    }

    #[test]
    fn archetype_update_parallel_executor() {
        fn query_count_system(
            mut entity_count: ResMut<usize>,
            query: Query<crate::entity::Entity>,
        ) {
            *entity_count = query.iter().count();
        }

        let mut world = World::new();
        world.insert_resource(0_usize);
        let mut stage = SystemStage::parallel();
        stage.add_system(query_count_system.system());

        let entity = world.spawn().insert_bundle(()).id();
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);

        world.get_entity_mut(entity).unwrap().insert(1);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);
    }
}
