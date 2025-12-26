#![expect(
    clippy::module_inception,
    reason = "This instance of module inception is being discussed; see #17344."
)]

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use bevy_platform::{
    collections::{HashMap, HashSet},
    hash::NoOpHash,
};
use bevy_utils::{default, TypeIdMap};
use core::{
    any::{Any, TypeId},
    fmt::Debug,
};
use fixedbitset::FixedBitSet;
use indexmap::IndexMap;
use log::warn;
use pass::ScheduleBuildPassObj;
use thiserror::Error;
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::{change_detection::CheckChangeTicks, component::Component, system::System};
use crate::{resource::Resource, schedule::*, system::ScheduleSystem, world::World};

pub use stepping::Stepping;
use Direction::{Incoming, Outgoing};

/// Resource that stores [`Schedule`]s mapped to [`ScheduleLabel`]s excluding the current running [`Schedule`].
#[derive(Default, Resource)]
pub struct Schedules {
    inner: HashMap<InternedScheduleLabel, Schedule>,
}

impl Schedules {
    /// Constructs an empty `Schedules` with zero initial capacity.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a labeled schedule into the map.
    ///
    /// If the map already had an entry for `label`, `schedule` is inserted,
    /// and the old schedule is returned. Otherwise, `None` is returned.
    pub fn insert(&mut self, schedule: Schedule) -> Option<Schedule> {
        self.inner.insert(schedule.label, schedule)
    }

    /// Removes the schedule corresponding to the `label` from the map, returning it if it existed.
    pub fn remove(&mut self, label: impl ScheduleLabel) -> Option<Schedule> {
        self.inner.remove(&label.intern())
    }

    /// Removes the (schedule, label) pair corresponding to the `label` from the map, returning it if it existed.
    pub fn remove_entry(
        &mut self,
        label: impl ScheduleLabel,
    ) -> Option<(InternedScheduleLabel, Schedule)> {
        self.inner.remove_entry(&label.intern())
    }

    /// Does a schedule with the provided label already exist?
    pub fn contains(&self, label: impl ScheduleLabel) -> bool {
        self.inner.contains_key(&label.intern())
    }

    /// Returns a reference to the schedule associated with `label`, if it exists.
    pub fn get(&self, label: impl ScheduleLabel) -> Option<&Schedule> {
        self.inner.get(&label.intern())
    }

    /// Returns a mutable reference to the schedule associated with `label`, if it exists.
    pub fn get_mut(&mut self, label: impl ScheduleLabel) -> Option<&mut Schedule> {
        self.inner.get_mut(&label.intern())
    }

    /// Returns a mutable reference to the schedules associated with `label`, creating one if it doesn't already exist.
    pub fn entry(&mut self, label: impl ScheduleLabel) -> &mut Schedule {
        self.inner
            .entry(label.intern())
            .or_insert_with(|| Schedule::new(label))
    }

    /// Returns an iterator over all schedules. Iteration order is undefined.
    pub fn iter(&self) -> impl Iterator<Item = (&dyn ScheduleLabel, &Schedule)> {
        self.inner
            .iter()
            .map(|(label, schedule)| (&**label, schedule))
    }
    /// Returns an iterator over mutable references to all schedules. Iteration order is undefined.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&dyn ScheduleLabel, &mut Schedule)> {
        self.inner
            .iter_mut()
            .map(|(label, schedule)| (&**label, schedule))
    }

    /// Iterates the change ticks of all systems in all stored schedules and clamps any older than
    /// [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        #[cfg(feature = "trace")]
        let _all_span = info_span!("check stored schedule ticks").entered();
        #[cfg_attr(
            not(feature = "trace"),
            expect(
                unused_variables,
                reason = "The `label` variable goes unused if the `trace` feature isn't active"
            )
        )]
        for (label, schedule) in &mut self.inner {
            #[cfg(feature = "trace")]
            let name = format!("{label:?}");
            #[cfg(feature = "trace")]
            let _one_span = info_span!("check schedule ticks", name = &name).entered();
            schedule.check_change_ticks(check);
        }
    }

    /// Applies the provided [`ScheduleBuildSettings`] to all schedules.
    pub fn configure_schedules(&mut self, schedule_build_settings: ScheduleBuildSettings) {
        for (_, schedule) in &mut self.inner {
            schedule.set_build_settings(schedule_build_settings.clone());
        }
    }

    /// Ignore system order ambiguities caused by conflicts on [`Component`]s of type `T`.
    #[deprecated(
        since = "0.18.0",
        note = "Use `World::allow_ambiguous_component` instead"
    )]
    pub fn allow_ambiguous_component<T: Component>(&mut self, world: &mut World) {
        world.allow_ambiguous_component::<T>();
    }

    /// Ignore system order ambiguities caused by conflicts on [`Resource`]s of type `T`.
    #[deprecated(
        since = "0.18.0",
        note = "Use `World::allow_ambiguous_resource` instead"
    )]
    pub fn allow_ambiguous_resource<T: Resource>(&mut self, world: &mut World) {
        world.allow_ambiguous_resource::<T>();
    }

    /// Adds one or more systems to the [`Schedule`] matching the provided [`ScheduleLabel`].
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.entry(schedule).add_systems(systems);

        self
    }

    /// Removes all systems in a [`SystemSet`]. This will cause the schedule to be rebuilt when
    /// the schedule is run again. A [`ScheduleError`] is returned if the schedule needs to be
    /// [`Schedule::initialize`]'d or the `set` is not found.
    pub fn remove_systems_in_set<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        set: impl IntoSystemSet<M>,
        world: &mut World,
        policy: ScheduleCleanupPolicy,
    ) -> Result<usize, ScheduleError> {
        self.get_mut(schedule)
            .ok_or(ScheduleError::ScheduleNotFound)?
            .remove_systems_in_set(set, world, policy)
    }

    /// Configures a collection of system sets in the provided schedule, adding any sets that do not exist.
    #[track_caller]
    pub fn configure_sets<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.entry(schedule).configure_sets(sets);

        self
    }

    /// Suppress warnings and errors that would result from systems in these sets having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    ///
    /// When possible, do this directly in the `.add_systems(Update, a.ambiguous_with(b))` call.
    /// However, sometimes two independent plugins `A` and `B` are reported as ambiguous, which you
    /// can only suppress as the consumer of both.
    #[track_caller]
    pub fn ignore_ambiguity<M1, M2, S1, S2>(
        &mut self,
        schedule: impl ScheduleLabel,
        a: S1,
        b: S2,
    ) -> &mut Self
    where
        S1: IntoSystemSet<M1>,
        S2: IntoSystemSet<M2>,
    {
        self.entry(schedule).ignore_ambiguity(a, b);

        self
    }
}

fn make_executor(kind: ExecutorKind) -> Box<dyn SystemExecutor> {
    match kind {
        ExecutorKind::SingleThreaded => Box::new(SingleThreadedExecutor::new()),
        #[cfg(feature = "std")]
        ExecutorKind::MultiThreaded => Box::new(MultiThreadedExecutor::new()),
    }
}

/// Chain systems into dependencies
#[derive(Default)]
pub enum Chain {
    /// Systems are independent. Nodes are allowed to run in any order.
    #[default]
    Unchained,
    /// Systems are chained. `before -> after` ordering constraints
    /// will be added between the successive elements.
    Chained(TypeIdMap<Box<dyn Any>>),
}

impl Chain {
    /// Specify that the systems must be chained.
    pub fn set_chained(&mut self) {
        if matches!(self, Chain::Unchained) {
            *self = Self::Chained(Default::default());
        };
    }
    /// Specify that the systems must be chained, and add the specified configuration for
    /// all dependencies created between these systems.
    pub fn set_chained_with_config<T: 'static>(&mut self, config: T) {
        self.set_chained();
        if let Chain::Chained(config_map) = self {
            config_map.insert(TypeId::of::<T>(), Box::new(config));
        } else {
            unreachable!()
        };
    }
}

/// A collection of systems, and the metadata and executor needed to run them
/// in a certain order under certain conditions.
///
/// # Schedule labels
///
/// Each schedule has a [`ScheduleLabel`] value. This value is used to uniquely identify the
/// schedule when added to a [`World`]’s [`Schedules`], and may be used to specify which schedule
/// a system should be added to.
///
/// # Example
///
/// Here is an example of a `Schedule` running a "Hello world" system:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// fn hello_world() { println!("Hello world!") }
///
/// fn main() {
///     let mut world = World::new();
///     let mut schedule = Schedule::default();
///     schedule.add_systems(hello_world);
///
///     schedule.run(&mut world);
/// }
/// ```
///
/// A schedule can also run several systems in an ordered way:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// fn system_one() { println!("System 1 works!") }
/// fn system_two() { println!("System 2 works!") }
/// fn system_three() { println!("System 3 works!") }
///
/// fn main() {
///     let mut world = World::new();
///     let mut schedule = Schedule::default();
///     schedule.add_systems((
///         system_two,
///         system_one.before(system_two),
///         system_three.after(system_two),
///     ));
///
///     schedule.run(&mut world);
/// }
/// ```
///
/// Schedules are often inserted into a [`World`] and identified by their [`ScheduleLabel`] only:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::schedule::ScheduleLabel;
///
/// // Declare a new schedule label.
/// #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
/// struct Update;
///
/// // This system shall be part of the schedule.
/// fn an_update_system() {
///     println!("Hello world!");
/// }
///
/// fn main() {
///     let mut world = World::new();
///
///     // Add a system to the schedule with that label (creating it automatically).
///     world.get_resource_or_init::<Schedules>().add_systems(Update, an_update_system);
///
///     // Run the schedule, and therefore run the system.
///     world.run_schedule(Update);
/// }
/// ```
pub struct Schedule {
    label: InternedScheduleLabel,
    graph: ScheduleGraph,
    executable: SystemSchedule,
    executor: Box<dyn SystemExecutor>,
    executor_initialized: bool,
    warnings: Vec<ScheduleBuildWarning>,
}

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct DefaultSchedule;

