use crate::{
    component::ComponentId,
    prelude::IntoSystem,
    schedule::{
        graph_utils::{self, DependencyGraphError},
        BoxedRunCriteria, BoxedRunCriteriaLabel, BoxedSystemLabel, DuplicateLabelStrategy,
        ExclusiveSystemContainer, GraphNode, InsertionPoint, ParallelExecutor,
        ParallelSystemContainer, ParallelSystemExecutor, RunCriteriaContainer,
        RunCriteriaDescriptor, RunCriteriaDescriptorOrLabel, RunCriteriaInner, ShouldRun,
        SingleThreadedExecutor, SystemContainer, SystemDescriptor, SystemSet,
    },
    world::{World, WorldId},
};
use bevy_utils::{tracing::info, HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;
use std::fmt::Debug;

use super::IntoSystemDescriptor;

/// A type that can run as a step of a [`Schedule`](super::Schedule).
pub trait Stage: Downcast + Send + Sync {
    /// Runs the stage; this happens once per update.
    /// Implementors must initialize all of their state and systems before running the first time.
    fn run(&mut self, world: &mut World);
}

impl_downcast!(Stage);

/// When this resource is present in the `App`'s `Resources`,
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

/// Stores and executes systems. Execution order is not defined unless explicitly specified;
/// see `SystemDescriptor` documentation.
pub struct SystemStage {
    /// The WorldId this stage was last run on.
    world_id: Option<WorldId>,
    /// Instance of a scheduling algorithm for running the systems.
    executor: Box<dyn ParallelSystemExecutor>,
    /// Determines whether the stage should run.
    stage_run_criteria: BoxedRunCriteria,
    /// Topologically sorted run criteria of systems.
    run_criteria: Vec<RunCriteriaContainer>,
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
    /// Newly inserted run criteria that will be initialized at the next opportunity.
    uninitialized_run_criteria: Vec<(usize, DuplicateLabelStrategy)>,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_at_start: Vec<usize>,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_before_commands: Vec<usize>,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_at_end: Vec<usize>,
    /// Newly inserted systems that will be initialized at the next opportunity.
    uninitialized_parallel: Vec<usize>,
    /// Saves the value of the World change_tick during the last tick check
    last_tick_check: u32,
    /// If true, buffers will be automatically applied at the end of the stage. If false, buffers must be manually applied.
    apply_buffers: bool,
}

impl SystemStage {
    pub fn new(executor: Box<dyn ParallelSystemExecutor>) -> Self {
        SystemStage {
            world_id: None,
            executor,
            stage_run_criteria: Default::default(),
            run_criteria: vec![],
            uninitialized_run_criteria: vec![],
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
            last_tick_check: Default::default(),
            apply_buffers: true,
        }
    }

    pub fn single<Params>(system: impl IntoSystemDescriptor<Params>) -> Self {
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

    pub fn with_system<Params>(mut self, system: impl IntoSystemDescriptor<Params>) -> Self {
        self.add_system(system);
        self
    }

    pub fn add_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_inner(system.into_descriptor(), None);
        self
    }

    fn add_system_inner(&mut self, system: SystemDescriptor, default_run_criteria: Option<usize>) {
        self.systems_modified = true;
        match system {
            SystemDescriptor::Exclusive(mut descriptor) => {
                let insertion_point = descriptor.insertion_point;
                let criteria = descriptor.run_criteria.take();
                let mut container = ExclusiveSystemContainer::from_descriptor(descriptor);
                match criteria {
                    Some(RunCriteriaDescriptorOrLabel::Label(label)) => {
                        container.run_criteria_label = Some(label);
                    }
                    Some(RunCriteriaDescriptorOrLabel::Descriptor(criteria_descriptor)) => {
                        container.run_criteria_label = criteria_descriptor.label.clone();
                        container.run_criteria_index =
                            Some(self.add_run_criteria_internal(criteria_descriptor));
                    }
                    None => {
                        container.run_criteria_index = default_run_criteria;
                    }
                }
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
            SystemDescriptor::Parallel(mut descriptor) => {
                let criteria = descriptor.run_criteria.take();
                let mut container = ParallelSystemContainer::from_descriptor(descriptor);
                match criteria {
                    Some(RunCriteriaDescriptorOrLabel::Label(label)) => {
                        container.run_criteria_label = Some(label);
                    }
                    Some(RunCriteriaDescriptorOrLabel::Descriptor(criteria_descriptor)) => {
                        container.run_criteria_label = criteria_descriptor.label.clone();
                        container.run_criteria_index =
                            Some(self.add_run_criteria_internal(criteria_descriptor));
                    }
                    None => {
                        container.run_criteria_index = default_run_criteria;
                    }
                }
                self.uninitialized_parallel.push(self.parallel.len());
                self.parallel.push(container);
            }
        }
    }

    pub fn apply_buffers(&mut self, world: &mut World) {
        for container in self.parallel.iter_mut() {
            let system = container.system_mut();
            #[cfg(feature = "trace")]
            let span = bevy_utils::tracing::info_span!("system_commands", name = &*system.name());
            #[cfg(feature = "trace")]
            let _guard = span.enter();
            system.apply_buffers(world);
        }
    }

    pub fn set_apply_buffers(&mut self, apply_buffers: bool) {
        self.apply_buffers = apply_buffers;
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

    pub fn with_system_set(mut self, system_set: impl Into<SystemSet>) -> Self {
        self.add_system_set(system_set);
        self
    }

    pub fn add_system_set(&mut self, system_set: impl Into<SystemSet>) -> &mut Self {
        let system_set = system_set.into();
        self.systems_modified = true;
        let (run_criteria, mut systems) = system_set.bake();
        let set_run_criteria_index = run_criteria.and_then(|criteria| {
            // validate that no systems have criteria
            for system in systems.iter_mut() {
                if let Some(name) = match system {
                    SystemDescriptor::Exclusive(descriptor) => descriptor
                        .run_criteria
                        .is_some()
                        .then(|| descriptor.system.name()),
                    SystemDescriptor::Parallel(descriptor) => descriptor
                        .run_criteria
                        .is_some()
                        .then(|| descriptor.system.name()),
                } {
                    panic!(
                        "The system {} has a run criteria, but its `SystemSet` also has a run \
                        criteria. This is not supported. Consider moving the system into a \
                        different `SystemSet` or calling `add_system()` instead.",
                        name
                    )
                }
            }
            match criteria {
                RunCriteriaDescriptorOrLabel::Descriptor(descriptor) => {
                    Some(self.add_run_criteria_internal(descriptor))
                }
                RunCriteriaDescriptorOrLabel::Label(label) => {
                    for system in systems.iter_mut() {
                        match system {
                            SystemDescriptor::Exclusive(descriptor) => {
                                descriptor.run_criteria =
                                    Some(RunCriteriaDescriptorOrLabel::Label(label.clone()))
                            }
                            SystemDescriptor::Parallel(descriptor) => {
                                descriptor.run_criteria =
                                    Some(RunCriteriaDescriptorOrLabel::Label(label.clone()))
                            }
                        }
                    }

                    None
                }
            }
        });
        for system in systems.drain(..) {
            self.add_system_inner(system, set_run_criteria_index);
        }
        self
    }

    pub fn with_run_criteria<Param, S: IntoSystem<(), ShouldRun, Param>>(
        mut self,
        system: S,
    ) -> Self {
        self.set_run_criteria(system.system());
        self
    }

    pub fn set_run_criteria<Param, S: IntoSystem<(), ShouldRun, Param>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.stage_run_criteria.set(Box::new(system.system()));
        self
    }

    pub fn with_system_run_criteria(mut self, run_criteria: RunCriteriaDescriptor) -> Self {
        self.add_system_run_criteria(run_criteria);
        self
    }

    pub fn add_system_run_criteria(&mut self, run_criteria: RunCriteriaDescriptor) -> &mut Self {
        self.add_run_criteria_internal(run_criteria);
        self
    }

    pub(crate) fn add_run_criteria_internal(&mut self, descriptor: RunCriteriaDescriptor) -> usize {
        let index = self.run_criteria.len();
        self.uninitialized_run_criteria
            .push((index, descriptor.duplicate_label_strategy));

        self.run_criteria
            .push(RunCriteriaContainer::from_descriptor(descriptor));
        index
    }

    fn initialize_systems(&mut self, world: &mut World) {
        let mut criteria_labels = HashMap::default();
        let uninitialized_criteria: HashMap<_, _> =
            self.uninitialized_run_criteria.drain(..).collect();
        // track the number of filtered criteria to correct run criteria indices
        let mut filtered_criteria = 0;
        let mut new_indices = Vec::new();
        self.run_criteria = self
            .run_criteria
            .drain(..)
            .enumerate()
            .filter_map(|(index, mut container)| {
                let new_index = index - filtered_criteria;
                let label = container.label.clone();
                if let Some(strategy) = uninitialized_criteria.get(&index) {
                    if let Some(ref label) = label {
                        if let Some(duplicate_index) = criteria_labels.get(label) {
                            match strategy {
                                DuplicateLabelStrategy::Panic => panic!(
                                    "Run criteria {} is labelled with {:?}, which \
                            is already in use. Consider using \
                            `RunCriteriaDescriptorCoercion::label_discard_if_duplicate().",
                                    container.name(),
                                    container.label
                                ),
                                DuplicateLabelStrategy::Discard => {
                                    new_indices.push(*duplicate_index);
                                    filtered_criteria += 1;
                                    return None;
                                }
                            }
                        }
                    }
                    container.initialize(world);
                }
                if let Some(label) = label {
                    criteria_labels.insert(label, new_index);
                }
                new_indices.push(new_index);
                Some(container)
            })
            .collect();

        for index in self.uninitialized_at_start.drain(..) {
            let container = &mut self.exclusive_at_start[index];
            if let Some(index) = container.run_criteria() {
                container.set_run_criteria(new_indices[index]);
            }
            container.system_mut().initialize(world);
        }
        for index in self.uninitialized_before_commands.drain(..) {
            let container = &mut self.exclusive_before_commands[index];
            if let Some(index) = container.run_criteria() {
                container.set_run_criteria(new_indices[index]);
            }
            container.system_mut().initialize(world);
        }
        for index in self.uninitialized_at_end.drain(..) {
            let container = &mut self.exclusive_at_end[index];
            if let Some(index) = container.run_criteria() {
                container.set_run_criteria(new_indices[index]);
            }
            container.system_mut().initialize(world);
        }
        for index in self.uninitialized_parallel.drain(..) {
            let container = &mut self.parallel[index];
            if let Some(index) = container.run_criteria() {
                container.set_run_criteria(new_indices[index]);
            }
            container.system_mut().initialize(world);
        }
    }

    /// Rearranges all systems in topological orders. Systems must be initialized.
    fn rebuild_orders_and_dependencies(&mut self) {
        // This assertion is there to document that a maximum of `u32::MAX / 8` systems should be
        // added to a stage to guarantee that change detection has no false positive, but it
        // can be circumvented using exclusive or chained systems
        assert!(
            self.exclusive_at_start.len()
                + self.exclusive_before_commands.len()
                + self.exclusive_at_end.len()
                + self.parallel.len()
                < (u32::MAX / 8) as usize
        );
        debug_assert!(
            self.uninitialized_run_criteria.is_empty()
                && self.uninitialized_parallel.is_empty()
                && self.uninitialized_at_start.is_empty()
                && self.uninitialized_before_commands.is_empty()
                && self.uninitialized_at_end.is_empty()
        );
        fn unwrap_dependency_cycle_error<Node: GraphNode, Output, Labels: Debug>(
            result: Result<Output, DependencyGraphError<Labels>>,
            nodes: &[Node],
            nodes_description: &'static str,
        ) -> Output {
            match result {
                Ok(output) => output,
                Err(DependencyGraphError::GraphCycles(cycle)) => {
                    use std::fmt::Write;
                    let mut message = format!("Found a dependency cycle in {}:", nodes_description);
                    writeln!(message).unwrap();
                    for (index, labels) in &cycle {
                        writeln!(message, " - {}", nodes[*index].name()).unwrap();
                        writeln!(
                            message,
                            "    wants to be after (because of labels: {:?})",
                            labels,
                        )
                        .unwrap();
                    }
                    writeln!(message, " - {}", cycle[0].0).unwrap();
                    panic!("{}", message);
                }
            }
        }
        let run_criteria_labels = unwrap_dependency_cycle_error(
            self.process_run_criteria(),
            &self.run_criteria,
            "run criteria",
        );
        unwrap_dependency_cycle_error(
            process_systems(&mut self.parallel, &run_criteria_labels),
            &self.parallel,
            "parallel systems",
        );
        unwrap_dependency_cycle_error(
            process_systems(&mut self.exclusive_at_start, &run_criteria_labels),
            &self.exclusive_at_start,
            "exclusive systems at start of stage",
        );
        unwrap_dependency_cycle_error(
            process_systems(&mut self.exclusive_before_commands, &run_criteria_labels),
            &self.exclusive_before_commands,
            "exclusive systems before commands of stage",
        );
        unwrap_dependency_cycle_error(
            process_systems(&mut self.exclusive_at_end, &run_criteria_labels),
            &self.exclusive_at_end,
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
                    add an explicit dependency relation between some of these systems:\n"
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

    /// Checks for old component and system change ticks
    fn check_change_ticks(&mut self, world: &mut World) {
        let change_tick = world.change_tick();
        let time_since_last_check = change_tick.wrapping_sub(self.last_tick_check);
        // Only check after at least `u32::MAX / 8` counts, and at most `u32::MAX / 4` counts
        // since the max number of [System] in a [SystemStage] is limited to `u32::MAX / 8`
        // and this function is called at the end of each [SystemStage] loop
        const MIN_TIME_SINCE_LAST_CHECK: u32 = u32::MAX / 8;

        if time_since_last_check > MIN_TIME_SINCE_LAST_CHECK {
            // Check all system change ticks
            for exclusive_system in &mut self.exclusive_at_start {
                exclusive_system.system_mut().check_change_tick(change_tick);
            }
            for exclusive_system in &mut self.exclusive_before_commands {
                exclusive_system.system_mut().check_change_tick(change_tick);
            }
            for exclusive_system in &mut self.exclusive_at_end {
                exclusive_system.system_mut().check_change_tick(change_tick);
            }
            for parallel_system in &mut self.parallel {
                parallel_system.system_mut().check_change_tick(change_tick);
            }

            // Check component ticks
            world.check_change_ticks();

            self.last_tick_check = change_tick;
        }
    }

    /// Sorts run criteria and populates resolved input-criteria for piping.
    /// Returns a map of run criteria labels to their indices.
    fn process_run_criteria(
        &mut self,
    ) -> Result<
        HashMap<BoxedRunCriteriaLabel, usize>,
        DependencyGraphError<HashSet<BoxedRunCriteriaLabel>>,
    > {
        let graph = graph_utils::build_dependency_graph(&self.run_criteria);
        let order = graph_utils::topological_order(&graph)?;
        let mut order_inverted = order.iter().enumerate().collect::<Vec<_>>();
        order_inverted.sort_unstable_by_key(|(_, &key)| key);
        let labels: HashMap<_, _> = self
            .run_criteria
            .iter()
            .enumerate()
            .filter_map(|(index, criteria)| {
                criteria
                    .label
                    .as_ref()
                    .map(|label| (label.clone(), order_inverted[index].0))
            })
            .collect();
        for criteria in self.run_criteria.iter_mut() {
            if let RunCriteriaInner::Piped { input: parent, .. } = &mut criteria.inner {
                let label = &criteria.after[0];
                *parent = *labels.get(label).unwrap_or_else(|| {
                    panic!(
                        "Couldn't find run criteria labelled {:?} to pipe from.",
                        label
                    )
                });
            }
        }

        fn update_run_criteria_indices<T: SystemContainer>(
            systems: &mut [T],
            order_inverted: &[(usize, &usize)],
        ) {
            for system in systems {
                if let Some(index) = system.run_criteria() {
                    system.set_run_criteria(order_inverted[index].0);
                }
            }
        }

        update_run_criteria_indices(&mut self.exclusive_at_end, &order_inverted);
        update_run_criteria_indices(&mut self.exclusive_at_start, &order_inverted);
        update_run_criteria_indices(&mut self.exclusive_before_commands, &order_inverted);
        update_run_criteria_indices(&mut self.parallel, &order_inverted);

        let mut temp = self.run_criteria.drain(..).map(Some).collect::<Vec<_>>();
        for index in order {
            self.run_criteria.push(temp[index].take().unwrap());
        }
        Ok(labels)
    }
}

/// Sorts given system containers topologically, populates their resolved dependencies
/// and run criteria.
fn process_systems(
    systems: &mut Vec<impl SystemContainer>,
    run_criteria_labels: &HashMap<BoxedRunCriteriaLabel, usize>,
) -> Result<(), DependencyGraphError<HashSet<BoxedSystemLabel>>> {
    let mut graph = graph_utils::build_dependency_graph(systems);
    let order = graph_utils::topological_order(&graph)?;
    let mut order_inverted = order.iter().enumerate().collect::<Vec<_>>();
    order_inverted.sort_unstable_by_key(|(_, &key)| key);
    for (index, container) in systems.iter_mut().enumerate() {
        if let Some(index) = container.run_criteria_label().map(|label| {
            *run_criteria_labels
                .get(label)
                .unwrap_or_else(|| panic!("No run criteria with label {:?} found.", label))
        }) {
            container.set_run_criteria(index);
        }
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

        let mut run_stage_loop = true;
        while run_stage_loop {
            let should_run = self.stage_run_criteria.should_run(world);
            match should_run {
                ShouldRun::No => return,
                ShouldRun::NoAndCheckAgain => continue,
                ShouldRun::YesAndCheckAgain => (),
                ShouldRun::Yes => {
                    run_stage_loop = false;
                }
            };

            // Evaluate system run criteria.
            for index in 0..self.run_criteria.len() {
                let (run_criteria, tail) = self.run_criteria.split_at_mut(index);
                let mut criteria = &mut tail[0];
                criteria.update_archetypes(world);
                match &mut criteria.inner {
                    RunCriteriaInner::Single(system) => criteria.should_run = system.run((), world),
                    RunCriteriaInner::Piped {
                        input: parent,
                        system,
                        ..
                    } => criteria.should_run = system.run(run_criteria[*parent].should_run, world),
                }
            }

            let mut run_system_loop = true;
            let mut default_should_run = ShouldRun::Yes;
            while run_system_loop {
                run_system_loop = false;

                fn should_run(
                    container: &impl SystemContainer,
                    run_criteria: &[RunCriteriaContainer],
                    default: ShouldRun,
                ) -> bool {
                    matches!(
                        container
                            .run_criteria()
                            .map(|index| run_criteria[index].should_run)
                            .unwrap_or(default),
                        ShouldRun::Yes | ShouldRun::YesAndCheckAgain
                    )
                }

                // Run systems that want to be at the start of stage.
                for container in &mut self.exclusive_at_start {
                    if should_run(container, &self.run_criteria, default_should_run) {
                        #[cfg(feature = "trace")]
                        let system_span = bevy_utils::tracing::info_span!(
                            "exclusive_system",
                            name = &*container.name()
                        );
                        #[cfg(feature = "trace")]
                        let _guard = system_span.enter();
                        container.system_mut().run(world);
                    }
                }

                // Run parallel systems using the executor.
                // TODO: hard dependencies, nested sets, whatever... should be evaluated here.
                for container in &mut self.parallel {
                    container.should_run =
                        should_run(container, &self.run_criteria, default_should_run);
                }
                self.executor.run_systems(&mut self.parallel, world);

                // Run systems that want to be between parallel systems and their command buffers.
                for container in &mut self.exclusive_before_commands {
                    if should_run(container, &self.run_criteria, default_should_run) {
                        #[cfg(feature = "trace")]
                        let system_span = bevy_utils::tracing::info_span!(
                            "exclusive_system",
                            name = &*container.name()
                        );
                        #[cfg(feature = "trace")]
                        let _guard = system_span.enter();
                        container.system_mut().run(world);
                    }
                }

                // Apply parallel systems' buffers.
                if self.apply_buffers {
                    for container in &mut self.parallel {
                        if container.should_run {
                            #[cfg(feature = "trace")]
                            let span = bevy_utils::tracing::info_span!(
                                "system_commands",
                                name = &*container.name()
                            );
                            #[cfg(feature = "trace")]
                            let _guard = span.enter();
                            container.system_mut().apply_buffers(world);
                        }
                    }
                }

                // Run systems that want to be at the end of stage.
                for container in &mut self.exclusive_at_end {
                    if should_run(container, &self.run_criteria, default_should_run) {
                        #[cfg(feature = "trace")]
                        let system_span = bevy_utils::tracing::info_span!(
                            "exclusive_system",
                            name = &*container.name()
                        );
                        #[cfg(feature = "trace")]
                        let _guard = system_span.enter();
                        container.system_mut().run(world);
                    }
                }

                // Check for old component and system change ticks
                self.check_change_ticks(world);

                // Evaluate run criteria.
                let run_criteria = &mut self.run_criteria;
                for index in 0..run_criteria.len() {
                    let (run_criteria, tail) = run_criteria.split_at_mut(index);
                    let criteria = &mut tail[0];
                    criteria.update_archetypes(world);
                    match criteria.should_run {
                        ShouldRun::No => (),
                        ShouldRun::Yes => criteria.should_run = ShouldRun::No,
                        ShouldRun::YesAndCheckAgain | ShouldRun::NoAndCheckAgain => {
                            match &mut criteria.inner {
                                RunCriteriaInner::Single(system) => {
                                    criteria.should_run = system.run((), world)
                                }
                                RunCriteriaInner::Piped {
                                    input: parent,
                                    system,
                                    ..
                                } => {
                                    criteria.should_run =
                                        system.run(run_criteria[*parent].should_run, world)
                                }
                            }
                            match criteria.should_run {
                                ShouldRun::Yes => {
                                    run_system_loop = true;
                                }
                                ShouldRun::YesAndCheckAgain | ShouldRun::NoAndCheckAgain => {
                                    run_system_loop = true;
                                }
                                ShouldRun::No => (),
                            }
                        }
                    }
                }

                // after the first loop, default to not running systems without run criteria
                default_should_run = ShouldRun::No;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entity,
        query::{ChangeTrackers, Changed},
        schedule::{
            BoxedSystemLabel, ExclusiveSystemDescriptorCoercion, ParallelSystemDescriptorCoercion,
            RunCriteria, RunCriteriaDescriptorCoercion, RunCriteriaPiping, ShouldRun,
            SingleThreadedExecutor, Stage, SystemSet, SystemStage,
        },
        system::{In, IntoExclusiveSystem, IntoSystem, Local, Query, ResMut},
        world::World,
    };

    use crate as bevy_ecs;
    use crate::component::Component;
    #[derive(Component)]
    struct W<T>(T);

    fn make_exclusive(tag: usize) -> impl FnMut(&mut World) {
        move |world| world.get_resource_mut::<Vec<usize>>().unwrap().push(tag)
    }

    fn make_parallel(tag: usize) -> impl FnMut(ResMut<Vec<usize>>) {
        move |mut resource: ResMut<Vec<usize>>| resource.push(tag)
    }

    fn every_other_time(mut has_ran: Local<bool>) -> ShouldRun {
        *has_ran = !*has_ran;
        if *has_ran {
            ShouldRun::Yes
        } else {
            ShouldRun::No
        }
    }

    #[test]
    fn insertion_points() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().at_start())
            .with_system(make_parallel(1))
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
            .with_system(make_parallel(1))
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
            .with_system(make_parallel(2).exclusive_system().before_commands())
            .with_system(make_parallel(3).exclusive_system().at_end())
            .with_system(make_parallel(1))
            .with_system(make_parallel(0).exclusive_system().at_start());
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
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).exclusive_system().before("1"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(every_other_time)
                    .with_system(make_exclusive(1).exclusive_system().label("1")),
            )
            .with_system(make_exclusive(2).exclusive_system().after("1"));
        stage.run(&mut world);
        stage.run(&mut world);
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
            .with_system(make_parallel(1).after("0").label("1"))
            .with_system(make_parallel(2).after("1"))
            .with_system(make_parallel(0).label("0"));
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
            .with_system(make_parallel(1).label("1").before("2"))
            .with_system(make_parallel(2).label("2"))
            .with_system(make_parallel(0).before("1"));
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
            .with_system(make_parallel(2).label("2"))
            .with_system(make_parallel(1).after("0").before("2"))
            .with_system(make_parallel(0).label("0"))
            .with_system(make_parallel(4).label("4"))
            .with_system(make_parallel(3).after("2").before("4"));
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
            .with_system(make_parallel(1).label("first").after("0"))
            .with_system(make_parallel(2).after("first"))
            .with_system(make_parallel(0).label("first").label("0"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 1, 2]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).after("01").label("2"))
            .with_system(make_parallel(1).label("01").after("0"))
            .with_system(make_parallel(0).label("01").label("0"))
            .with_system(make_parallel(4).label("4"))
            .with_system(make_parallel(3).after("2").before("4"));
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).label("234").label("2"))
            .with_system(make_parallel(1).before("234").after("0"))
            .with_system(make_parallel(0).label("0"))
            .with_system(make_parallel(4).label("234").label("4"))
            .with_system(make_parallel(3).label("234").after("2").before("4"));
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
                make_parallel(2)
                    .label("2")
                    .after("1")
                    .before("3")
                    .before("3"),
            )
            .with_system(
                make_parallel(1)
                    .label("1")
                    .after("0")
                    .after("0")
                    .before("2"),
            )
            .with_system(make_parallel(0).label("0").before("1"))
            .with_system(make_parallel(4).label("4").after("3"))
            .with_system(make_parallel(3).label("3").after("2").before("4"));
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
            .with_system(make_parallel(2).label("2"))
            .with_system_set(
                SystemSet::new()
                    .with_system(make_parallel(0).label("0"))
                    .with_system(make_parallel(4).label("4"))
                    .with_system(make_parallel(3).after("2").before("4")),
            )
            .with_system(make_parallel(1).after("0").before("2"));
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
        let mut stage = SystemStage::parallel()
            .with_system(
                make_parallel(0)
                    .label("0")
                    .with_run_criteria(every_other_time),
            )
            .with_system(make_parallel(1).after("0"));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 1, 0, 1, 1]
        );

        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).before("1"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(every_other_time)
                    .with_system(make_parallel(1).label("1")),
            )
            .with_system(make_parallel(2).after("1"));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 0, 2, 0, 1, 2, 0, 2]
        );

        // Reusing criteria.
        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_system_run_criteria(every_other_time.label("every other time"))
            .with_system(make_parallel(0).before("1"))
            .with_system(
                make_parallel(1)
                    .label("1")
                    .with_run_criteria("every other time"),
            )
            .with_system(
                make_parallel(2)
                    .label("2")
                    .after("1")
                    .with_run_criteria("every other time"),
            )
            .with_system(make_parallel(3).after("2"));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 3]
        );
        assert_eq!(stage.run_criteria.len(), 1);

        // Piping criteria.
        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        fn eot_piped(input: In<ShouldRun>, has_ran: Local<bool>) -> ShouldRun {
            if let ShouldRun::Yes | ShouldRun::YesAndCheckAgain = input.0 {
                every_other_time(has_ran)
            } else {
                ShouldRun::No
            }
        }
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).label("0"))
            .with_system(
                make_parallel(1)
                    .label("1")
                    .after("0")
                    .with_run_criteria(every_other_time.label("every other time")),
            )
            .with_system(
                make_parallel(2)
                    .label("2")
                    .after("1")
                    .with_run_criteria(RunCriteria::pipe("every other time", eot_piped.system())),
            )
            .with_system(
                make_parallel(3)
                    .label("3")
                    .after("2")
                    .with_run_criteria("every other time".pipe(eot_piped.system()).label("piped")),
            )
            .with_system(make_parallel(4).after("3").with_run_criteria("piped"));
        for _ in 0..4 {
            stage.run(&mut world);
        }
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        for _ in 0..5 {
            stage.run(&mut world);
        }
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 4, 0, 0, 1, 0, 0, 1, 2, 3, 4, 0, 0, 1, 0, 0, 1, 2, 3, 4]
        );
        assert_eq!(stage.run_criteria.len(), 3);

        // Discarding extra criteria with matching labels.
        world.get_resource_mut::<Vec<usize>>().unwrap().clear();
        let mut stage =
            SystemStage::parallel()
                .with_system(make_parallel(0).before("1"))
                .with_system(make_parallel(1).label("1").with_run_criteria(
                    every_other_time.label_discard_if_duplicate("every other time"),
                ))
                .with_system(make_parallel(2).label("2").after("1").with_run_criteria(
                    every_other_time.label_discard_if_duplicate("every other time"),
                ))
                .with_system(make_parallel(3).after("2"));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            *world.get_resource::<Vec<usize>>().unwrap(),
            vec![0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 3]
        );
        assert_eq!(stage.run_criteria.len(), 1);
    }

    #[test]
    #[should_panic]
    fn duplicate_run_criteria_label_panic() {
        let mut world = World::new();
        let mut stage = SystemStage::parallel()
            .with_system_run_criteria(every_other_time.label("every other time"))
            .with_system_run_criteria(every_other_time.label("every other time"));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_1() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel().with_system(make_parallel(0).label("0").after("0"));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_2() {
        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).label("0").after("1"))
            .with_system(make_parallel(1).label("1").after("0"));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_3() {
        let mut world = World::new();

        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).label("0"))
            .with_system(make_parallel(1).after("0").before("2"))
            .with_system(make_parallel(2).label("2").before("0"));
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
        fn component(_: Query<&mut W<f32>>) {}

        let mut world = World::new();

        let mut stage = SystemStage::parallel()
            .with_system(empty.label("0"))
            .with_system(empty.label("1").after("0"))
            .with_system(empty.label("2"))
            .with_system(empty.label("3").after("2").before("4"))
            .with_system(empty.label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(find_ambiguities(&stage.parallel).len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(empty.label("0"))
            .with_system(component.label("1").after("0"))
            .with_system(empty.label("2"))
            .with_system(empty.label("3").after("2").before("4"))
            .with_system(component.label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.label("0"))
            .with_system(resource.label("1").after("0"))
            .with_system(empty.label("2"))
            .with_system(empty.label("3").after("2").before("4"))
            .with_system(resource.label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("4")))
                || ambiguities.contains(&(Box::new("4"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(empty.label("0"))
            .with_system(resource.label("1").after("0"))
            .with_system(empty.label("2"))
            .with_system(empty.label("3").after("2").before("4"))
            .with_system(component.label("4"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        assert_eq!(find_ambiguities(&stage.parallel).len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(component.label("0"))
            .with_system(resource.label("1").after("0"))
            .with_system(empty.label("2"))
            .with_system(component.label("3").after("2").before("4"))
            .with_system(resource.label("4"));
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
            .with_system(component.label("0"))
            .with_system(resource.label("1").after("0").in_ambiguity_set("a"))
            .with_system(empty.label("2"))
            .with_system(component.label("3").after("2").before("4"))
            .with_system(resource.label("4").in_ambiguity_set("a"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("0"), Box::new("3")))
                || ambiguities.contains(&(Box::new("3"), Box::new("0")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.label("0").before("2"))
            .with_system(component.label("1").before("2"))
            .with_system(component.label("2"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("0"), Box::new("1")))
                || ambiguities.contains(&(Box::new("1"), Box::new("0")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.label("0"))
            .with_system(component.label("1").after("0"))
            .with_system(component.label("2").after("0"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("2")))
                || ambiguities.contains(&(Box::new("2"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.label("0").before("1").before("2"))
            .with_system(component.label("1"))
            .with_system(component.label("2"))
            .with_system(component.label("3").after("1").after("2"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert!(
            ambiguities.contains(&(Box::new("1"), Box::new("2")))
                || ambiguities.contains(&(Box::new("2"), Box::new("1")))
        );
        assert_eq!(ambiguities.len(), 1);

        let mut stage = SystemStage::parallel()
            .with_system(component.label("0").before("1").before("2"))
            .with_system(component.label("1").in_ambiguity_set("a"))
            .with_system(component.label("2").in_ambiguity_set("a"))
            .with_system(component.label("3").after("1").after("2"));
        stage.initialize_systems(&mut world);
        stage.rebuild_orders_and_dependencies();
        let ambiguities = find_ambiguities_first_labels(&stage.parallel);
        assert_eq!(ambiguities.len(), 0);

        let mut stage = SystemStage::parallel()
            .with_system(component.label("0").before("1").before("2"))
            .with_system(component.label("1").in_ambiguity_set("a"))
            .with_system(component.label("2").in_ambiguity_set("b"))
            .with_system(component.label("3").after("1").after("2"));
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
                    .label("0")
                    .before("1")
                    .before("2")
                    .before("3")
                    .before("4"),
            )
            .with_system(component.label("1"))
            .with_system(component.label("2"))
            .with_system(component.label("3"))
            .with_system(component.label("4"))
            .with_system(
                component
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
                    .label("0")
                    .before("1")
                    .before("2")
                    .before("3")
                    .before("4"),
            )
            .with_system(component.label("1").in_ambiguity_set("a"))
            .with_system(component.label("2").in_ambiguity_set("a"))
            .with_system(component.label("3").in_ambiguity_set("a"))
            .with_system(component.label("4").in_ambiguity_set("a"))
            .with_system(
                component
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
                    .label("0")
                    .before("1")
                    .before("2")
                    .before("3")
                    .before("4"),
            )
            .with_system(component.label("1").in_ambiguity_set("a"))
            .with_system(component.label("2").in_ambiguity_set("a"))
            .with_system(
                component
                    .label("3")
                    .in_ambiguity_set("a")
                    .in_ambiguity_set("b"),
            )
            .with_system(component.label("4").in_ambiguity_set("b"))
            .with_system(
                component
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
        let mut stage = SystemStage::single(query_count_system);

        let entity = world.spawn().insert_bundle(()).id();
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);

        world.get_entity_mut(entity).unwrap().insert(W(1));
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
        stage.add_system(query_count_system);

        let entity = world.spawn().insert_bundle(()).id();
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);

        world.get_entity_mut(entity).unwrap().insert(W(1));
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);
    }

    #[test]
    fn change_ticks_wrapover() {
        const MIN_TIME_SINCE_LAST_CHECK: u32 = u32::MAX / 8;
        const MAX_DELTA: u32 = (u32::MAX / 4) * 3;

        let mut world = World::new();
        world.spawn().insert(W(0usize));
        *world.change_tick.get_mut() += MAX_DELTA + 1;

        let mut stage = SystemStage::parallel();
        fn work() {}
        stage.add_system(work);

        // Overflow twice
        for _ in 0..10 {
            stage.run(&mut world);
            for tracker in world.query::<ChangeTrackers<W<usize>>>().iter(&world) {
                let time_since_last_check = tracker
                    .change_tick
                    .wrapping_sub(tracker.component_ticks.added);
                assert!(time_since_last_check <= MAX_DELTA);
                let time_since_last_check = tracker
                    .change_tick
                    .wrapping_sub(tracker.component_ticks.changed);
                assert!(time_since_last_check <= MAX_DELTA);
            }
            let change_tick = world.change_tick.get_mut();
            *change_tick = change_tick.wrapping_add(MIN_TIME_SINCE_LAST_CHECK + 1);
        }
    }

    #[test]
    fn change_query_wrapover() {
        use crate::{self as bevy_ecs, component::Component};

        #[derive(Component)]
        struct C;
        let mut world = World::new();

        // Spawn entities at various ticks
        let component_ticks = [0, u32::MAX / 4, u32::MAX / 2, u32::MAX / 4 * 3, u32::MAX];
        let ids = component_ticks
            .iter()
            .map(|tick| {
                *world.change_tick.get_mut() = *tick;
                world.spawn().insert(C).id()
            })
            .collect::<Vec<Entity>>();

        let test_cases = [
            // normal
            (0, u32::MAX / 2, vec![ids[1], ids[2]]),
            // just wrapped over
            (u32::MAX / 2, 0, vec![ids[0], ids[3], ids[4]]),
        ];
        for (last_change_tick, change_tick, changed_entities) in test_cases.iter() {
            *world.change_tick.get_mut() = *change_tick;
            world.last_change_tick = *last_change_tick;

            assert_eq!(
                world
                    .query_filtered::<Entity, Changed<C>>()
                    .iter(&world)
                    .collect::<Vec<Entity>>(),
                *changed_entities
            );
        }
    }

    #[test]
    fn run_criteria_with_query() {
        use crate::{self as bevy_ecs, component::Component};

        #[derive(Component)]
        struct Foo;

        fn even_number_of_entities_critiera(query: Query<&Foo>) -> ShouldRun {
            if query.iter().len() % 2 == 0 {
                ShouldRun::Yes
            } else {
                ShouldRun::No
            }
        }

        fn spawn_entity(mut commands: crate::prelude::Commands) {
            commands.spawn().insert(Foo);
        }

        fn count_entities(query: Query<&Foo>, mut res: ResMut<Vec<usize>>) {
            res.push(query.iter().len());
        }

        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(spawn_entity.label("spawn"))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(even_number_of_entities_critiera)
                    .with_system(count_entities.before("spawn")),
            );
        stage.run(&mut world);
        stage.run(&mut world);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<Vec<usize>>().unwrap(), vec![0, 2]);
    }

    #[test]
    fn stage_run_criteria_with_query() {
        use crate::{self as bevy_ecs, component::Component};

        #[derive(Component)]
        struct Foo;

        fn even_number_of_entities_critiera(query: Query<&Foo>) -> ShouldRun {
            if query.iter().len() % 2 == 0 {
                ShouldRun::Yes
            } else {
                ShouldRun::No
            }
        }

        fn spawn_entity(mut commands: crate::prelude::Commands) {
            commands.spawn().insert(Foo);
        }

        fn count_entities(query: Query<&Foo>, mut res: ResMut<Vec<usize>>) {
            res.push(query.iter().len());
        }

        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage_spawn = SystemStage::parallel().with_system(spawn_entity);
        let mut stage_count = SystemStage::parallel()
            .with_run_criteria(even_number_of_entities_critiera)
            .with_system(count_entities);
        stage_count.run(&mut world);
        stage_spawn.run(&mut world);
        stage_count.run(&mut world);
        stage_spawn.run(&mut world);
        stage_count.run(&mut world);
        stage_spawn.run(&mut world);
        stage_count.run(&mut world);
        stage_spawn.run(&mut world);
        assert_eq!(*world.get_resource::<Vec<usize>>().unwrap(), vec![0, 2]);
    }
}
