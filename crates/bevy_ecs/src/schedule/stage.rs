use crate::{
    self as bevy_ecs,
    change_detection::CHECK_TICK_THRESHOLD,
    component::ComponentId,
    prelude::IntoSystem,
    schedule::{
        graph_utils::{self, DependencyGraphError},
        BoxedRunCriteria, DuplicateLabelStrategy, ExclusiveInsertionPoint, GraphNode,
        ParallelExecutor, ParallelSystemExecutor, RunCriteriaContainer, RunCriteriaDescriptor,
        RunCriteriaDescriptorOrLabel, RunCriteriaInner, RunCriteriaLabelId, ShouldRun,
        SingleThreadedExecutor, SystemContainer, SystemDescriptor, SystemLabelId, SystemSet,
    },
    world::{World, WorldId},
};
use bevy_ecs_macros::Resource;
use bevy_utils::{tracing::warn, HashMap, HashSet};
use core::fmt::Debug;
use downcast_rs::{impl_downcast, Downcast};

use super::{IntoSystemDescriptor, Schedule};

/// A type that can run as a step of a [`Schedule`](super::Schedule).
pub trait Stage: Downcast + Send + Sync {
    /// Runs the stage; this happens once per update.
    /// Implementors must initialize all of their state and systems before running the first time.
    fn run(&mut self, world: &mut World);
}