impl Default for Schedule {
    /// Creates a schedule with a default label. Only use in situations where
    /// you don't care about the [`ScheduleLabel`]. Inserting a default schedule
    /// into the world risks overwriting another schedule. For most situations
    /// you should use [`Schedule::new`].
    fn default() -> Self {
        Self::new(DefaultSchedule)
    }
}

impl Schedule {
    /// Constructs an empty `Schedule`.
    pub fn new(label: impl ScheduleLabel) -> Self {
        let mut this = Self {
            label: label.intern(),
            graph: ScheduleGraph::new(),
            executable: SystemSchedule::new(),
            executor: make_executor(ExecutorKind::default()),
            executor_initialized: false,
            warnings: Vec::new(),
        };
        // Call `set_build_settings` to add any default build passes
        this.set_build_settings(Default::default());
        this
    }

    /// Returns the [`InternedScheduleLabel`] for this `Schedule`,
    /// corresponding to the [`ScheduleLabel`] this schedule was created with.
    pub fn label(&self) -> InternedScheduleLabel {
        self.label
    }

    /// Add a collection of systems to the schedule.
    pub fn add_systems<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.graph.process_configs(systems.into_configs(), false);
        self
    }

    /// Removes all systems in a [`SystemSet`]. This will cause the schedule to be rebuilt when
    /// the schedule is run again. A [`ScheduleError`] is returned if the schedule needs to be
    /// [`Schedule::initialize`]'d or the `set` is not found.
    ///
    /// Note that this can remove all systems of a type if you pass
    /// the system to this function as systems implicitly create a set based
    /// on the system type.
    ///
    /// ## Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::schedule::ScheduleCleanupPolicy;
    /// #
    /// # fn my_system() {}
    /// #
    /// let mut schedule = Schedule::default();
    /// // add the system to the schedule
    /// schedule.add_systems(my_system);
    /// let mut world = World::default();
    ///
    /// // remove the system
    /// schedule.remove_systems_in_set(my_system, &mut world, ScheduleCleanupPolicy::RemoveSystemsOnly);
    /// ```
    pub fn remove_systems_in_set<M>(
        &mut self,
        set: impl IntoSystemSet<M>,
        world: &mut World,
        policy: ScheduleCleanupPolicy,
    ) -> Result<usize, ScheduleError> {
        if self.graph.changed {
            self.initialize(world)?;
        }
        self.graph.remove_systems_in_set(set, policy)
    }

    /// Suppress warnings and errors that would result from systems in these sets having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    #[track_caller]
    pub fn ignore_ambiguity<M1, M2, S1, S2>(&mut self, a: S1, b: S2) -> &mut Self
    where
        S1: IntoSystemSet<M1>,
        S2: IntoSystemSet<M2>,
    {
        let a = a.into_system_set();
        let b = b.into_system_set();

        let a_id = self.graph.system_sets.get_key_or_insert(a.intern());
        let b_id = self.graph.system_sets.get_key_or_insert(b.intern());

        self.graph
            .ambiguous_with
            .add_edge(NodeId::Set(a_id), NodeId::Set(b_id));

        self
    }

    /// Configures a collection of system sets in this schedule, adding them if they does not exist.
    #[track_caller]
    pub fn configure_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.graph.configure_sets(sets);
        self
    }

    /// Add a custom build pass to the schedule.
    pub fn add_build_pass<T: ScheduleBuildPass>(&mut self, pass: T) -> &mut Self {
        self.graph.passes.insert(TypeId::of::<T>(), Box::new(pass));
        self
    }

    /// Remove a custom build pass.
    pub fn remove_build_pass<T: ScheduleBuildPass>(&mut self) {
        self.graph.passes.shift_remove(&TypeId::of::<T>());
    }

    /// Changes miscellaneous build settings.
    ///
    /// If [`settings.auto_insert_apply_deferred`][ScheduleBuildSettings::auto_insert_apply_deferred]
    /// is `false`, this clears `*_ignore_deferred` edge settings configured so far.
    ///
    /// Generally this method should be used before adding systems or set configurations to the schedule,
    /// not after.
    pub fn set_build_settings(&mut self, settings: ScheduleBuildSettings) -> &mut Self {
        if settings.auto_insert_apply_deferred {
            if !self
                .graph
                .passes
                .contains_key(&TypeId::of::<passes::AutoInsertApplyDeferredPass>())
            {
                self.add_build_pass(passes::AutoInsertApplyDeferredPass::default());
            }
        } else {
            self.remove_build_pass::<passes::AutoInsertApplyDeferredPass>();
        }
        self.graph.settings = settings;
        self
    }

    /// Returns the schedule's current `ScheduleBuildSettings`.
    pub fn get_build_settings(&self) -> ScheduleBuildSettings {
        self.graph.settings.clone()
    }

    /// Returns the schedule's current execution strategy.
    pub fn get_executor_kind(&self) -> ExecutorKind {
        self.executor.kind()
    }

    /// Sets the schedule's execution strategy.
    pub fn set_executor_kind(&mut self, executor: ExecutorKind) -> &mut Self {
        if executor != self.executor.kind() {
            self.executor = make_executor(executor);
            self.executor_initialized = false;
        }
        self
    }

    /// Set whether the schedule applies deferred system buffers on final time or not. This is a catch-all
    /// in case a system uses commands but was not explicitly ordered before an instance of
    /// [`ApplyDeferred`]. By default this
    /// setting is true, but may be disabled if needed.
    pub fn set_apply_final_deferred(&mut self, apply_final_deferred: bool) -> &mut Self {
        self.executor.set_apply_final_deferred(apply_final_deferred);
        self
    }

    /// Runs all systems in this schedule on the `world`, using its current execution strategy.
    pub fn run(&mut self, world: &mut World) {
        #[cfg(feature = "trace")]
        let _span = info_span!("schedule", name = ?self.label).entered();

        world.check_change_ticks();
        self.initialize(world).unwrap_or_else(|e| {
            panic!(
                "Error when initializing schedule {:?}: {}",
                self.label,
                e.to_string(self.graph(), world)
            )
        });

        let error_handler = world.default_error_handler();

        #[cfg(not(feature = "bevy_debug_stepping"))]
        self.executor
            .run(&mut self.executable, world, None, error_handler);

        #[cfg(feature = "bevy_debug_stepping")]
        {
            let skip_systems = match world.get_resource_mut::<Stepping>() {
                None => None,
                Some(mut stepping) => stepping.skipped_systems(self),
            };

            self.executor.run(
                &mut self.executable,
                world,
                skip_systems.as_ref(),
                error_handler,
            );
        }
    }

    /// Initializes any newly-added systems and conditions, rebuilds the executable schedule,
    /// and re-initializes the executor.
    ///
    /// Moves all systems and run conditions out of the [`ScheduleGraph`].
    pub fn initialize(&mut self, world: &mut World) -> Result<(), ScheduleBuildError> {
        if self.graph.changed {
            self.graph.initialize(world);
            self.warnings = self
                .graph
                .update_schedule(world, &mut self.executable, self.label)?;
            self.graph.changed = false;
            self.executor_initialized = false;
        }

        if !self.executor_initialized {
            self.executor.init(&self.executable);
            self.executor_initialized = true;
        }

        Ok(())
    }

    /// Returns the [`ScheduleGraph`].
    pub fn graph(&self) -> &ScheduleGraph {
        &self.graph
    }

    /// Returns a mutable reference to the [`ScheduleGraph`].
    pub fn graph_mut(&mut self) -> &mut ScheduleGraph {
        &mut self.graph
    }

    /// Returns the [`SystemSchedule`].
    pub(crate) fn executable(&self) -> &SystemSchedule {
        &self.executable
    }

    /// Iterates the change ticks of all systems in the schedule and clamps any older than
    /// [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    pub fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        for system in &mut self.executable.systems {
            if !is_apply_deferred(system) {
                system.check_change_tick(check);
            }
        }

        for conditions in &mut self.executable.system_conditions {
            for condition in conditions {
                condition.check_change_tick(check);
            }
        }

        for conditions in &mut self.executable.set_conditions {
            for condition in conditions {
                condition.check_change_tick(check);
            }
        }
    }

    /// Directly applies any accumulated [`Deferred`](crate::system::Deferred) system parameters (like [`Commands`](crate::prelude::Commands)) to the `world`.
    ///
    /// Like always, deferred system parameters are applied in the "topological sort order" of the schedule graph.
    /// As a result, buffers from one system are only guaranteed to be applied before those of other systems
    /// if there is an explicit system ordering between the two systems.
    ///
    /// This is used in rendering to extract data from the main world, storing the data in system buffers,
    /// before applying their buffers in a different world.
    pub fn apply_deferred(&mut self, world: &mut World) {
        for SystemWithAccess { system, .. } in &mut self.executable.systems {
            system.apply_deferred(world);
        }
    }

    /// Returns an iterator over all systems in this schedule.
    ///
    /// Note: this method will return [`ScheduleNotInitialized`] if the
    /// schedule has never been initialized or run.
    pub fn systems(
        &self,
    ) -> Result<impl Iterator<Item = (SystemKey, &ScheduleSystem)> + Sized, ScheduleNotInitialized>
    {
        if !self.executor_initialized {
            return Err(ScheduleNotInitialized);
        }

        let iter = self
            .executable
            .system_ids
            .iter()
            .zip(&self.executable.systems)
            .map(|(&node_id, system)| (node_id, &system.system));

        Ok(iter)
    }

    /// Returns the number of systems in this schedule.
    pub fn systems_len(&self) -> usize {
        if !self.executor_initialized {
            self.graph.systems.len()
        } else {
            self.executable.systems.len()
        }
    }

    /// Returns warnings that were generated during the last call to
    /// [`Schedule::initialize`].
    pub fn warnings(&self) -> &[ScheduleBuildWarning] {
        &self.warnings
    }
}

/// Metadata for a [`Schedule`].
///
/// The order isn't optimized; calling `ScheduleGraph::build_schedule` will return a
/// `SystemSchedule` where the order is optimized for execution.
#[derive(Default)]
pub struct ScheduleGraph {
    /// Container of systems in the schedule.
    pub systems: Systems,
    /// Container of system sets in the schedule.
    pub system_sets: SystemSets,
    /// Directed acyclic graph of the hierarchy (which systems/sets are children of which sets)
    hierarchy: Dag<NodeId>,
    /// Directed acyclic graph of the dependency (which systems/sets have to run before which other systems/sets)
    dependency: Dag<NodeId>,
    /// Map of systems in each set
    set_systems: DagGroups<SystemSetKey, SystemKey>,
    ambiguous_with: UnGraph<NodeId>,
    /// Nodes that are allowed to have ambiguous ordering relationship with any other systems.
    pub ambiguous_with_all: HashSet<NodeId>,
    conflicting_systems: ConflictingSystems,
    anonymous_sets: usize,
    changed: bool,
    settings: ScheduleBuildSettings,
    passes: IndexMap<TypeId, Box<dyn ScheduleBuildPassObj>, NoOpHash>,
}

impl ScheduleGraph {
    /// Creates an empty [`ScheduleGraph`] with default settings.
    pub fn new() -> Self {
        Self {
            systems: Systems::default(),
            system_sets: SystemSets::default(),
            hierarchy: Dag::new(),
            dependency: Dag::new(),
            set_systems: DagGroups::default(),
            ambiguous_with: UnGraph::default(),
            ambiguous_with_all: HashSet::default(),
            conflicting_systems: ConflictingSystems::default(),
            anonymous_sets: 0,
            changed: false,
            settings: default(),
            passes: default(),
        }
    }

    /// Returns the [`Dag`] of the hierarchy.
    ///
    /// The hierarchy is a directed acyclic graph of the systems and sets,
    /// where an edge denotes that a system or set is the child of another set.
    pub fn hierarchy(&self) -> &Dag<NodeId> {
        &self.hierarchy
    }

    /// Returns the [`Dag`] of the dependencies in the schedule.
    ///
    /// Nodes in this graph are systems and sets, and edges denote that
    /// a system or set has to run before another system or set.
    pub fn dependency(&self) -> &Dag<NodeId> {
        &self.dependency
    }

    /// Returns the list of systems that conflict with each other, i.e. have ambiguities in their access.
    ///
    /// If the `Vec<ComponentId>` is empty, the systems conflict on [`World`] access.
    /// Must be called after [`ScheduleGraph::build_schedule`] to be non-empty.
    pub fn conflicting_systems(&self) -> &ConflictingSystems {
        &self.conflicting_systems
    }

    fn process_config<T: ProcessScheduleConfig + Schedulable>(
        &mut self,
        config: ScheduleConfig<T>,
        collect_nodes: bool,
    ) -> ProcessConfigsResult {
        ProcessConfigsResult {
            densely_chained: true,
            nodes: collect_nodes
                .then_some(T::process_config(self, config))
                .into_iter()
                .collect(),
        }
    }

    fn apply_collective_conditions<
        T: ProcessScheduleConfig + Schedulable<Metadata = GraphInfo, GroupMetadata = Chain>,
    >(
        &mut self,
        configs: &mut [ScheduleConfigs<T>],
        collective_conditions: Vec<BoxedCondition>,
    ) {
        if !collective_conditions.is_empty() {
            if let [config] = configs {
                for condition in collective_conditions {
                    config.run_if_dyn(condition);
                }
            } else {
                let set = self.create_anonymous_set();
                for config in configs.iter_mut() {
                    config.in_set_inner(set.intern());
                }
                let mut set_config = InternedSystemSet::into_config(set.intern());
                set_config.conditions.extend(collective_conditions);
                self.configure_set_inner(set_config);
            }
        }
    }

    /// Adds the config nodes to the graph.
    ///
    /// `collect_nodes` controls whether the `NodeId`s of the processed config nodes are stored in the returned [`ProcessConfigsResult`].
    /// `process_config` is the function which processes each individual config node and returns a corresponding `NodeId`.
    ///
    /// The fields on the returned [`ProcessConfigsResult`] are:
    /// - `nodes`: a vector of all node ids contained in the nested `ScheduleConfigs`
    /// - `densely_chained`: a boolean that is true if all nested nodes are linearly chained (with successive `after` orderings) in the order they are defined
    #[track_caller]
    fn process_configs<
        T: ProcessScheduleConfig + Schedulable<Metadata = GraphInfo, GroupMetadata = Chain>,
    >(
        &mut self,
        configs: ScheduleConfigs<T>,
        collect_nodes: bool,
    ) -> ProcessConfigsResult {
        match configs {
            ScheduleConfigs::ScheduleConfig(config) => self.process_config(config, collect_nodes),
            ScheduleConfigs::Configs {
                metadata,
                mut configs,
                collective_conditions,
            } => {
                self.apply_collective_conditions(&mut configs, collective_conditions);

                let is_chained = matches!(metadata, Chain::Chained(_));

                // Densely chained if
                // * chained and all configs in the chain are densely chained, or
                // * unchained with a single densely chained config
                let mut densely_chained = is_chained || configs.len() == 1;
                let mut configs = configs.into_iter();
                let mut nodes = Vec::new();

                let Some(first) = configs.next() else {
                    return ProcessConfigsResult {
                        nodes: Vec::new(),
                        densely_chained,
                    };
                };
                let mut previous_result = self.process_configs(first, collect_nodes || is_chained);
                densely_chained &= previous_result.densely_chained;

                for current in configs {
                    let current_result = self.process_configs(current, collect_nodes || is_chained);
                    densely_chained &= current_result.densely_chained;

                    if let Chain::Chained(chain_options) = &metadata {
                        // if the current result is densely chained, we only need to chain the first node
                        let current_nodes = if current_result.densely_chained {
                            &current_result.nodes[..1]
                        } else {
                            &current_result.nodes
                        };
                        // if the previous result was densely chained, we only need to chain the last node
                        let previous_nodes = if previous_result.densely_chained {
                            &previous_result.nodes[previous_result.nodes.len() - 1..]
                        } else {
                            &previous_result.nodes
                        };

                        self.dependency
                            .reserve_edges(previous_nodes.len() * current_nodes.len());
                        for previous_node in previous_nodes {
                            for current_node in current_nodes {
                                self.dependency.add_edge(*previous_node, *current_node);

                                for pass in self.passes.values_mut() {
                                    pass.add_dependency(
                                        *previous_node,
                                        *current_node,
                                        chain_options,
                                    );
                                }
                            }
                        }
                    }
                    if collect_nodes {
                        nodes.append(&mut previous_result.nodes);
                    }

                    previous_result = current_result;
                }
                if collect_nodes {
                    nodes.append(&mut previous_result.nodes);
                }

                ProcessConfigsResult {
                    nodes,
                    densely_chained,
                }
            }
        }
    }

    /// Add a [`ScheduleConfig`] to the graph, including its dependencies and conditions.
    fn add_system_inner(&mut self, config: ScheduleConfig<ScheduleSystem>) -> SystemKey {
        let key = self.systems.insert(config.node, config.conditions);

        // graph updates are immediate
        self.update_graphs(NodeId::System(key), config.metadata);

        key
    }

    #[track_caller]
    fn configure_sets<M>(&mut self, sets: impl IntoScheduleConfigs<InternedSystemSet, M>) {
        self.process_configs(sets.into_configs(), false);
    }

    /// Add a single `ScheduleConfig` to the graph, including its dependencies and conditions.
    fn configure_set_inner(&mut self, config: ScheduleConfig<InternedSystemSet>) -> SystemSetKey {
        let key = self.system_sets.insert(config.node, config.conditions);

        // graph updates are immediate
        self.update_graphs(NodeId::Set(key), config.metadata);

        key
    }

    fn create_anonymous_set(&mut self) -> AnonymousSet {
        let id = self.anonymous_sets;
        self.anonymous_sets += 1;
        AnonymousSet::new(id)
    }

    /// Returns a `Vec` containing all [`SystemKey`]s in a [`SystemSet`].
    ///
    /// # Errors
    ///
    /// This method may return an error. It'll be:
    ///
    /// - `ScheduleError::Uninitialized` if the schedule has been changed,
    ///   and `Self::initialize` has not been called.
    /// - `ScheduleError::NotFound` if `system_set` isn't present in the
    ///   schedule.
    pub fn systems_in_set(
        &self,
        system_set: InternedSystemSet,
    ) -> Result<&HashSet<SystemKey>, ScheduleError> {
        if self.changed {
            return Err(ScheduleError::Uninitialized);
        }
        let system_set_id = self
            .system_sets
            .get_key(system_set)
            .ok_or(ScheduleError::SetNotFound)?;
        self.set_systems
            .get(&system_set_id)
            .ok_or(ScheduleError::SetNotFound)
    }