impl Debug for dyn Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(as_systemstage) = self.as_any().downcast_ref::<SystemStage>() {
            write!(f, "{as_systemstage:?}")
        } else if let Some(as_schedule) = self.as_any().downcast_ref::<Schedule>() {
            write!(f, "{as_schedule:?}")
        } else {
            write!(f, "Unknown dyn Stage")
        }
    }
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
#[derive(Resource, Default)]
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
    pub(super) exclusive_at_start: Vec<SystemContainer>,
    /// Topologically sorted exclusive systems that want to be run after parallel systems but
    /// before the application of their command buffers.
    pub(super) exclusive_before_commands: Vec<SystemContainer>,
    /// Topologically sorted exclusive systems that want to be run at the end of the stage.
    pub(super) exclusive_at_end: Vec<SystemContainer>,
    /// Topologically sorted parallel systems.
    pub(super) parallel: Vec<SystemContainer>,
    /// Determines if the stage was modified and needs to rebuild its graphs and orders.
    pub(super) systems_modified: bool,
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
    must_read_resource: Option<ComponentId>,
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
            must_read_resource: None,
        }
    }

    pub fn single<Params>(system: impl IntoSystemDescriptor<Params>) -> Self {
        Self::single_threaded().with_system(system)
    }

    pub fn single_threaded() -> Self {
        Self::new(Box::<SingleThreadedExecutor>::default())
    }

    pub fn parallel() -> Self {
        Self::new(Box::<ParallelExecutor>::default())
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

    pub fn set_must_read_resource(&mut self, resource_id: ComponentId) {
        self.must_read_resource = Some(resource_id);
    }

    #[must_use]
    pub fn with_system<Params>(mut self, system: impl IntoSystemDescriptor<Params>) -> Self {
        self.add_system(system);
        self
    }

    pub fn add_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_inner(system.into_descriptor(), None);
        self
    }

    fn add_system_inner(
        &mut self,
        mut descriptor: SystemDescriptor,
        default_run_criteria: Option<usize>,
    ) {
        self.systems_modified = true;
        if let Some(insertion_point) = descriptor.exclusive_insertion_point {
            let criteria = descriptor.run_criteria.take();
            let mut container = SystemContainer::from_descriptor(descriptor);
            match criteria {
                Some(RunCriteriaDescriptorOrLabel::Label(label)) => {
                    container.run_criteria_label = Some(label);
                }
                Some(RunCriteriaDescriptorOrLabel::Descriptor(criteria_descriptor)) => {
                    container.run_criteria_label = criteria_descriptor.label;
                    container.run_criteria_index =
                        Some(self.add_run_criteria_internal(criteria_descriptor));
                }
                None => {
                    container.run_criteria_index = default_run_criteria;
                }
            }
            match insertion_point {
                ExclusiveInsertionPoint::AtStart => {
                    let index = self.exclusive_at_start.len();
                    self.uninitialized_at_start.push(index);
                    self.exclusive_at_start.push(container);
                }
                ExclusiveInsertionPoint::BeforeCommands => {
                    let index = self.exclusive_before_commands.len();
                    self.uninitialized_before_commands.push(index);
                    self.exclusive_before_commands.push(container);
                }
                ExclusiveInsertionPoint::AtEnd => {
                    let index = self.exclusive_at_end.len();
                    self.uninitialized_at_end.push(index);
                    self.exclusive_at_end.push(container);
                }
            }
        } else {
            let criteria = descriptor.run_criteria.take();
            let mut container = SystemContainer::from_descriptor(descriptor);
            match criteria {
                Some(RunCriteriaDescriptorOrLabel::Label(label)) => {
                    container.run_criteria_label = Some(label);
                }
                Some(RunCriteriaDescriptorOrLabel::Descriptor(criteria_descriptor)) => {
                    container.run_criteria_label = criteria_descriptor.label;
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

    pub fn apply_buffers(&mut self, world: &mut World) {
        for container in &mut self.parallel {
            let system = container.system_mut();
            #[cfg(feature = "trace")]
            let _span = bevy_utils::tracing::info_span!("system_commands", name = &*system.name())
                .entered();
            system.apply_buffers(world);
        }
    }

    pub fn set_apply_buffers(&mut self, apply_buffers: bool) {
        self.apply_buffers = apply_buffers;
    }

    /// Topologically sorted parallel systems.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn parallel_systems(&self) -> &[SystemContainer] {
        &self.parallel
    }

    /// Topologically sorted exclusive systems that want to be run at the start of the stage.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn exclusive_at_start_systems(&self) -> &[SystemContainer] {
        &self.exclusive_at_start
    }

    /// Topologically sorted exclusive systems that want to be run at the end of the stage.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn exclusive_at_end_systems(&self) -> &[SystemContainer] {
        &self.exclusive_at_end
    }

    /// Topologically sorted exclusive systems that want to be run after parallel systems but
    /// before the application of their command buffers.
    ///
    /// Note that systems won't be fully-formed until the stage has been run at least once.
    pub fn exclusive_before_commands_systems(&self) -> &[SystemContainer] {
        &self.exclusive_before_commands
    }

    #[must_use]
    pub fn with_system_set(mut self, system_set: SystemSet) -> Self {
        self.add_system_set(system_set);
        self
    }

    pub fn add_system_set(&mut self, system_set: SystemSet) -> &mut Self {
        self.systems_modified = true;
        let (run_criteria, mut systems) = system_set.bake();
        let set_run_criteria_index = run_criteria.and_then(|criteria| {
            // validate that no systems have criteria
            for descriptor in &mut systems {
                if let Some(name) = descriptor
                    .run_criteria
                    .is_some()
                    .then(|| descriptor.system.name())
                {
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
                    for system in &mut systems {
                        system.run_criteria = Some(RunCriteriaDescriptorOrLabel::Label(label));
                    }

                    None
                }
            }
        });
        for system in systems {
            self.add_system_inner(system, set_run_criteria_index);
        }
        self
    }

    #[must_use]
    pub fn with_run_criteria<Param, S: IntoSystem<(), ShouldRun, Param>>(
        mut self,
        system: S,
    ) -> Self {
        self.set_run_criteria(system);
        self
    }

    pub fn set_run_criteria<Param, S: IntoSystem<(), ShouldRun, Param>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.stage_run_criteria
            .set(Box::new(IntoSystem::into_system(system)));
        self
    }

    #[must_use]
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
                let label = container.label;
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
        // This assertion exists to document that the number of systems in a stage is limited
        // to guarantee that change detection never yields false positives. However, it's possible
        // (but still unlikely) to circumvent this by abusing exclusive or chained systems.
        assert!(
            self.exclusive_at_start.len()
                + self.exclusive_before_commands.len()
                + self.exclusive_at_end.len()
                + self.parallel.len()
                < (CHECK_TICK_THRESHOLD as usize)
        );
        debug_assert!(
            self.uninitialized_run_criteria.is_empty()
                && self.uninitialized_parallel.is_empty()
                && self.uninitialized_at_start.is_empty()
                && self.uninitialized_before_commands.is_empty()
                && self.uninitialized_at_end.is_empty()
        );
        fn unwrap_dependency_cycle_error<Node: GraphNode, Output, Labels: std::fmt::Debug>(
            result: Result<Output, DependencyGraphError<Labels>>,
            nodes: &[Node],
            nodes_description: &'static str,
        ) -> Output {
            match result {
                Ok(output) => output,
                Err(DependencyGraphError::GraphCycles(cycle)) => {
                    use std::fmt::Write;
                    let mut message = format!("Found a dependency cycle in {nodes_description}:");
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

    fn check_uses_resource(&self, resource_id: ComponentId, world: &World) {
        debug_assert!(!self.systems_modified);
        for system in &self.parallel {
            if !system.component_access().has_read(resource_id) {
                let component_name = world.components().get_info(resource_id).unwrap().name();
                warn!(
                    "System {} doesn't access resource {component_name}, despite being required to",
                    system.name()
                );
            }
        }
    }

    /// All system and component change ticks are scanned once the world counter has incremented
    /// at least [`CHECK_TICK_THRESHOLD`](crate::change_detection::CHECK_TICK_THRESHOLD)
    /// times since the previous `check_tick` scan.
    ///
    /// During each scan, any change ticks older than [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE)
    /// are clamped to that age. This prevents false positives from appearing due to overflow.
    fn check_change_ticks(&mut self, world: &mut World) {
        let change_tick = world.change_tick();
        let ticks_since_last_check = change_tick.wrapping_sub(self.last_tick_check);

        if ticks_since_last_check >= CHECK_TICK_THRESHOLD {
            // Check all system change ticks.
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

            // Check all component change ticks.
            world.check_change_ticks();
            self.last_tick_check = change_tick;
        }
    }

    /// Sorts run criteria and populates resolved input-criteria for piping.
    /// Returns a map of run criteria labels to their indices.
    fn process_run_criteria(
        &mut self,
    ) -> Result<HashMap<RunCriteriaLabelId, usize>, DependencyGraphError<HashSet<RunCriteriaLabelId>>>
    {
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
                    .map(|&label| (label, order_inverted[index].0))
            })
            .collect();
        for criteria in &mut self.run_criteria {
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

        fn update_run_criteria_indices(
            systems: &mut [SystemContainer],
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

    pub fn vec_system_container_debug(
        &self,
        name: &str,
        v: &Vec<SystemContainer>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{name}: ")?;
        if v.len() > 1 {
            writeln!(f, "[")?;
            for sc in v.iter() {
                writeln!(f, "{sc:?},")?;
            }
            write!(f, "], ")
        } else {
            write!(f, "{v:?}, ")
        }
    }
}

impl std::fmt::Debug for SystemStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SystemStage: {{ ")?;
        write!(
            f,
            "world_id: {:?}, executor: {:?}, stage_run_criteria: {:?}, run_criteria: {:?}, ",
            self.world_id, self.executor, self.stage_run_criteria, self.run_criteria
        )?;
        self.vec_system_container_debug("exclusive_at_start", &self.exclusive_at_start, f)?;
        self.vec_system_container_debug(
            "exclusive_before_commands",
            &self.exclusive_before_commands,
            f,
        )?;
        self.vec_system_container_debug("exclusive_at_end", &self.exclusive_at_end, f)?;
        self.vec_system_container_debug("parallel", &self.parallel, f)?;
        write!(
            f,
            "systems_modified: {:?}, uninitialized_run_criteria: {:?}, ",
            self.systems_modified, self.uninitialized_run_criteria
        )?;
        write!(
            f,
            "uninitialized_at_start: {:?}, uninitialized_before_commands: {:?}, ",
            self.uninitialized_at_start, self.uninitialized_before_commands
        )?;
        write!(
            f,
            "uninitialized_at_end: {:?}, uninitialized_parallel: {:?}, ",
            self.uninitialized_at_end, self.uninitialized_parallel
        )?;
        write!(
            f,
            "last_tick_check: {:?}, apply_buffers: {:?}, ",
            self.last_tick_check, self.apply_buffers
        )?;
        write!(f, "must_read_resource: {:?}}}", self.must_read_resource)
    }
}

/// Sorts given system containers topologically, populates their resolved dependencies
/// and run criteria.
fn process_systems(
    systems: &mut Vec<SystemContainer>,
    run_criteria_labels: &HashMap<RunCriteriaLabelId, usize>,
) -> Result<(), DependencyGraphError<HashSet<SystemLabelId>>> {
    let mut graph = graph_utils::build_dependency_graph(systems);
    let order = graph_utils::topological_order(&graph)?;
    let mut order_inverted = order.iter().enumerate().collect::<Vec<_>>();
    order_inverted.sort_unstable_by_key(|(_, &key)| key);
    for (index, container) in systems.iter_mut().enumerate() {
        if let Some(index) = container.run_criteria_label().map(|label| {
            *run_criteria_labels
                .get(label)
                .unwrap_or_else(|| panic!("No run criteria with label {label:?} found."))
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
            if let Some(resource_id) = self.must_read_resource {
                self.check_uses_resource(resource_id, world);
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

                #[cfg(feature = "trace")]
                let _span =
                    bevy_utils::tracing::info_span!("run criteria", name = &*criteria.name())
                        .entered();

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
                    container: &SystemContainer,
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
                        {
                            #[cfg(feature = "trace")]
                            let _system_span = bevy_utils::tracing::info_span!(
                                "exclusive_system",
                                name = &*container.name()
                            )
                            .entered();
                            container.system_mut().run((), world);
                        }
                        {
                            #[cfg(feature = "trace")]
                            let _system_span = bevy_utils::tracing::info_span!(
                                "system_commands",
                                name = &*container.name()
                            )
                            .entered();
                            container.system_mut().apply_buffers(world);
                        }
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
                        {
                            #[cfg(feature = "trace")]
                            let _system_span = bevy_utils::tracing::info_span!(
                                "exclusive_system",
                                name = &*container.name()
                            )
                            .entered();
                            container.system_mut().run((), world);
                        }
                        {
                            #[cfg(feature = "trace")]
                            let _system_span = bevy_utils::tracing::info_span!(
                                "system_commands",
                                name = &*container.name()
                            )
                            .entered();
                            container.system_mut().apply_buffers(world);
                        }
                    }
                }

                // Apply parallel systems' buffers.
                if self.apply_buffers {
                    for container in &mut self.parallel {
                        if container.should_run {
                            #[cfg(feature = "trace")]
                            let _span = bevy_utils::tracing::info_span!(
                                "system_commands",
                                name = &*container.name()
                            )
                            .entered();
                            container.system_mut().apply_buffers(world);
                        }
                    }
                }

                // Run systems that want to be at the end of stage.
                for container in &mut self.exclusive_at_end {
                    if should_run(container, &self.run_criteria, default_should_run) {
                        {
                            #[cfg(feature = "trace")]
                            let _system_span = bevy_utils::tracing::info_span!(
                                "exclusive_system",
                                name = &*container.name()
                            )
                            .entered();
                            container.system_mut().run((), world);
                        }
                        {
                            #[cfg(feature = "trace")]
                            let _system_span = bevy_utils::tracing::info_span!(
                                "system_commands",
                                name = &*container.name()
                            )
                            .entered();
                            container.system_mut().apply_buffers(world);
                        }
                    }
                }

                // Check for old component and system change ticks
                self.check_change_ticks(world);

                // Evaluate run criteria.
                let run_criteria = &mut self.run_criteria;
                for index in 0..run_criteria.len() {
                    let (run_criteria, tail) = run_criteria.split_at_mut(index);
                    let criteria = &mut tail[0];
                    match criteria.should_run {
                        ShouldRun::No => (),
                        ShouldRun::Yes => criteria.should_run = ShouldRun::No,
                        ShouldRun::YesAndCheckAgain | ShouldRun::NoAndCheckAgain => {
                            match &mut criteria.inner {
                                RunCriteriaInner::Single(system) => {
                                    criteria.should_run = system.run((), world);
                                }
                                RunCriteriaInner::Piped {
                                    input: parent,
                                    system,
                                    ..
                                } => {
                                    criteria.should_run =
                                        system.run(run_criteria[*parent].should_run, world);
                                }
                            }
                            match criteria.should_run {
                                ShouldRun::Yes
                                | ShouldRun::YesAndCheckAgain
                                | ShouldRun::NoAndCheckAgain => {
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
    use bevy_ecs_macros::RunCriteriaLabel;

    use crate::{
        schedule::{
            IntoSystemDescriptor, RunCriteria, RunCriteriaDescriptorCoercion, ShouldRun,
            SingleThreadedExecutor, Stage, SystemLabel, SystemSet, SystemStage,
        },
        system::{In, Local, Query, ResMut},
        world::World,
    };

    use crate as bevy_ecs;
    use crate::component::Component;
    use crate::system::Resource;

    #[derive(Component)]
    struct W<T>(T);
    #[derive(Resource)]
    struct R(usize);

    #[derive(Resource, Default)]
    struct EntityCount(Vec<usize>);

    fn make_exclusive(tag: usize) -> impl FnMut(&mut World) {
        move |world| world.resource_mut::<EntityCount>().0.push(tag)
    }

    fn make_parallel(tag: usize) -> impl FnMut(ResMut<EntityCount>) {
        move |mut resource: ResMut<EntityCount>| resource.0.push(tag)
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
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).at_start())
            .with_system(make_parallel(1))
            .with_system(make_exclusive(2).before_commands())
            .with_system(make_exclusive(3).at_end());
        stage.run(&mut world);
        assert_eq!(world.resource_mut::<EntityCount>().0, vec![0, 1, 2, 3]);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );

        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).before_commands())
            .with_system(make_exclusive(3).at_end())
            .with_system(make_parallel(1))
            .with_system(make_exclusive(0).at_start());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 3]);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );

        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).before_commands())
            .with_system(make_parallel(3).at_end())
            .with_system(make_parallel(1))
            .with_system(make_parallel(0).at_start());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 3]);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 0, 1, 2, 3]
        );
    }

    #[derive(SystemLabel)]
    enum TestLabels {
        L0,
        L1,
        L2,
        L3,
        L4,
        First,
        L01,
        L234,
    }
    use TestLabels::*;

    #[test]
    fn exclusive_after() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(1).label(L1).after(L0))
            .with_system(make_exclusive(2).after(L1))
            .with_system(make_exclusive(0).label(L0));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn exclusive_before() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(1).label(L1).before(L2))
            .with_system(make_exclusive(2).label(L2))
            .with_system(make_exclusive(0).before(L1));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn exclusive_mixed() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).label(L2))
            .with_system(make_exclusive(1).after(L0).before(L2))
            .with_system(make_exclusive(0).label(L0))
            .with_system(make_exclusive(4).label(L4))
            .with_system(make_exclusive(3).after(L2).before(L4));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_multiple_labels() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(1).label(First).after(L0))
            .with_system(make_exclusive(2).after(First))
            .with_system(make_exclusive(0).label(First).label(L0));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 0, 1, 2]);

        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).after(L01).label(L2))
            .with_system(make_exclusive(1).label(L01).after(L0))
            .with_system(make_exclusive(0).label(L01).label(L0))
            .with_system(make_exclusive(4).label(L4))
            .with_system(make_exclusive(3).after(L2).before(L4));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );

        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).label(L234).label(L2))
            .with_system(make_exclusive(1).before(L234).after(L0))
            .with_system(make_exclusive(0).label(L0))
            .with_system(make_exclusive(4).label(L234).label(L4))
            .with_system(make_exclusive(3).label(L234).after(L2).before(L4));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_redundant_constraints() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).label(L2).after(L1).before(L3).before(L3))
            .with_system(make_exclusive(1).label(L1).after(L0).after(L0).before(L2))
            .with_system(make_exclusive(0).label(L0).before(L1))
            .with_system(make_exclusive(4).label(L4).after(L3))
            .with_system(make_exclusive(3).label(L3).after(L2).before(L4));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_mixed_across_sets() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(2).label(L2))
            .with_system_set(
                SystemSet::new()
                    .with_system(make_exclusive(0).label(L0))
                    .with_system(make_exclusive(4).label(L4))
                    .with_system(make_exclusive(3).after(L2).before(L4)),
            )
            .with_system(make_exclusive(1).after(L0).before(L2));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn exclusive_run_criteria() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).before(L1))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(every_other_time)
                    .with_system(make_exclusive(1).label(L1)),
            )
            .with_system(make_exclusive(2).after(L1));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 0, 2, 0, 1, 2, 0, 2]
        );
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_1() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel().with_system(make_exclusive(0).label(L0).after(L0));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_2() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).label(L0).after(L1))
            .with_system(make_exclusive(1).label(L1).after(L0));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn exclusive_cycle_3() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_exclusive(0).label(L0))
            .with_system(make_exclusive(1).after(L0).before(L2))
            .with_system(make_exclusive(2).label(L2).before(L0));
        stage.run(&mut world);
    }

    #[test]
    fn parallel_after() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(1).after(L0).label(L1))
            .with_system(make_parallel(2).after(L1))
            .with_system(make_parallel(0).label(L0));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn parallel_before() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(1).label(L1).before(L2))
            .with_system(make_parallel(2).label(L2))
            .with_system(make_parallel(0).before(L1));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn parallel_mixed() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).label(L2))
            .with_system(make_parallel(1).after(L0).before(L2))
            .with_system(make_parallel(0).label(L0))
            .with_system(make_parallel(4).label(L4))
            .with_system(make_parallel(3).after(L2).before(L4));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_multiple_labels() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(1).label(First).after(L0))
            .with_system(make_parallel(2).after(First))
            .with_system(make_parallel(0).label(First).label(L0));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 2, 0, 1, 2]);

        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).after(L01).label(L2))
            .with_system(make_parallel(1).label(L01).after(L0))
            .with_system(make_parallel(0).label(L01).label(L0))
            .with_system(make_parallel(4).label(L4))
            .with_system(make_parallel(3).after(L2).before(L4));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );

        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).label(L234).label(L2))
            .with_system(make_parallel(1).before(L234).after(L0))
            .with_system(make_parallel(0).label(L0))
            .with_system(make_parallel(4).label(L234).label(L4))
            .with_system(make_parallel(3).label(L234).after(L2).before(L4));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_redundant_constraints() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).label(L2).after(L1).before(L3).before(L3))
            .with_system(make_parallel(1).label(L1).after(L0).after(L0).before(L2))
            .with_system(make_parallel(0).label(L0).before(L1))
            .with_system(make_parallel(4).label(L4).after(L3))
            .with_system(make_parallel(3).label(L3).after(L2).before(L4));
        stage.run(&mut world);
        for container in &stage.parallel {
            assert!(container.dependencies().len() <= 1);
        }
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn parallel_mixed_across_sets() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(2).label(L2))
            .with_system_set(
                SystemSet::new()
                    .with_system(make_parallel(0).label(L0))
                    .with_system(make_parallel(4).label(L4))
                    .with_system(make_parallel(3).after(L2).before(L4)),
            )
            .with_system(make_parallel(1).after(L0).before(L2));
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]
        );
    }

    #[derive(RunCriteriaLabel)]
    struct EveryOtherTime;

    #[test]
    fn parallel_run_criteria() {
        let mut world = World::new();

        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(
                make_parallel(0)
                    .label(L0)
                    .with_run_criteria(every_other_time),
            )
            .with_system(make_parallel(1).after(L0));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 1, 1, 0, 1, 1]);

        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).before(L1))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(every_other_time)
                    .with_system(make_parallel(1).label(L1)),
            )
            .with_system(make_parallel(2).after(L1));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 0, 2, 0, 1, 2, 0, 2]
        );

        // Reusing criteria.
        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system_run_criteria(every_other_time.label(EveryOtherTime))
            .with_system(make_parallel(0).before(L1))
            .with_system(make_parallel(1).label(L1).with_run_criteria(EveryOtherTime))
            .with_system(
                make_parallel(2)
                    .label(L2)
                    .after(L1)
                    .with_run_criteria(EveryOtherTime),
            )
            .with_system(make_parallel(3).after(L2));
        stage.run(&mut world);
        stage.run(&mut world);
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 3]
        );
        assert_eq!(stage.run_criteria.len(), 1);

        // Piping criteria.
        world.resource_mut::<EntityCount>().0.clear();
        fn eot_piped(input: In<ShouldRun>, has_ran: Local<bool>) -> ShouldRun {
            if let ShouldRun::Yes | ShouldRun::YesAndCheckAgain = input.0 {
                every_other_time(has_ran)
            } else {
                ShouldRun::No
            }
        }

        #[derive(RunCriteriaLabel)]
        struct Piped;

        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).label(L0))
            .with_system(
                make_parallel(1)
                    .label(L1)
                    .after(L0)
                    .with_run_criteria(every_other_time.label(EveryOtherTime)),
            )
            .with_system(
                make_parallel(2)
                    .label(L2)
                    .after(L1)
                    .with_run_criteria(RunCriteria::pipe(EveryOtherTime, eot_piped)),
            )
            .with_system(
                make_parallel(3)
                    .label(L3)
                    .after(L2)
                    .with_run_criteria(RunCriteria::pipe(EveryOtherTime, eot_piped).label(Piped)),
            )
            .with_system(make_parallel(4).after(L3).with_run_criteria(Piped));
        for _ in 0..4 {
            stage.run(&mut world);
        }
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        for _ in 0..5 {
            stage.run(&mut world);
        }
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 4, 0, 0, 1, 0, 0, 1, 2, 3, 4, 0, 0, 1, 0, 0, 1, 2, 3, 4]
        );
        assert_eq!(stage.run_criteria.len(), 3);

        // Discarding extra criteria with matching labels.
        world.resource_mut::<EntityCount>().0.clear();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).before(L1))
            .with_system(
                make_parallel(1)
                    .label(L1)
                    .with_run_criteria(every_other_time.label_discard_if_duplicate(EveryOtherTime)),
            )
            .with_system(
                make_parallel(2)
                    .label(L2)
                    .after(L1)
                    .with_run_criteria(every_other_time.label_discard_if_duplicate(EveryOtherTime)),
            )
            .with_system(make_parallel(3).after(L2));
        stage.run(&mut world);
        stage.run(&mut world);
        // false positive, `Box::default` cannot coerce `SingleThreadedExecutor` to `dyn ParallelSystemExectutor`
        stage.set_executor(Box::<SingleThreadedExecutor>::default());
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            world.resource::<EntityCount>().0,
            vec![0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 3]
        );
        assert_eq!(stage.run_criteria.len(), 1);
    }

    #[test]
    #[should_panic]
    fn duplicate_run_criteria_label_panic() {
        let mut world = World::new();
        let mut stage = SystemStage::parallel()
            .with_system_run_criteria(every_other_time.label(EveryOtherTime))
            .with_system_run_criteria(every_other_time.label(EveryOtherTime));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_1() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel().with_system(make_parallel(0).label(L0).after(L0));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_2() {
        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).label(L0).after(L1))
            .with_system(make_parallel(1).label(L1).after(L0));
        stage.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn parallel_cycle_3() {
        let mut world = World::new();

        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(make_parallel(0).label(L0))
            .with_system(make_parallel(1).after(L0).before(L2))
            .with_system(make_parallel(2).label(L2).before(L0));
        stage.run(&mut world);
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
        fn query_count_system(mut entity_count: ResMut<R>, query: Query<crate::entity::Entity>) {
            *entity_count = R(query.iter().count());
        }

        let mut world = World::new();
        world.insert_resource(R(0));
        let mut stage = SystemStage::single(query_count_system);

        let entity = world.spawn_empty().id();
        stage.run(&mut world);
        assert_eq!(world.resource::<R>().0, 1);

        world.get_entity_mut(entity).unwrap().insert(W(1));
        stage.run(&mut world);
        assert_eq!(world.resource::<R>().0, 1);
    }

    #[test]
    fn archetype_update_parallel_executor() {
        fn query_count_system(mut entity_count: ResMut<R>, query: Query<crate::entity::Entity>) {
            *entity_count = R(query.iter().count());
        }

        let mut world = World::new();
        world.insert_resource(R(0));
        let mut stage = SystemStage::parallel();
        stage.add_system(query_count_system);

        let entity = world.spawn_empty().id();
        stage.run(&mut world);
        assert_eq!(world.resource::<R>().0, 1);

        world.get_entity_mut(entity).unwrap().insert(W(1));
        stage.run(&mut world);
        assert_eq!(world.resource::<R>().0, 1);
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
            commands.spawn(Foo);
        }

        fn count_entities(query: Query<&Foo>, mut res: ResMut<EntityCount>) {
            res.0.push(query.iter().len());
        }

        #[derive(SystemLabel)]
        struct Spawn;

        let mut world = World::new();
        world.init_resource::<EntityCount>();
        let mut stage = SystemStage::parallel()
            .with_system(spawn_entity.label(Spawn))
            .with_system_set(
                SystemSet::new()
                    .with_run_criteria(even_number_of_entities_critiera)
                    .with_system(count_entities.before(Spawn)),
            );
        stage.run(&mut world);
        stage.run(&mut world);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 2]);
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
            commands.spawn(Foo);
        }

        fn count_entities(query: Query<&Foo>, mut res: ResMut<EntityCount>) {
            res.0.push(query.iter().len());
        }

        let mut world = World::new();
        world.init_resource::<EntityCount>();
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
        assert_eq!(world.resource::<EntityCount>().0, vec![0, 2]);
    }
}