    fn add_edges_for_transitive_dependencies(&mut self, node: NodeId) {
        let in_nodes: Vec<_> = self.hierarchy.neighbors_directed(node, Incoming).collect();
        let out_nodes: Vec<_> = self.hierarchy.neighbors_directed(node, Outgoing).collect();

        self.hierarchy
            .reserve_edges(in_nodes.len() * out_nodes.len());
        for &in_node in &in_nodes {
            for &out_node in &out_nodes {
                self.hierarchy.add_edge(in_node, out_node);
            }
        }

        let in_nodes: Vec<_> = self.dependency.neighbors_directed(node, Incoming).collect();
        let out_nodes: Vec<_> = self.dependency.neighbors_directed(node, Outgoing).collect();

        self.dependency
            .reserve_edges(in_nodes.len() * out_nodes.len());
        for &in_node in &in_nodes {
            for &out_node in &out_nodes {
                self.dependency.add_edge(in_node, out_node);
            }
        }
    }

    /// Remove all systems in a set and any dependencies on those systems and set.
    pub fn remove_systems_in_set<M>(
        &mut self,
        system_set: impl IntoSystemSet<M>,
        policy: ScheduleCleanupPolicy,
    ) -> Result<usize, ScheduleError> {
        let set = system_set.into_system_set();
        let interned = set.intern();
        // clone the keys out of the schedule as the systems are getting removed from self
        let keys = self.systems_in_set(interned)?.clone();

        self.changed = true;

        match policy {
            ScheduleCleanupPolicy::RemoveSetAndSystemsAllowBreakages => {
                let Some(set_key) = self.system_sets.get_key(interned) else {
                    return Err(ScheduleError::SetNotFound);
                };

                self.remove_systems_by_keys(&keys);
                self.remove_set_by_key(set_key);

                Ok(keys.len())
            }
            ScheduleCleanupPolicy::RemoveSystemsOnlyAllowBreakages => {
                self.remove_systems_by_keys(&keys);

                Ok(keys.len())
            }
            ScheduleCleanupPolicy::RemoveSetAndSystems => {
                let Some(set_key) = self.system_sets.get_key(interned) else {
                    return Err(ScheduleError::SetNotFound);
                };

                for &key in &keys {
                    self.add_edges_for_transitive_dependencies(key.into());
                }

                self.add_edges_for_transitive_dependencies(set_key.into());

                self.remove_systems_by_keys(&keys);
                self.remove_set_by_key(set_key);

                Ok(keys.len())
            }
            ScheduleCleanupPolicy::RemoveSystemsOnly => {
                for &key in &keys {
                    self.add_edges_for_transitive_dependencies(key.into());
                }

                self.remove_systems_by_keys(&keys);

                Ok(keys.len())
            }
        }
    }

    fn remove_systems_by_keys(&mut self, keys: &HashSet<SystemKey>) {
        for &key in keys {
            self.systems.remove(key);

            self.hierarchy.remove_node(key.into());
            self.dependency.remove_node(key.into());
            self.ambiguous_with.remove_node(key.into());
            self.ambiguous_with_all.remove(&NodeId::from(key));
        }
    }

    fn remove_set_by_key(&mut self, key: SystemSetKey) {
        self.system_sets.remove(key);
        self.set_systems.remove(&key);
        self.hierarchy.remove_node(key.into());
        self.dependency.remove_node(key.into());
        self.ambiguous_with.remove_node(key.into());
        self.ambiguous_with_all.remove(&NodeId::from(key));
    }

    /// Update the internal graphs (hierarchy, dependency, ambiguity) by adding a single [`GraphInfo`]
    fn update_graphs(&mut self, id: NodeId, graph_info: GraphInfo) {
        self.changed = true;

        let GraphInfo {
            hierarchy: sets,
            dependencies,
            ambiguous_with,
            ..
        } = graph_info;

        self.hierarchy.add_node(id);
        self.dependency.add_node(id);

        for key in sets
            .into_iter()
            .map(|set| self.system_sets.get_key_or_insert(set))
        {
            self.hierarchy.add_edge(NodeId::Set(key), id);

            // ensure set also appears in dependency graph
            self.dependency.add_node(NodeId::Set(key));
        }

        for (kind, key, options) in
            dependencies
                .into_iter()
                .map(|Dependency { kind, set, options }| {
                    (kind, self.system_sets.get_key_or_insert(set), options)
                })
        {
            let (lhs, rhs) = match kind {
                DependencyKind::Before => (id, NodeId::Set(key)),
                DependencyKind::After => (NodeId::Set(key), id),
            };
            self.dependency.add_edge(lhs, rhs);
            for pass in self.passes.values_mut() {
                pass.add_dependency(lhs, rhs, &options);
            }

            // ensure set also appears in hierarchy graph
            self.hierarchy.add_node(NodeId::Set(key));
        }

        match ambiguous_with {
            Ambiguity::Check => (),
            Ambiguity::IgnoreWithSet(ambiguous_with) => {
                for key in ambiguous_with
                    .into_iter()
                    .map(|set| self.system_sets.get_key_or_insert(set))
                {
                    self.ambiguous_with.add_edge(id, NodeId::Set(key));
                }
            }
            Ambiguity::IgnoreAll => {
                self.ambiguous_with_all.insert(id);
            }
        }
    }

    /// Initializes any newly-added systems and conditions by calling
    /// [`System::initialize`](crate::system::System).
    pub fn initialize(&mut self, world: &mut World) {
        self.systems.initialize(world);
        self.system_sets.initialize(world);
    }

    /// Builds an execution-optimized [`SystemSchedule`] from the current state
    /// of the graph. Also returns any warnings that were generated during the
    /// build process.
    ///
    /// This method also
    /// - checks for dependency or hierarchy cycles
    /// - checks for system access conflicts and reports ambiguities
    pub fn build_schedule(
        &mut self,
        world: &mut World,
    ) -> Result<(SystemSchedule, Vec<ScheduleBuildWarning>), ScheduleBuildError> {
        let mut warnings = Vec::new();

        // Check system set memberships for cycles.
        let hierarchy_analysis = self
            .hierarchy
            .analyze()
            .map_err(ScheduleBuildError::HierarchySort)?;

        // Check for redundant system set memberships, logging warnings or
        // returning errors as configured.
        if self.settings.hierarchy_detection != LogLevel::Ignore
            && let Err(e) = hierarchy_analysis.check_for_redundant_edges()
        {
            match self.settings.hierarchy_detection {
                LogLevel::Error => return Err(ScheduleBuildWarning::HierarchyRedundancy(e).into()),
                LogLevel::Warn => warnings.push(ScheduleBuildWarning::HierarchyRedundancy(e)),
                LogLevel::Ignore => unreachable!(),
            }
        }
        // Remove redundant system set memberships.
        self.hierarchy.remove_redundant_edges(&hierarchy_analysis);

        // Check system and system set ordering dependencies for cycles.
        let dependency_analysis = self
            .dependency
            .analyze()
            .map_err(ScheduleBuildError::DependencySort)?;

        // System sets that share systems and have an ordering dependency cannot be ordered.
        dependency_analysis.check_for_cross_dependencies(&hierarchy_analysis)?;

        // Group all systems by the system sets they belong to.
        self.set_systems = self
            .hierarchy
            .group_by_key(self.system_sets.len())
            .map_err(ScheduleBuildError::HierarchySort)?;
        // Check for system sets that share systems but have an ordering dependency.
        dependency_analysis.check_for_overlapping_groups(&self.set_systems)?;

        // There can be no edges to system-type sets that have multiple instances.
        self.system_sets.check_type_set_ambiguity(
            &self.set_systems,
            &self.ambiguous_with,
            &self.dependency,
        )?;

        // Flatten system ordering dependencies by collapsing system sets. This
        // means that if a system set has ordering dependencies, those
        // dependencies are applied to all systems in the set.
        let mut flat_dependency =
            self.set_systems
                .flatten(self.dependency.clone(), |set, systems, flattening, temp| {
                    for pass in self.passes.values_mut() {
                        pass.collapse_set(set, systems, flattening, temp);
                    }
                });

        // Allow modification of the schedule graph by build passes.
        let mut passes = core::mem::take(&mut self.passes);
        for pass in passes.values_mut() {
            pass.build(world, self, &mut flat_dependency)?;
        }
        self.passes = passes;

        // Check system ordering dependencies for cycles after collapsing sets
        // and applying build passes.
        let flat_dependency_analysis = flat_dependency
            .analyze()
            .map_err(ScheduleBuildError::FlatDependencySort)?;
        flat_dependency.remove_redundant_edges(&flat_dependency_analysis);

        // Flatten accepted system ordering ambiguities by collapsing system sets.
        // This means that if a system set is allowed to have ambiguous ordering
        // with another set, all systems in the first set are allowed to have
        // ambiguous ordering with all systems in the second set.
        let flat_ambiguous_with = self.set_systems.flatten_undirected(&self.ambiguous_with);

        // Find all system ordering ambiguities, ignoring those that are accepted.
        self.conflicting_systems = self.systems.get_conflicting_systems(
            &flat_dependency_analysis,
            &flat_ambiguous_with,
            &self.ambiguous_with_all,
            world
                .get_resource::<IgnoredAmbiguities>()
                .map(|ia| &ia.0)
                .unwrap_or(&HashSet::new()),
        );
        // If there are any ambiguities, log warnings or return errors as configured.
        if self.settings.ambiguity_detection != LogLevel::Ignore
            && let Err(e) = self.conflicting_systems.check_if_not_empty()
        {
            match self.settings.ambiguity_detection {
                LogLevel::Error => return Err(ScheduleBuildWarning::Ambiguity(e).into()),
                LogLevel::Warn => warnings.push(ScheduleBuildWarning::Ambiguity(e)),
                LogLevel::Ignore => unreachable!(),
            }
        }

        // build the schedule
        Ok((
            self.build_schedule_inner(flat_dependency, hierarchy_analysis),
            warnings,
        ))
    }

    fn build_schedule_inner(
        &self,
        flat_dependency: Dag<SystemKey>,
        hierarchy_analysis: DagAnalysis<NodeId>,
    ) -> SystemSchedule {
        let dg_system_ids = flat_dependency.get_toposort().unwrap().to_vec();
        let dg_system_idx_map = dg_system_ids
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect::<HashMap<_, _>>();

        let hierarchy_toposort = self.hierarchy.get_toposort().unwrap();
        let hg_systems = hierarchy_toposort
            .iter()
            .cloned()
            .enumerate()
            .filter_map(|(i, id)| Some((i, id.as_system()?)))
            .collect::<Vec<_>>();
        let (hg_set_with_conditions_idxs, hg_set_ids): (Vec<_>, Vec<_>) = hierarchy_toposort
            .iter()
            .cloned()
            .enumerate()
            .filter_map(|(i, id)| {
                // ignore system sets that have no conditions
                // ignore system type sets (already covered, they don't have conditions)
                let key = id.as_set()?;
                self.system_sets.has_conditions(key).then_some((i, key))
            })
            .unzip();

        let sys_count = self.systems.len();
        let set_with_conditions_count = hg_set_ids.len();
        let hg_node_count = self.hierarchy.node_count();

        // get the number of dependencies and the immediate dependents of each system
        // (needed by multi_threaded executor to run systems in the correct order)
        let mut system_dependencies = Vec::with_capacity(sys_count);
        let mut system_dependents = Vec::with_capacity(sys_count);
        for &sys_key in &dg_system_ids {
            let num_dependencies = flat_dependency
                .neighbors_directed(sys_key, Incoming)
                .count();

            let dependents = flat_dependency
                .neighbors_directed(sys_key, Outgoing)
                .map(|dep_id| dg_system_idx_map[&dep_id])
                .collect::<Vec<_>>();

            system_dependencies.push(num_dependencies);
            system_dependents.push(dependents);
        }

        // get the rows and columns of the hierarchy graph's reachability matrix
        // (needed to we can evaluate conditions in the correct order)
        let mut systems_in_sets_with_conditions =
            vec![FixedBitSet::with_capacity(sys_count); set_with_conditions_count];
        for (i, &row) in hg_set_with_conditions_idxs.iter().enumerate() {
            let bitset = &mut systems_in_sets_with_conditions[i];
            for &(col, sys_key) in &hg_systems {
                let idx = dg_system_idx_map[&sys_key];
                let is_descendant = hierarchy_analysis.reachable()[index(row, col, hg_node_count)];
                bitset.set(idx, is_descendant);
            }
        }

        let mut sets_with_conditions_of_systems =
            vec![FixedBitSet::with_capacity(set_with_conditions_count); sys_count];
        for &(col, sys_key) in &hg_systems {
            let i = dg_system_idx_map[&sys_key];
            let bitset = &mut sets_with_conditions_of_systems[i];
            for (idx, &row) in hg_set_with_conditions_idxs
                .iter()
                .enumerate()
                .take_while(|&(_idx, &row)| row < col)
            {
                let is_ancestor = hierarchy_analysis.reachable()[index(row, col, hg_node_count)];
                bitset.set(idx, is_ancestor);
            }
        }

        SystemSchedule {
            systems: Vec::with_capacity(sys_count),
            system_conditions: Vec::with_capacity(sys_count),
            set_conditions: Vec::with_capacity(set_with_conditions_count),
            system_ids: dg_system_ids,
            set_ids: hg_set_ids,
            system_dependencies,
            system_dependents,
            sets_with_conditions_of_systems,
            systems_in_sets_with_conditions,
        }
    }

    /// Updates the `SystemSchedule` from the `ScheduleGraph`.
    fn update_schedule(
        &mut self,
        world: &mut World,
        schedule: &mut SystemSchedule,
        schedule_label: InternedScheduleLabel,
    ) -> Result<Vec<ScheduleBuildWarning>, ScheduleBuildError> {
        if !self.systems.is_initialized() || !self.system_sets.is_initialized() {
            return Err(ScheduleBuildError::Uninitialized);
        }

        // move systems out of old schedule
        for ((key, system), conditions) in schedule
            .system_ids
            .drain(..)
            .zip(schedule.systems.drain(..))
            .zip(schedule.system_conditions.drain(..))
        {
            if let Some(node) = self.systems.node_mut(key) {
                node.inner = Some(system);
            }

            if let Some(node_conditions) = self.systems.get_conditions_mut(key) {
                *node_conditions = conditions;
            }
        }

        for (key, conditions) in schedule
            .set_ids
            .drain(..)
            .zip(schedule.set_conditions.drain(..))
        {
            if let Some(node_conditions) = self.system_sets.get_conditions_mut(key) {
                *node_conditions = conditions;
            }
        }

        let (new_schedule, warnings) = self.build_schedule(world)?;
        *schedule = new_schedule;

        for warning in &warnings {
            warn!(
                "{:?} schedule built successfully, however: {}",
                schedule_label,
                warning.to_string(self, world)
            );
        }

        // move systems into new schedule
        for &key in &schedule.system_ids {
            let system = self.systems.node_mut(key).unwrap().inner.take().unwrap();
            let conditions = core::mem::take(self.systems.get_conditions_mut(key).unwrap());
            schedule.systems.push(system);
            schedule.system_conditions.push(conditions);
        }

        for &key in &schedule.set_ids {
            let conditions = core::mem::take(self.system_sets.get_conditions_mut(key).unwrap());
            schedule.set_conditions.push(conditions);
        }

        Ok(warnings)
    }
}

/// Values returned by [`ScheduleGraph::process_configs`]
struct ProcessConfigsResult {
    /// All nodes contained inside this `process_configs` call's [`ScheduleConfigs`] hierarchy,
    /// if `ancestor_chained` is true
    nodes: Vec<NodeId>,
    /// True if and only if all nodes are "densely chained", meaning that all nested nodes
    /// are linearly chained (as if `after` system ordering had been applied between each node)
    /// in the order they are defined
    densely_chained: bool,
}

/// Trait used by [`ScheduleGraph::process_configs`] to process a single [`ScheduleConfig`].
trait ProcessScheduleConfig: Schedulable + Sized {
    /// Process a single [`ScheduleConfig`].
    fn process_config(schedule_graph: &mut ScheduleGraph, config: ScheduleConfig<Self>) -> NodeId;
}

impl ProcessScheduleConfig for ScheduleSystem {
    fn process_config(schedule_graph: &mut ScheduleGraph, config: ScheduleConfig<Self>) -> NodeId {
        NodeId::System(schedule_graph.add_system_inner(config))
    }
}

impl ProcessScheduleConfig for InternedSystemSet {
    fn process_config(schedule_graph: &mut ScheduleGraph, config: ScheduleConfig<Self>) -> NodeId {
        NodeId::Set(schedule_graph.configure_set_inner(config))
    }
}

/// Policy to use when removing systems.
#[derive(Default)]
pub enum ScheduleCleanupPolicy {
    /// Remove the referenced set and any systems in the set.
    /// Attempts to maintain the order between the transitive dependencies by adding new edges
    /// between the existing before and after dependencies on the set and the systems.
    /// This does not remove sets that might sub sets of the set.
    #[default]
    RemoveSetAndSystems,
    /// Remove only the systems in the set. The set
    /// Attempts to maintain the order between the transitive dependencies by adding new edges
    /// between the existing before and after dependencies on the systems.
    RemoveSystemsOnly,
    /// Remove the set and any systems in the set.
    /// Note that this will not add new edges and
    /// so will break any transitive dependencies on that set or systems.
    /// This does not remove sets that might sub sets of the set.
    RemoveSetAndSystemsAllowBreakages,
    /// Remove only the systems in the set.
    /// Note that this will not add new edges and
    /// so will break any transitive dependencies on that set or systems.
    RemoveSystemsOnlyAllowBreakages,
}

// methods for reporting errors
impl ScheduleGraph {
    /// Returns the name of the node with the given [`NodeId`]. Resolves
    /// anonymous sets to a string that describes their contents.
    ///
    /// Also displays the set(s) the node is contained in if
    /// [`ScheduleBuildSettings::report_sets`] is true, and shortens system names
    /// if [`ScheduleBuildSettings::use_shortnames`] is true.
    pub fn get_node_name(&self, id: &NodeId) -> String {
        self.get_node_name_inner(id, self.settings.report_sets)
    }

    #[inline]
    fn get_node_name_inner(&self, id: &NodeId, report_sets: bool) -> String {
        match *id {
            NodeId::System(key) => {
                let name = self.systems[key].name();
                let name = if self.settings.use_shortnames {
                    name.shortname().to_string()
                } else {
                    name.to_string()
                };
                if report_sets {
                    let sets = self.names_of_sets_containing_node(id);
                    if sets.is_empty() {
                        name
                    } else if sets.len() == 1 {
                        format!("{name} (in set {})", sets[0])
                    } else {
                        format!("{name} (in sets {})", sets.join(", "))
                    }
                } else {
                    name
                }
            }
            NodeId::Set(key) => {
                let set = &self.system_sets[key];
                if set.is_anonymous() {
                    self.anonymous_set_name(id)
                } else {
                    format!("{set:?}")
                }
            }
        }
    }

    fn anonymous_set_name(&self, id: &NodeId) -> String {
        format!(
            "({})",
            self.hierarchy
                .edges_directed(*id, Outgoing)
                // never get the sets of the members or this will infinite recurse when the report_sets setting is on.
                .map(|(_, member_id)| self.get_node_name_inner(&member_id, false))
                .reduce(|a, b| format!("{a}, {b}"))
                .unwrap_or_default()
        )
    }

    fn traverse_sets_containing_node(&self, id: NodeId, f: &mut impl FnMut(SystemSetKey) -> bool) {
        for (set_id, _) in self.hierarchy.edges_directed(id, Incoming) {
            let NodeId::Set(set_key) = set_id else {
                continue;
            };
            if f(set_key) {
                self.traverse_sets_containing_node(NodeId::Set(set_key), f);
            }
        }
    }

    fn names_of_sets_containing_node(&self, id: &NodeId) -> Vec<String> {
        let mut sets = <HashSet<_>>::default();
        self.traverse_sets_containing_node(*id, &mut |key| {
            self.system_sets[key].system_type().is_none() && sets.insert(key)
        });
        let mut sets: Vec<_> = sets
            .into_iter()
            .map(|key| self.get_node_name(&NodeId::Set(key)))
            .collect();
        sets.sort();
        sets
    }
}

/// Specifies how schedule construction should respond to detecting a certain kind of issue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    /// Occurrences are completely ignored.
    Ignore,
    /// Occurrences are logged only.
    Warn,
    /// Occurrences are logged and result in errors.
    Error,
}

/// Specifies miscellaneous settings for schedule construction.
#[derive(Clone, Debug)]
pub struct ScheduleBuildSettings {
    /// Determines whether the presence of ambiguities (systems with conflicting access but indeterminate order)
    /// is only logged or also results in an [`Ambiguity`](ScheduleBuildWarning::Ambiguity)
    /// warning or error.
    ///
    /// Defaults to [`LogLevel::Ignore`].
    pub ambiguity_detection: LogLevel,
    /// Determines whether the presence of redundant edges in the hierarchy of system sets is only
    /// logged or also results in a [`HierarchyRedundancy`](ScheduleBuildWarning::HierarchyRedundancy)
    /// warning or error.
    ///
    /// Defaults to [`LogLevel::Warn`].
    pub hierarchy_detection: LogLevel,
    /// Auto insert [`ApplyDeferred`] systems into the schedule,
    /// when there are [`Deferred`](crate::prelude::Deferred)
    /// in one system and there are ordering dependencies on that system. [`Commands`](crate::system::Commands) is one
    /// such deferred buffer.
    ///
    /// You may want to disable this if you only want to sync deferred params at the end of the schedule,
    /// or want to manually insert all your sync points.
    ///
    /// Defaults to `true`
    pub auto_insert_apply_deferred: bool,
    /// If set to true, node names will be shortened instead of the fully qualified type path.
    ///
    /// Defaults to `true`.
    pub use_shortnames: bool,
    /// If set to true, report all system sets the conflicting systems are part of.
    ///
    /// Defaults to `true`.
    pub report_sets: bool,
}

impl Default for ScheduleBuildSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleBuildSettings {
    /// Default build settings.
    /// See the field-level documentation for the default value of each field.
    pub const fn new() -> Self {
        Self {
            ambiguity_detection: LogLevel::Ignore,
            hierarchy_detection: LogLevel::Warn,
            auto_insert_apply_deferred: true,
            use_shortnames: true,
            report_sets: true,
        }
    }
}

/// Error to denote that [`Schedule::initialize`] or [`Schedule::run`] has not yet been called for
/// this schedule.
#[derive(Error, Debug)]
#[error("executable schedule has not been built")]
pub struct ScheduleNotInitialized;

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};
    use core::any::TypeId;

    use bevy_ecs_macros::ScheduleLabel;

    use crate::{
        error::{ignore, panic, DefaultErrorHandler, Result},
        prelude::{ApplyDeferred, IntoSystemSet, Res, Resource},
        schedule::{
            passes::AutoInsertApplyDeferredPass, tests::ResMut, IntoScheduleConfigs, Schedule,
            ScheduleBuildPass, ScheduleBuildSettings, ScheduleCleanupPolicy, SystemSet,
        },
        system::Commands,
        world::World,
    };

    use super::Schedules;

    #[derive(Resource)]
    struct Resource1;

    #[derive(Resource)]
    struct Resource2;

    #[test]
    fn unchanged_auto_insert_apply_deferred_has_no_effect() {
        use alloc::{vec, vec::Vec};

        #[derive(PartialEq, Debug)]
        enum Entry {
            System(usize),
            SyncPoint(usize),
        }

        #[derive(Resource, Default)]
        struct Log(Vec<Entry>);

        fn system<const N: usize>(mut res: ResMut<Log>, mut commands: Commands) {
            res.0.push(Entry::System(N));
            commands
                .queue(|world: &mut World| world.resource_mut::<Log>().0.push(Entry::SyncPoint(N)));
        }

        let mut world = World::default();
        world.init_resource::<Log>();
        let mut schedule = Schedule::default();
        schedule.add_systems((system::<1>, system::<2>).chain_ignore_deferred());
        schedule.set_build_settings(ScheduleBuildSettings {
            auto_insert_apply_deferred: true,
            ..Default::default()
        });
        schedule.run(&mut world);
        let actual = world.remove_resource::<Log>().unwrap().0;

        let expected = vec![
            Entry::System(1),
            Entry::System(2),
            Entry::SyncPoint(1),
            Entry::SyncPoint(2),
        ];

        assert_eq!(actual, expected);
    }

    // regression test for https://github.com/bevyengine/bevy/issues/9114
    #[test]
    fn ambiguous_with_not_breaking_run_conditions() {
        #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
        struct Set;

        let mut world = World::new();
        let mut schedule = Schedule::default();

        let system: fn() = || {
            panic!("This system must not run");
        };

        schedule.configure_sets(Set.run_if(|| false));
        schedule.add_systems(system.ambiguous_with(|| ()).in_set(Set));
        schedule.run(&mut world);
    }

    #[test]
    fn inserts_a_sync_point() {
        let mut schedule = Schedule::default();
        let mut world = World::default();
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |_: Res<Resource1>| {},
            )
                .chain(),
        );
        schedule.run(&mut world);

        // inserted a sync point
        assert_eq!(schedule.executable.systems.len(), 3);
    }

    #[test]
    fn explicit_sync_point_used_as_auto_sync_point() {
        let mut schedule = Schedule::default();
        let mut world = World::default();
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |_: Res<Resource1>| {},
            )
                .chain(),
        );
        schedule.add_systems((|| {}, ApplyDeferred, || {}).chain());
        schedule.run(&mut world);

        // No sync point was inserted, since we can reuse the explicit sync point.
        assert_eq!(schedule.executable.systems.len(), 5);
    }

    #[test]
    fn conditional_explicit_sync_point_not_used_as_auto_sync_point() {
        let mut schedule = Schedule::default();
        let mut world = World::default();
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |_: Res<Resource1>| {},
            )
                .chain(),
        );
        schedule.add_systems((|| {}, ApplyDeferred.run_if(|| false), || {}).chain());
        schedule.run(&mut world);

        // A sync point was inserted, since the explicit sync point is not always run.
        assert_eq!(schedule.executable.systems.len(), 6);
    }

    #[test]
    fn conditional_explicit_sync_point_not_used_as_auto_sync_point_condition_on_chain() {
        let mut schedule = Schedule::default();
        let mut world = World::default();
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |_: Res<Resource1>| {},
            )
                .chain(),
        );
        schedule.add_systems((|| {}, ApplyDeferred, || {}).chain().run_if(|| false));
        schedule.run(&mut world);

        // A sync point was inserted, since the explicit sync point is not always run.
        assert_eq!(schedule.executable.systems.len(), 6);
    }

    #[test]
    fn conditional_explicit_sync_point_not_used_as_auto_sync_point_condition_on_system_set() {
        #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
        struct Set;

        let mut schedule = Schedule::default();
        let mut world = World::default();
        schedule.configure_sets(Set.run_if(|| false));
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |_: Res<Resource1>| {},
            )
                .chain(),
        );
        schedule.add_systems((|| {}, ApplyDeferred.in_set(Set), || {}).chain());
        schedule.run(&mut world);

        // A sync point was inserted, since the explicit sync point is not always run.
        assert_eq!(schedule.executable.systems.len(), 6);
    }

    #[test]
    fn conditional_explicit_sync_point_not_used_as_auto_sync_point_condition_on_nested_system_set()
    {
        #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
        struct Set1;
        #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
        struct Set2;

        let mut schedule = Schedule::default();
        let mut world = World::default();
        schedule.configure_sets(Set2.run_if(|| false));
        schedule.configure_sets(Set1.in_set(Set2));
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |_: Res<Resource1>| {},
            )
                .chain(),
        );
        schedule.add_systems((|| {}, ApplyDeferred, || {}).chain().in_set(Set1));
        schedule.run(&mut world);

        // A sync point was inserted, since the explicit sync point is not always run.
        assert_eq!(schedule.executable.systems.len(), 6);
    }

    #[test]
    fn merges_sync_points_into_one() {
        let mut schedule = Schedule::default();
        let mut world = World::default();
        // insert two parallel command systems, it should only create one sync point
        schedule.add_systems(
            (
                (
                    |mut commands: Commands| commands.insert_resource(Resource1),
                    |mut commands: Commands| commands.insert_resource(Resource2),
                ),
                |_: Res<Resource1>, _: Res<Resource2>| {},
            )
                .chain(),
        );
        schedule.run(&mut world);

        // inserted sync points
        assert_eq!(schedule.executable.systems.len(), 4);

        // merges sync points on rebuild
        schedule.add_systems(((
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |mut commands: Commands| commands.insert_resource(Resource2),
            ),
            |_: Res<Resource1>, _: Res<Resource2>| {},
        )
            .chain(),));
        schedule.run(&mut world);

        assert_eq!(schedule.executable.systems.len(), 7);
    }

    #[test]
    fn adds_multiple_consecutive_syncs() {
        let mut schedule = Schedule::default();
        let mut world = World::default();
        // insert two consecutive command systems, it should create two sync points
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |mut commands: Commands| commands.insert_resource(Resource2),
                |_: Res<Resource1>, _: Res<Resource2>| {},
            )
                .chain(),
        );
        schedule.run(&mut world);

        assert_eq!(schedule.executable.systems.len(), 5);
    }

    #[test]
    fn do_not_consider_ignore_deferred_before_exclusive_system() {
        let mut schedule = Schedule::default();
        let mut world = World::default();
        // chain_ignore_deferred adds no sync points usually but an exception is made for exclusive systems
        schedule.add_systems(
            (
                |_: Commands| {},
                // <- no sync point is added here because the following system is not exclusive
                |mut commands: Commands| commands.insert_resource(Resource1),
                // <- sync point is added here because the following system is exclusive which expects to see all commands to that point
                |world: &mut World| assert!(world.contains_resource::<Resource1>()),
                // <- no sync point is added here because the previous system has no deferred parameters
                |_: &mut World| {},
                // <- no sync point is added here because the following system is not exclusive
                |_: Commands| {},
            )
                .chain_ignore_deferred(),
        );
        schedule.run(&mut world);

        assert_eq!(schedule.executable.systems.len(), 6); // 5 systems + 1 sync point
    }

    #[test]
    fn bubble_sync_point_through_ignore_deferred_node() {
        let mut schedule = Schedule::default();
        let mut world = World::default();

        let insert_resource_config = (
            // the first system has deferred commands
            |mut commands: Commands| commands.insert_resource(Resource1),
            // the second system has no deferred commands
            || {},
        )
            // the first two systems are chained without a sync point in between
            .chain_ignore_deferred();

        schedule.add_systems(
            (
                insert_resource_config,
                // the third system would panic if the command of the first system was not applied
                |_: Res<Resource1>| {},
            )
                // the third system is chained after the first two, possibly with a sync point in between
                .chain(),
        );

        // To add a sync point between the second and third system despite the second having no commands,
        // the first system has to signal the second system that there are unapplied commands.
        // With that the second system will add a sync point after it so the third system will find the resource.

        schedule.run(&mut world);

        assert_eq!(schedule.executable.systems.len(), 4); // 3 systems + 1 sync point
    }

    #[test]
    fn disable_auto_sync_points() {
        let mut schedule = Schedule::default();
        schedule.set_build_settings(ScheduleBuildSettings {
            auto_insert_apply_deferred: false,
            ..Default::default()
        });
        let mut world = World::default();
        schedule.add_systems(
            (
                |mut commands: Commands| commands.insert_resource(Resource1),
                |res: Option<Res<Resource1>>| assert!(res.is_none()),
            )
                .chain(),
        );
        schedule.run(&mut world);

        assert_eq!(schedule.executable.systems.len(), 2);
    }

    mod no_sync_edges {
        use super::*;

        fn insert_resource(mut commands: Commands) {
            commands.insert_resource(Resource1);
        }

        fn resource_does_not_exist(res: Option<Res<Resource1>>) {
            assert!(res.is_none());
        }

        #[derive(SystemSet, Hash, PartialEq, Eq, Debug, Clone)]
        enum Sets {
            A,
            B,
        }

        fn check_no_sync_edges(add_systems: impl FnOnce(&mut Schedule)) {
            let mut schedule = Schedule::default();
            let mut world = World::default();
            add_systems(&mut schedule);

            schedule.run(&mut world);

            assert_eq!(schedule.executable.systems.len(), 2);
        }

        #[test]
        fn system_to_system_after() {
            check_no_sync_edges(|schedule| {
                schedule.add_systems((
                    insert_resource,
                    resource_does_not_exist.after_ignore_deferred(insert_resource),
                ));
            });
        }

        #[test]
        fn system_to_system_before() {
            check_no_sync_edges(|schedule| {
                schedule.add_systems((
                    insert_resource.before_ignore_deferred(resource_does_not_exist),
                    resource_does_not_exist,
                ));
            });
        }

        #[test]
        fn set_to_system_after() {
            check_no_sync_edges(|schedule| {
                schedule
                    .add_systems((insert_resource, resource_does_not_exist.in_set(Sets::A)))
                    .configure_sets(Sets::A.after_ignore_deferred(insert_resource));
            });
        }

        #[test]
        fn set_to_system_before() {
            check_no_sync_edges(|schedule| {
                schedule
                    .add_systems((insert_resource.in_set(Sets::A), resource_does_not_exist))
                    .configure_sets(Sets::A.before_ignore_deferred(resource_does_not_exist));
            });
        }

        #[test]
        fn set_to_set_after() {
            check_no_sync_edges(|schedule| {
                schedule
                    .add_systems((
                        insert_resource.in_set(Sets::A),
                        resource_does_not_exist.in_set(Sets::B),
                    ))
                    .configure_sets(Sets::B.after_ignore_deferred(Sets::A));
            });
        }

        #[test]
        fn set_to_set_before() {
            check_no_sync_edges(|schedule| {
                schedule
                    .add_systems((
                        insert_resource.in_set(Sets::A),
                        resource_does_not_exist.in_set(Sets::B),
                    ))
                    .configure_sets(Sets::A.before_ignore_deferred(Sets::B));
            });
        }
    }

    mod no_sync_chain {
        use super::*;

        #[derive(Resource)]
        struct Ra;

        #[derive(Resource)]
        struct Rb;

        #[derive(Resource)]
        struct Rc;

        fn run_schedule(expected_num_systems: usize, add_systems: impl FnOnce(&mut Schedule)) {
            let mut schedule = Schedule::default();
            let mut world = World::default();
            add_systems(&mut schedule);

            schedule.run(&mut world);

            assert_eq!(schedule.executable.systems.len(), expected_num_systems);
        }

        #[test]
        fn only_chain_outside() {
            run_schedule(5, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands| commands.insert_resource(Rb),
                        ),
                        (
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                            },
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                            },
                        ),
                    )
                        .chain(),
                );
            });

            run_schedule(4, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands| commands.insert_resource(Rb),
                        ),
                        (
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_none());
                                assert!(res_b.is_none());
                            },
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_none());
                                assert!(res_b.is_none());
                            },
                        ),
                    )
                        .chain_ignore_deferred(),
                );
            });
        }

        #[test]
        fn chain_first() {
            run_schedule(6, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands, res_a: Option<Res<Ra>>| {
                                commands.insert_resource(Rb);
                                assert!(res_a.is_some());
                            },
                        )
                            .chain(),
                        (
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                            },
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                            },
                        ),
                    )
                        .chain(),
                );
            });

            run_schedule(5, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands, res_a: Option<Res<Ra>>| {
                                commands.insert_resource(Rb);
                                assert!(res_a.is_some());
                            },
                        )
                            .chain(),
                        (
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_none());
                            },
                            |res_a: Option<Res<Ra>>, res_b: Option<Res<Rb>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_none());
                            },
                        ),
                    )
                        .chain_ignore_deferred(),
                );
            });
        }

        #[test]
        fn chain_second() {
            run_schedule(6, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands| commands.insert_resource(Rb),
                        ),
                        (
                            |mut commands: Commands,
                             res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>| {
                                commands.insert_resource(Rc);
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                            },
                            |res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>,
                             res_c: Option<Res<Rc>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                                assert!(res_c.is_some());
                            },
                        )
                            .chain(),
                    )
                        .chain(),
                );
            });

            run_schedule(5, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands| commands.insert_resource(Rb),
                        ),
                        (
                            |mut commands: Commands,
                             res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>| {
                                commands.insert_resource(Rc);
                                assert!(res_a.is_none());
                                assert!(res_b.is_none());
                            },
                            |res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>,
                             res_c: Option<Res<Rc>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                                assert!(res_c.is_some());
                            },
                        )
                            .chain(),
                    )
                        .chain_ignore_deferred(),
                );
            });
        }

        #[test]
        fn chain_all() {
            run_schedule(7, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands, res_a: Option<Res<Ra>>| {
                                commands.insert_resource(Rb);
                                assert!(res_a.is_some());
                            },
                        )
                            .chain(),
                        (
                            |mut commands: Commands,
                             res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>| {
                                commands.insert_resource(Rc);
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                            },
                            |res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>,
                             res_c: Option<Res<Rc>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                                assert!(res_c.is_some());
                            },
                        )
                            .chain(),
                    )
                        .chain(),
                );
            });

            run_schedule(6, |schedule: &mut Schedule| {
                schedule.add_systems(
                    (
                        (
                            |mut commands: Commands| commands.insert_resource(Ra),
                            |mut commands: Commands, res_a: Option<Res<Ra>>| {
                                commands.insert_resource(Rb);
                                assert!(res_a.is_some());
                            },
                        )
                            .chain(),
                        (
                            |mut commands: Commands,
                             res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>| {
                                commands.insert_resource(Rc);
                                assert!(res_a.is_some());
                                assert!(res_b.is_none());
                            },
                            |res_a: Option<Res<Ra>>,
                             res_b: Option<Res<Rb>>,
                             res_c: Option<Res<Rc>>| {
                                assert!(res_a.is_some());
                                assert!(res_b.is_some());
                                assert!(res_c.is_some());
                            },
                        )
                            .chain(),
                    )
                        .chain_ignore_deferred(),
                );
            });
        }
    }

    #[derive(ScheduleLabel, Hash, Debug, Clone, PartialEq, Eq)]
    struct TestSchedule;

    #[derive(Resource)]
    struct CheckSystemRan(usize);

    #[test]
    fn add_systems_to_existing_schedule() {
        let mut schedules = Schedules::default();
        let schedule = Schedule::new(TestSchedule);

        schedules.insert(schedule);
        schedules.add_systems(TestSchedule, |mut ran: ResMut<CheckSystemRan>| ran.0 += 1);

        let mut world = World::new();

        world.insert_resource(CheckSystemRan(0));
        world.insert_resource(schedules);
        world.run_schedule(TestSchedule);

        let value = world
            .get_resource::<CheckSystemRan>()
            .expect("CheckSystemRan Resource Should Exist");
        assert_eq!(value.0, 1);
    }

    #[test]
    fn add_systems_to_non_existing_schedule() {
        let mut schedules = Schedules::default();

        schedules.add_systems(TestSchedule, |mut ran: ResMut<CheckSystemRan>| ran.0 += 1);

        let mut world = World::new();

        world.insert_resource(CheckSystemRan(0));
        world.insert_resource(schedules);
        world.run_schedule(TestSchedule);

        let value = world
            .get_resource::<CheckSystemRan>()
            .expect("CheckSystemRan Resource Should Exist");
        assert_eq!(value.0, 1);
    }

    #[derive(SystemSet, Debug, Hash, Clone, PartialEq, Eq)]
    enum TestSet {
        First,
        Second,
    }

    #[test]
    fn configure_set_on_existing_schedule() {
        let mut schedules = Schedules::default();
        let schedule = Schedule::new(TestSchedule);

        schedules.insert(schedule);

        schedules.configure_sets(TestSchedule, (TestSet::First, TestSet::Second).chain());
        schedules.add_systems(
            TestSchedule,
            (|mut ran: ResMut<CheckSystemRan>| {
                assert_eq!(ran.0, 0);
                ran.0 += 1;
            })
            .in_set(TestSet::First),
        );

        schedules.add_systems(
            TestSchedule,
            (|mut ran: ResMut<CheckSystemRan>| {
                assert_eq!(ran.0, 1);
                ran.0 += 1;
            })
            .in_set(TestSet::Second),
        );

        let mut world = World::new();

        world.insert_resource(CheckSystemRan(0));
        world.insert_resource(schedules);
        world.run_schedule(TestSchedule);

        let value = world
            .get_resource::<CheckSystemRan>()
            .expect("CheckSystemRan Resource Should Exist");
        assert_eq!(value.0, 2);
    }

    #[test]
    fn configure_set_on_new_schedule() {
        let mut schedules = Schedules::default();

        schedules.configure_sets(TestSchedule, (TestSet::First, TestSet::Second).chain());
        schedules.add_systems(
            TestSchedule,
            (|mut ran: ResMut<CheckSystemRan>| {
                assert_eq!(ran.0, 0);
                ran.0 += 1;
            })
            .in_set(TestSet::First),
        );

        schedules.add_systems(
            TestSchedule,
            (|mut ran: ResMut<CheckSystemRan>| {
                assert_eq!(ran.0, 1);
                ran.0 += 1;
            })
            .in_set(TestSet::Second),
        );

        let mut world = World::new();

        world.insert_resource(CheckSystemRan(0));
        world.insert_resource(schedules);
        world.run_schedule(TestSchedule);

        let value = world
            .get_resource::<CheckSystemRan>()
            .expect("CheckSystemRan Resource Should Exist");
        assert_eq!(value.0, 2);
    }

    #[test]
    fn test_default_error_handler() {
        #[derive(Resource, Default)]
        struct Ran(bool);

        fn system(mut ran: ResMut<Ran>) -> Result {
            ran.0 = true;
            Err("I failed!".into())
        }

        // Test that the default error handler is used
        let mut world = World::default();
        world.init_resource::<Ran>();
        world.insert_resource(DefaultErrorHandler(ignore));
        let mut schedule = Schedule::default();
        schedule.add_systems(system).run(&mut world);
        assert!(world.resource::<Ran>().0);

        // Test that the handler doesn't change within the schedule
        schedule.add_systems(
            (|world: &mut World| {
                world.insert_resource(DefaultErrorHandler(panic));
            })
            .before(system),
        );
        schedule.run(&mut world);
    }

    #[test]
    fn get_a_system_key() {
        fn test_system() {}

        let mut schedule = Schedule::default();
        schedule.add_systems(test_system);
        let mut world = World::default();
        let _ = schedule.initialize(&mut world);

        let keys = schedule
            .graph()
            .systems_in_set(test_system.into_system_set().intern())
            .unwrap();
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn get_system_keys_in_set() {
        fn system_1() {}
        fn system_2() {}

        let mut schedule = Schedule::default();
        schedule.add_systems((system_1, system_2).in_set(TestSet::First));
        let mut world = World::default();
        let _ = schedule.initialize(&mut world);

        let keys = schedule
            .graph()
            .systems_in_set(TestSet::First.into_system_set().intern())
            .unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn get_system_keys_with_same_name() {
        fn test_system() {}

        let mut schedule = Schedule::default();
        schedule.add_systems((test_system, test_system));
        let mut world = World::default();
        let _ = schedule.initialize(&mut world);

        let keys = schedule
            .graph()
            .systems_in_set(test_system.into_system_set().intern())
            .unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn remove_a_system() {
        fn system() {}

        let mut schedule = Schedule::default();
        schedule.add_systems(system);
        let mut world = World::default();

        let remove_count = schedule.remove_systems_in_set(
            system,
            &mut world,
            ScheduleCleanupPolicy::RemoveSetAndSystemsAllowBreakages,
        );
        assert_eq!(remove_count.unwrap(), 1);

        // schedule has changed, so we check initializing again
        schedule.initialize(&mut world).unwrap();
        assert_eq!(schedule.graph().systems.len(), 0);
    }

    #[test]
    fn remove_multiple_systems() {
        fn system() {}

        let mut schedule = Schedule::default();
        schedule.add_systems((system, system));
        let mut world = World::default();

        let remove_count = schedule.remove_systems_in_set(
            system,
            &mut world,
            ScheduleCleanupPolicy::RemoveSetAndSystemsAllowBreakages,
        );
        assert_eq!(remove_count.unwrap(), 2);

        // schedule has changed, so we check initializing again
        schedule.initialize(&mut world).unwrap();
        assert_eq!(schedule.graph().systems.len(), 0);
    }

    #[test]
    fn remove_a_system_with_dependencies() {
        fn system_1() {}
        fn system_2() {}

        let mut schedule = Schedule::default();
        schedule.add_systems((system_1, system_2).chain());
        let mut world = World::default();

        let remove_count = schedule.remove_systems_in_set(
            system_1,
            &mut world,
            ScheduleCleanupPolicy::RemoveSetAndSystemsAllowBreakages,
        );
        assert_eq!(remove_count.unwrap(), 1);

        // schedule has changed, so we check initializing again
        schedule.initialize(&mut world).unwrap();
        assert_eq!(schedule.graph().systems.len(), 1);
    }

    #[test]
    fn remove_a_system_and_still_ordered() {
        #[derive(Resource)]
        struct A;

        fn system_1(_: ResMut<A>) {}
        fn system_2() {}
        fn system_3(_: ResMut<A>) {}

        let mut schedule = Schedule::default();
        schedule.add_systems((system_1, system_2, system_3).chain());
        let mut world = World::new();

        let _ = schedule.remove_systems_in_set(
            system_2,
            &mut world,
            ScheduleCleanupPolicy::RemoveSetAndSystems,
        );

        let result = schedule.initialize(&mut world);
        assert!(result.is_ok());
        let conflicts = schedule.graph().conflicting_systems();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn remove_a_set_and_still_ordered() {
        #[derive(Resource)]
        struct A;

        #[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
        struct B;

        fn system_1(_: ResMut<A>) {}
        fn system_2() {}
        fn system_3(_: ResMut<A>) {}

        let mut schedule = Schedule::default();
        schedule.add_systems((system_1.before(B), system_2, system_3.after(B)));
        let mut world = World::new();

        let _ = schedule.remove_systems_in_set(
            B,
            &mut world,
            ScheduleCleanupPolicy::RemoveSetAndSystems,
        );

        let result = schedule.initialize(&mut world);
        assert!(result.is_ok());
        let conflicts = schedule.graph().conflicting_systems();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn build_pass_iteration_order() {
        #[derive(Debug)]
        struct Pass<const N: usize>;

        impl<const N: usize> ScheduleBuildPass for Pass<N> {
            type EdgeOptions = ();
            fn add_dependency(
                &mut self,
                _from: crate::schedule::NodeId,
                _to: crate::schedule::NodeId,
                _options: Option<&Self::EdgeOptions>,
            ) {
            }
            fn build(
                &mut self,
                _world: &mut World,
                _graph: &mut super::ScheduleGraph,
                _dependency_flattened: &mut crate::schedule::graph::Dag<crate::schedule::SystemKey>,
            ) -> core::result::Result<(), crate::schedule::ScheduleBuildError> {
                Ok(())
            }
            fn collapse_set(
                &mut self,
                _set: crate::schedule::SystemSetKey,
                _systems: &bevy_platform::collections::HashSet<crate::schedule::SystemKey>,
                _dependency_flattening: &crate::schedule::graph::DiGraph<crate::schedule::NodeId>,
            ) -> impl Iterator<Item = (crate::schedule::NodeId, crate::schedule::NodeId)>
            {
                core::iter::empty()
            }
        }

        let mut schedule = Schedule::default();
        schedule.add_build_pass(Pass::<0>);
        schedule.add_build_pass(Pass::<1>);
        schedule.add_build_pass(Pass::<2>);

        let pass_order: Vec<TypeId> = schedule.graph().passes.keys().cloned().collect();

        assert_eq!(
            pass_order,
            vec![
                TypeId::of::<AutoInsertApplyDeferredPass>(),
                TypeId::of::<Pass<0>>(),
                TypeId::of::<Pass<1>>(),
                TypeId::of::<Pass<2>>()
            ]
        );
    }
}
