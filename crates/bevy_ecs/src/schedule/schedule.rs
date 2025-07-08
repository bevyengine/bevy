#![expect(
    clippy::module_inception,
    reason = "This instance of module inception is being discussed; see #17344."
)]
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_utils::{default, prelude::DebugName, TypeIdMap};
use core::{
    any::{Any, TypeId},
    fmt::{Debug, Write},
};
use fixedbitset::FixedBitSet;
use log::{error, info, warn};
use pass::ScheduleBuildPassObj;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use thiserror::Error;
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::component::CheckChangeTicks;
use crate::{
    component::{ComponentId, Components},
    prelude::Component,
    query::FilteredAccessSet,
    resource::Resource,
    schedule::*,
    system::ScheduleSystem,
    world::World,
};

use crate::{query::AccessConflicts, storage::SparseSetIndex};
pub use stepping::Stepping;
use Direction::{Incoming, Outgoing};

/// Resource that stores [`Schedule`]s mapped to [`ScheduleLabel`]s excluding the current running [`Schedule`].
#[derive(Default, Resource)]
pub struct Schedules {
    inner: HashMap<InternedScheduleLabel, Schedule>,
    /// List of [`ComponentId`]s to ignore when reporting system order ambiguity conflicts
    pub ignored_scheduling_ambiguities: BTreeSet<ComponentId>,
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
    pub fn allow_ambiguous_component<T: Component>(&mut self, world: &mut World) {
        self.ignored_scheduling_ambiguities
            .insert(world.register_component::<T>());
    }

    /// Ignore system order ambiguities caused by conflicts on [`Resource`]s of type `T`.
    pub fn allow_ambiguous_resource<T: Resource>(&mut self, world: &mut World) {
        self.ignored_scheduling_ambiguities
            .insert(world.components_registrator().register_resource::<T>());
    }

    /// Iterate through the [`ComponentId`]'s that will be ignored.
    pub fn iter_ignored_ambiguities(&self) -> impl Iterator<Item = &ComponentId> + '_ {
        self.ignored_scheduling_ambiguities.iter()
    }

    /// Prints the names of the components and resources with [`info`]
    ///
    /// May panic or retrieve incorrect names if [`Components`] is not from the same
    /// world
    pub fn print_ignored_ambiguities(&self, components: &Components) {
        let mut message =
            "System order ambiguities caused by conflicts on the following types are ignored:\n"
                .to_string();
        for id in self.iter_ignored_ambiguities() {
            writeln!(message, "{}", components.get_name(*id).unwrap()).unwrap();
        }

        info!("{message}");
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
        #[expect(deprecated, reason = "We still need to support this.")]
        ExecutorKind::Simple => Box::new(SimpleExecutor::new()),
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

        let Some(&a_id) = self.graph.system_set_ids.get(&a.intern()) else {
            panic!(
                "Could not mark system as ambiguous, `{:?}` was not found in the schedule.
                Did you try to call `ambiguous_with` before adding the system to the world?",
                a
            );
        };
        let Some(&b_id) = self.graph.system_set_ids.get(&b.intern()) else {
            panic!(
                "Could not mark system as ambiguous, `{:?}` was not found in the schedule.
                Did you try to call `ambiguous_with` before adding the system to the world?",
                b
            );
        };

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
        self.graph.passes.remove(&TypeId::of::<T>());
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
        self.initialize(world)
            .unwrap_or_else(|e| panic!("Error when initializing schedule {:?}: {e}", self.label));

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
            let ignored_ambiguities = world
                .get_resource_or_init::<Schedules>()
                .ignored_scheduling_ambiguities
                .clone();
            self.graph.update_schedule(
                world,
                &mut self.executable,
                &ignored_ambiguities,
                self.label,
            )?;
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
        for SystemWithAccess { system, .. } in &mut self.executable.systems {
            if !is_apply_deferred(system) {
                system.check_change_tick(check);
            }
        }

        for conditions in &mut self.executable.system_conditions {
            for system in conditions {
                system.condition.check_change_tick(check);
            }
        }

        for conditions in &mut self.executable.set_conditions {
            for system in conditions {
                system.condition.check_change_tick(check);
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
}

/// A directed acyclic graph structure.
#[derive(Default)]
pub struct Dag {
    /// A directed graph.
    graph: DiGraph,
    /// A cached topological ordering of the graph.
    topsort: Vec<NodeId>,
}

impl Dag {
    fn new() -> Self {
        Self {
            graph: DiGraph::default(),
            topsort: Vec::new(),
        }
    }

    /// The directed graph of the stored systems, connected by their ordering dependencies.
    pub fn graph(&self) -> &DiGraph {
        &self.graph
    }

    /// A cached topological ordering of the graph.
    ///
    /// The order is determined by the ordering dependencies between systems.
    pub fn cached_topsort(&self) -> &[NodeId] {
        &self.topsort
    }
}

/// A [`SystemSet`] with metadata, stored in a [`ScheduleGraph`].
struct SystemSetNode {
    inner: InternedSystemSet,
}

impl SystemSetNode {
    pub fn new(set: InternedSystemSet) -> Self {
        Self { inner: set }
    }

    pub fn name(&self) -> String {
        format!("{:?}", &self.inner)
    }

    pub fn is_system_type(&self) -> bool {
        self.inner.system_type().is_some()
    }

    pub fn is_anonymous(&self) -> bool {
        self.inner.is_anonymous()
    }
}

/// A [`SystemWithAccess`] stored in a [`ScheduleGraph`].
pub struct SystemNode {
    inner: Option<SystemWithAccess>,
}

/// A [`ScheduleSystem`] stored alongside the access returned from [`System::initialize`](crate::system::System::initialize).
pub struct SystemWithAccess {
    /// The system itself.
    pub system: ScheduleSystem,
    /// The access returned by [`System::initialize`](crate::system::System::initialize).
    /// This will be empty if the system has not been initialized yet.
    pub access: FilteredAccessSet<ComponentId>,
}

impl SystemWithAccess {
    /// Constructs a new [`SystemWithAccess`] from a [`ScheduleSystem`].
    /// The `access` will initially be empty.
    pub fn new(system: ScheduleSystem) -> Self {
        Self {
            system,
            access: FilteredAccessSet::new(),
        }
    }
}

/// A [`BoxedCondition`] stored alongside the access returned from [`System::initialize`](crate::system::System::initialize).
pub struct ConditionWithAccess {
    /// The condition itself.
    pub condition: BoxedCondition,
    /// The access returned by [`System::initialize`](crate::system::System::initialize).
    /// This will be empty if the system has not been initialized yet.
    pub access: FilteredAccessSet<ComponentId>,
}

impl ConditionWithAccess {
    /// Constructs a new [`ConditionWithAccess`] from a [`BoxedCondition`].
    /// The `access` will initially be empty.
    pub const fn new(condition: BoxedCondition) -> Self {
        Self {
            condition,
            access: FilteredAccessSet::new(),
        }
    }
}

impl SystemNode {
    /// Create a new [`SystemNode`]
    pub fn new(system: ScheduleSystem) -> Self {
        Self {
            inner: Some(SystemWithAccess::new(system)),
        }
    }

    /// Obtain a reference to the [`SystemWithAccess`] represented by this node.
    pub fn get(&self) -> Option<&SystemWithAccess> {
        self.inner.as_ref()
    }

    /// Obtain a mutable reference to the [`SystemWithAccess`] represented by this node.
    pub fn get_mut(&mut self) -> Option<&mut SystemWithAccess> {
        self.inner.as_mut()
    }
}

new_key_type! {
    /// A unique identifier for a system in a [`ScheduleGraph`].
    pub struct SystemKey;
    /// A unique identifier for a system set in a [`ScheduleGraph`].
    pub struct SystemSetKey;
}

enum UninitializedId {
    System(SystemKey),
    Set {
        key: SystemSetKey,
        first_uninit_condition: usize,
    },
}

/// Metadata for a [`Schedule`].
///
/// The order isn't optimized; calling `ScheduleGraph::build_schedule` will return a
/// `SystemSchedule` where the order is optimized for execution.
#[derive(Default)]
pub struct ScheduleGraph {
    /// List of systems in the schedule
    pub systems: SlotMap<SystemKey, SystemNode>,
    /// List of conditions for each system, in the same order as `systems`
    pub system_conditions: SecondaryMap<SystemKey, Vec<ConditionWithAccess>>,
    /// List of system sets in the schedule
    system_sets: SlotMap<SystemSetKey, SystemSetNode>,
    /// List of conditions for each system set, in the same order as `system_sets`
    system_set_conditions: SecondaryMap<SystemSetKey, Vec<ConditionWithAccess>>,
    /// Map from system set to node id
    system_set_ids: HashMap<InternedSystemSet, SystemSetKey>,
    /// Systems that have not been initialized yet; for system sets, we store the index of the first uninitialized condition
    /// (all the conditions after that index still need to be initialized)
    uninit: Vec<UninitializedId>,
    /// Directed acyclic graph of the hierarchy (which systems/sets are children of which sets)
    hierarchy: Dag,
    /// Directed acyclic graph of the dependency (which systems/sets have to run before which other systems/sets)
    dependency: Dag,
    ambiguous_with: UnGraph,
    /// Nodes that are allowed to have ambiguous ordering relationship with any other systems.
    pub ambiguous_with_all: HashSet<NodeId>,
    conflicting_systems: Vec<(SystemKey, SystemKey, Vec<ComponentId>)>,
    anonymous_sets: usize,
    changed: bool,
    settings: ScheduleBuildSettings,

    passes: BTreeMap<TypeId, Box<dyn ScheduleBuildPassObj>>,
}

impl ScheduleGraph {
    /// Creates an empty [`ScheduleGraph`] with default settings.
    pub fn new() -> Self {
        Self {
            systems: SlotMap::with_key(),
            system_conditions: SecondaryMap::new(),
            system_sets: SlotMap::with_key(),
            system_set_conditions: SecondaryMap::new(),
            system_set_ids: HashMap::default(),
            uninit: Vec::new(),
            hierarchy: Dag::new(),
            dependency: Dag::new(),
            ambiguous_with: UnGraph::default(),
            ambiguous_with_all: HashSet::default(),
            conflicting_systems: Vec::new(),
            anonymous_sets: 0,
            changed: false,
            settings: default(),
            passes: default(),
        }
    }

    /// Returns the system at the given [`SystemKey`], if it exists.
    pub fn get_system_at(&self, key: SystemKey) -> Option<&ScheduleSystem> {
        self.systems
            .get(key)
            .and_then(|system| system.get())
            .map(|system| &system.system)
    }

    /// Returns `true` if the given system set is part of the graph. Otherwise, returns `false`.
    pub fn contains_set(&self, set: impl SystemSet) -> bool {
        self.system_set_ids.contains_key(&set.intern())
    }

    /// Returns the system at the given [`NodeId`].
    ///
    /// Panics if it doesn't exist.
    #[track_caller]
    pub fn system_at(&self, key: SystemKey) -> &ScheduleSystem {
        self.get_system_at(key)
            .unwrap_or_else(|| panic!("system with key {key:?} does not exist in this Schedule"))
    }

    /// Returns the set at the given [`NodeId`], if it exists.
    pub fn get_set_at(&self, key: SystemSetKey) -> Option<&dyn SystemSet> {
        self.system_sets.get(key).map(|set| &*set.inner)
    }

    /// Returns the set at the given [`NodeId`].
    ///
    /// Panics if it doesn't exist.
    #[track_caller]
    pub fn set_at(&self, id: SystemSetKey) -> &dyn SystemSet {
        self.get_set_at(id)
            .unwrap_or_else(|| panic!("set with id {id:?} does not exist in this Schedule"))
    }

    /// Returns the conditions for the set at the given [`SystemSetKey`], if it exists.
    pub fn get_set_conditions_at(&self, key: SystemSetKey) -> Option<&[ConditionWithAccess]> {
        self.system_set_conditions.get(key).map(Vec::as_slice)
    }

    /// Returns the conditions for the set at the given [`SystemSetKey`].
    ///
    /// Panics if it doesn't exist.
    #[track_caller]
    pub fn set_conditions_at(&self, key: SystemSetKey) -> &[ConditionWithAccess] {
        self.get_set_conditions_at(key)
            .unwrap_or_else(|| panic!("set with key {key:?} does not exist in this Schedule"))
    }

    /// Returns an iterator over all systems in this schedule, along with the conditions for each system.
    pub fn systems(
        &self,
    ) -> impl Iterator<Item = (SystemKey, &ScheduleSystem, &[ConditionWithAccess])> {
        self.systems.iter().filter_map(|(key, system_node)| {
            let system = &system_node.inner.as_ref()?.system;
            let conditions = self.system_conditions.get(key)?;
            Some((key, system, conditions.as_slice()))
        })
    }

    /// Returns an iterator over all system sets in this schedule, along with the conditions for each
    /// system set.
    pub fn system_sets(
        &self,
    ) -> impl Iterator<Item = (SystemSetKey, &dyn SystemSet, &[ConditionWithAccess])> {
        self.system_sets.iter().filter_map(|(key, set_node)| {
            let set = &*set_node.inner;
            let conditions = self.system_set_conditions.get(key)?.as_slice();
            Some((key, set, conditions))
        })
    }

    /// Returns the [`Dag`] of the hierarchy.
    ///
    /// The hierarchy is a directed acyclic graph of the systems and sets,
    /// where an edge denotes that a system or set is the child of another set.
    pub fn hierarchy(&self) -> &Dag {
        &self.hierarchy
    }

    /// Returns the [`Dag`] of the dependencies in the schedule.
    ///
    /// Nodes in this graph are systems and sets, and edges denote that
    /// a system or set has to run before another system or set.
    pub fn dependency(&self) -> &Dag {
        &self.dependency
    }

    /// Returns the list of systems that conflict with each other, i.e. have ambiguities in their access.
    ///
    /// If the `Vec<ComponentId>` is empty, the systems conflict on [`World`] access.
    /// Must be called after [`ScheduleGraph::build_schedule`] to be non-empty.
    pub fn conflicting_systems(&self) -> &[(SystemKey, SystemKey, Vec<ComponentId>)] {
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
                self.configure_set_inner(set_config).unwrap();
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

                        for previous_node in previous_nodes {
                            for current_node in current_nodes {
                                self.dependency
                                    .graph
                                    .add_edge(*previous_node, *current_node);

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
    fn add_system_inner(
        &mut self,
        config: ScheduleConfig<ScheduleSystem>,
    ) -> Result<NodeId, ScheduleBuildError> {
        let key = self.systems.insert(SystemNode::new(config.node));
        self.system_conditions.insert(
            key,
            config
                .conditions
                .into_iter()
                .map(ConditionWithAccess::new)
                .collect(),
        );
        // system init has to be deferred (need `&mut World`)
        self.uninit.push(UninitializedId::System(key));

        // graph updates are immediate
        self.update_graphs(NodeId::System(key), config.metadata)?;

        Ok(NodeId::System(key))
    }

    #[track_caller]
    fn configure_sets<M>(&mut self, sets: impl IntoScheduleConfigs<InternedSystemSet, M>) {
        self.process_configs(sets.into_configs(), false);
    }

    /// Add a single `ScheduleConfig` to the graph, including its dependencies and conditions.
    fn configure_set_inner(
        &mut self,
        set: ScheduleConfig<InternedSystemSet>,
    ) -> Result<NodeId, ScheduleBuildError> {
        let ScheduleConfig {
            node: set,
            metadata,
            conditions,
        } = set;

        let key = match self.system_set_ids.get(&set) {
            Some(&id) => id,
            None => self.add_set(set),
        };

        // graph updates are immediate
        self.update_graphs(NodeId::Set(key), metadata)?;

        // system init has to be deferred (need `&mut World`)
        let system_set_conditions = self.system_set_conditions.entry(key).unwrap().or_default();
        self.uninit.push(UninitializedId::Set {
            key,
            first_uninit_condition: system_set_conditions.len(),
        });
        system_set_conditions.extend(conditions.into_iter().map(ConditionWithAccess::new));

        Ok(NodeId::Set(key))
    }

    fn add_set(&mut self, set: InternedSystemSet) -> SystemSetKey {
        let key = self.system_sets.insert(SystemSetNode::new(set));
        self.system_set_conditions.insert(key, Vec::new());
        self.system_set_ids.insert(set, key);
        key
    }

    fn create_anonymous_set(&mut self) -> AnonymousSet {
        let id = self.anonymous_sets;
        self.anonymous_sets += 1;
        AnonymousSet::new(id)
    }

    /// Check that no set is included in itself.
    /// Add all the sets from the [`GraphInfo`]'s hierarchy to the graph.
    fn check_hierarchy_sets(
        &mut self,
        id: NodeId,
        graph_info: &GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        for &set in &graph_info.hierarchy {
            if let Some(&set_id) = self.system_set_ids.get(&set) {
                if let NodeId::Set(key) = id
                    && set_id == key
                {
                    {
                        return Err(ScheduleBuildError::HierarchyLoop(
                            self.get_node_name(&NodeId::Set(key)),
                        ));
                    }
                }
            } else {
                // If the set is not in the graph, we add it
                self.add_set(set);
            }
        }

        Ok(())
    }

    /// Checks that no system set is dependent on itself.
    /// Add all the sets from the [`GraphInfo`]'s dependencies to the graph.
    fn check_edges(
        &mut self,
        id: NodeId,
        graph_info: &GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        for Dependency { set, .. } in &graph_info.dependencies {
            if let Some(&set_id) = self.system_set_ids.get(set) {
                if let NodeId::Set(key) = id
                    && set_id == key
                {
                    return Err(ScheduleBuildError::DependencyLoop(
                        self.get_node_name(&NodeId::Set(key)),
                    ));
                }
            } else {
                // If the set is not in the graph, we add it
                self.add_set(*set);
            }
        }

        Ok(())
    }

    /// Add all the sets from the [`GraphInfo`]'s ambiguity to the graph.
    fn add_ambiguities(&mut self, graph_info: &GraphInfo) {
        if let Ambiguity::IgnoreWithSet(ambiguous_with) = &graph_info.ambiguous_with {
            for set in ambiguous_with {
                if !self.system_set_ids.contains_key(set) {
                    self.add_set(*set);
                }
            }
        }
    }

    /// Update the internal graphs (hierarchy, dependency, ambiguity) by adding a single [`GraphInfo`]
    fn update_graphs(
        &mut self,
        id: NodeId,
        graph_info: GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        self.check_hierarchy_sets(id, &graph_info)?;
        self.check_edges(id, &graph_info)?;
        self.add_ambiguities(&graph_info);
        self.changed = true;

        let GraphInfo {
            hierarchy: sets,
            dependencies,
            ambiguous_with,
            ..
        } = graph_info;

        self.hierarchy.graph.add_node(id);
        self.dependency.graph.add_node(id);

        for key in sets.into_iter().map(|set| self.system_set_ids[&set]) {
            self.hierarchy.graph.add_edge(NodeId::Set(key), id);

            // ensure set also appears in dependency graph
            self.dependency.graph.add_node(NodeId::Set(key));
        }

        for (kind, key, options) in dependencies
            .into_iter()
            .map(|Dependency { kind, set, options }| (kind, self.system_set_ids[&set], options))
        {
            let (lhs, rhs) = match kind {
                DependencyKind::Before => (id, NodeId::Set(key)),
                DependencyKind::After => (NodeId::Set(key), id),
            };
            self.dependency.graph.add_edge(lhs, rhs);
            for pass in self.passes.values_mut() {
                pass.add_dependency(lhs, rhs, &options);
            }

            // ensure set also appears in hierarchy graph
            self.hierarchy.graph.add_node(NodeId::Set(key));
        }

        match ambiguous_with {
            Ambiguity::Check => (),
            Ambiguity::IgnoreWithSet(ambiguous_with) => {
                for key in ambiguous_with
                    .into_iter()
                    .map(|set| self.system_set_ids[&set])
                {
                    self.ambiguous_with.add_edge(id, NodeId::Set(key));
                }
            }
            Ambiguity::IgnoreAll => {
                self.ambiguous_with_all.insert(id);
            }
        }

        Ok(())
    }

    /// Initializes any newly-added systems and conditions by calling [`System::initialize`](crate::system::System)
    pub fn initialize(&mut self, world: &mut World) {
        for id in self.uninit.drain(..) {
            match id {
                UninitializedId::System(key) => {
                    let system = self.systems[key].get_mut().unwrap();
                    system.access = system.system.initialize(world);
                    for condition in &mut self.system_conditions[key] {
                        condition.access = condition.condition.initialize(world);
                    }
                }
                UninitializedId::Set {
                    key,
                    first_uninit_condition,
                } => {
                    for condition in self.system_set_conditions[key]
                        .iter_mut()
                        .skip(first_uninit_condition)
                    {
                        condition.access = condition.condition.initialize(world);
                    }
                }
            }
        }
    }

    /// Build a [`SystemSchedule`] optimized for scheduler access from the [`ScheduleGraph`].
    ///
    /// This method also
    /// - checks for dependency or hierarchy cycles
    /// - checks for system access conflicts and reports ambiguities
    pub fn build_schedule(
        &mut self,
        world: &mut World,
        schedule_label: InternedScheduleLabel,
        ignored_ambiguities: &BTreeSet<ComponentId>,
    ) -> Result<SystemSchedule, ScheduleBuildError> {
        // check hierarchy for cycles
        self.hierarchy.topsort =
            self.topsort_graph(&self.hierarchy.graph, ReportCycles::Hierarchy)?;

        let hier_results = check_graph(&self.hierarchy.graph, &self.hierarchy.topsort);
        self.optionally_check_hierarchy_conflicts(&hier_results.transitive_edges, schedule_label)?;

        // remove redundant edges
        self.hierarchy.graph = hier_results.transitive_reduction;

        // check dependencies for cycles
        self.dependency.topsort =
            self.topsort_graph(&self.dependency.graph, ReportCycles::Dependency)?;

        // check for systems or system sets depending on sets they belong to
        let dep_results = check_graph(&self.dependency.graph, &self.dependency.topsort);
        self.check_for_cross_dependencies(&dep_results, &hier_results.connected)?;

        // map all system sets to their systems
        // go in reverse topological order (bottom-up) for efficiency
        let (set_systems, set_system_bitsets) =
            self.map_sets_to_systems(&self.hierarchy.topsort, &self.hierarchy.graph);
        self.check_order_but_intersect(&dep_results.connected, &set_system_bitsets)?;

        // check that there are no edges to system-type sets that have multiple instances
        self.check_system_type_set_ambiguity(&set_systems)?;

        let mut dependency_flattened = self.get_dependency_flattened(&set_systems);

        // modify graph with build passes
        let mut passes = core::mem::take(&mut self.passes);
        for pass in passes.values_mut() {
            pass.build(world, self, &mut dependency_flattened)?;
        }
        self.passes = passes;

        // topsort
        let mut dependency_flattened_dag = Dag {
            topsort: self.topsort_graph(&dependency_flattened, ReportCycles::Dependency)?,
            graph: dependency_flattened,
        };

        let flat_results = check_graph(
            &dependency_flattened_dag.graph,
            &dependency_flattened_dag.topsort,
        );

        // remove redundant edges
        dependency_flattened_dag.graph = flat_results.transitive_reduction;

        // flatten: combine `in_set` with `ambiguous_with` information
        let ambiguous_with_flattened = self.get_ambiguous_with_flattened(&set_systems);

        // check for conflicts
        let conflicting_systems = self.get_conflicting_systems(
            &flat_results.disconnected,
            &ambiguous_with_flattened,
            ignored_ambiguities,
        );
        self.optionally_check_conflicts(&conflicting_systems, world.components(), schedule_label)?;
        self.conflicting_systems = conflicting_systems;

        // build the schedule
        Ok(self.build_schedule_inner(dependency_flattened_dag, hier_results.reachable))
    }

    /// Return a map from system set `NodeId` to a list of system `NodeId`s that are included in the set.
    /// Also return a map from system set `NodeId` to a `FixedBitSet` of system `NodeId`s that are included in the set,
    /// where the bitset order is the same as `self.systems`
    fn map_sets_to_systems(
        &self,
        hierarchy_topsort: &[NodeId],
        hierarchy_graph: &DiGraph,
    ) -> (
        HashMap<SystemSetKey, Vec<SystemKey>>,
        HashMap<SystemSetKey, HashSet<SystemKey>>,
    ) {
        let mut set_systems: HashMap<SystemSetKey, Vec<SystemKey>> =
            HashMap::with_capacity_and_hasher(self.system_sets.len(), Default::default());
        let mut set_system_sets: HashMap<SystemSetKey, HashSet<SystemKey>> =
            HashMap::with_capacity_and_hasher(self.system_sets.len(), Default::default());
        for &id in hierarchy_topsort.iter().rev() {
            let NodeId::Set(set_key) = id else {
                continue;
            };

            let mut systems = Vec::new();
            let mut system_set = HashSet::with_capacity(self.systems.len());

            for child in hierarchy_graph.neighbors_directed(id, Outgoing) {
                match child {
                    NodeId::System(key) => {
                        systems.push(key);
                        system_set.insert(key);
                    }
                    NodeId::Set(key) => {
                        let child_systems = set_systems.get(&key).unwrap();
                        let child_system_set = set_system_sets.get(&key).unwrap();
                        systems.extend_from_slice(child_systems);
                        system_set.extend(child_system_set.iter());
                    }
                }
            }

            set_systems.insert(set_key, systems);
            set_system_sets.insert(set_key, system_set);
        }
        (set_systems, set_system_sets)
    }

    fn get_dependency_flattened(
        &mut self,
        set_systems: &HashMap<SystemSetKey, Vec<SystemKey>>,
    ) -> DiGraph {
        // flatten: combine `in_set` with `before` and `after` information
        // have to do it like this to preserve transitivity
        let mut dependency_flattened = self.dependency.graph.clone();
        let mut temp = Vec::new();
        for (&set, systems) in set_systems {
            for pass in self.passes.values_mut() {
                pass.collapse_set(set, systems, &dependency_flattened, &mut temp);
            }
            if systems.is_empty() {
                // collapse dependencies for empty sets
                for a in dependency_flattened.neighbors_directed(NodeId::Set(set), Incoming) {
                    for b in dependency_flattened.neighbors_directed(NodeId::Set(set), Outgoing) {
                        temp.push((a, b));
                    }
                }
            } else {
                for a in dependency_flattened.neighbors_directed(NodeId::Set(set), Incoming) {
                    for &sys in systems {
                        temp.push((a, NodeId::System(sys)));
                    }
                }

                for b in dependency_flattened.neighbors_directed(NodeId::Set(set), Outgoing) {
                    for &sys in systems {
                        temp.push((NodeId::System(sys), b));
                    }
                }
            }

            dependency_flattened.remove_node(NodeId::Set(set));
            for (a, b) in temp.drain(..) {
                dependency_flattened.add_edge(a, b);
            }
        }

        dependency_flattened
    }

    fn get_ambiguous_with_flattened(
        &self,
        set_systems: &HashMap<SystemSetKey, Vec<SystemKey>>,
    ) -> UnGraph {
        let mut ambiguous_with_flattened = UnGraph::default();
        for (lhs, rhs) in self.ambiguous_with.all_edges() {
            match (lhs, rhs) {
                (NodeId::System(_), NodeId::System(_)) => {
                    ambiguous_with_flattened.add_edge(lhs, rhs);
                }
                (NodeId::Set(lhs), NodeId::System(_)) => {
                    for &lhs_ in set_systems.get(&lhs).unwrap_or(&Vec::new()) {
                        ambiguous_with_flattened.add_edge(NodeId::System(lhs_), rhs);
                    }
                }
                (NodeId::System(_), NodeId::Set(rhs)) => {
                    for &rhs_ in set_systems.get(&rhs).unwrap_or(&Vec::new()) {
                        ambiguous_with_flattened.add_edge(lhs, NodeId::System(rhs_));
                    }
                }
                (NodeId::Set(lhs), NodeId::Set(rhs)) => {
                    for &lhs_ in set_systems.get(&lhs).unwrap_or(&Vec::new()) {
                        for &rhs_ in set_systems.get(&rhs).unwrap_or(&vec![]) {
                            ambiguous_with_flattened
                                .add_edge(NodeId::System(lhs_), NodeId::System(rhs_));
                        }
                    }
                }
            }
        }

        ambiguous_with_flattened
    }

    fn get_conflicting_systems(
        &self,
        flat_results_disconnected: &Vec<(NodeId, NodeId)>,
        ambiguous_with_flattened: &UnGraph,
        ignored_ambiguities: &BTreeSet<ComponentId>,
    ) -> Vec<(SystemKey, SystemKey, Vec<ComponentId>)> {
        let mut conflicting_systems = Vec::new();
        for &(a, b) in flat_results_disconnected {
            if ambiguous_with_flattened.contains_edge(a, b)
                || self.ambiguous_with_all.contains(&a)
                || self.ambiguous_with_all.contains(&b)
            {
                continue;
            }

            let NodeId::System(a) = a else {
                panic!(
                    "Encountered a non-system node in the flattened disconnected results: {a:?}"
                );
            };
            let NodeId::System(b) = b else {
                panic!(
                    "Encountered a non-system node in the flattened disconnected results: {b:?}"
                );
            };
            let system_a = self.systems[a].get().unwrap();
            let system_b = self.systems[b].get().unwrap();
            if system_a.system.is_exclusive() || system_b.system.is_exclusive() {
                conflicting_systems.push((a, b, Vec::new()));
            } else {
                let access_a = &system_a.access;
                let access_b = &system_b.access;
                if !access_a.is_compatible(access_b) {
                    match access_a.get_conflicts(access_b) {
                        AccessConflicts::Individual(conflicts) => {
                            let conflicts: Vec<_> = conflicts
                                .ones()
                                .map(ComponentId::get_sparse_set_index)
                                .filter(|id| !ignored_ambiguities.contains(id))
                                .collect();
                            if !conflicts.is_empty() {
                                conflicting_systems.push((a, b, conflicts));
                            }
                        }
                        AccessConflicts::All => {
                            // there is no specific component conflicting, but the systems are overall incompatible
                            // for example 2 systems with `Query<EntityMut>`
                            conflicting_systems.push((a, b, Vec::new()));
                        }
                    }
                }
            }
        }

        conflicting_systems
    }

    fn build_schedule_inner(
        &self,
        dependency_flattened_dag: Dag,
        hier_results_reachable: FixedBitSet,
    ) -> SystemSchedule {
        let dg_system_ids = dependency_flattened_dag
            .topsort
            .iter()
            .filter_map(NodeId::as_system)
            .collect::<Vec<_>>();
        let dg_system_idx_map = dg_system_ids
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect::<HashMap<_, _>>();

        let hg_systems = self
            .hierarchy
            .topsort
            .iter()
            .cloned()
            .enumerate()
            .filter_map(|(i, id)| Some((i, id.as_system()?)))
            .collect::<Vec<_>>();

        let (hg_set_with_conditions_idxs, hg_set_ids): (Vec<_>, Vec<_>) = self
            .hierarchy
            .topsort
            .iter()
            .cloned()
            .enumerate()
            .filter_map(|(i, id)| {
                // ignore system sets that have no conditions
                // ignore system type sets (already covered, they don't have conditions)
                let key = id.as_set()?;
                (!self.system_set_conditions[key].is_empty()).then_some((i, key))
            })
            .unzip();

        let sys_count = self.systems.len();
        let set_with_conditions_count = hg_set_ids.len();
        let hg_node_count = self.hierarchy.graph.node_count();

        // get the number of dependencies and the immediate dependents of each system
        // (needed by multi_threaded executor to run systems in the correct order)
        let mut system_dependencies = Vec::with_capacity(sys_count);
        let mut system_dependents = Vec::with_capacity(sys_count);
        for &sys_key in &dg_system_ids {
            let num_dependencies = dependency_flattened_dag
                .graph
                .neighbors_directed(NodeId::System(sys_key), Incoming)
                .count();

            let dependents = dependency_flattened_dag
                .graph
                .neighbors_directed(NodeId::System(sys_key), Outgoing)
                .filter_map(|dep_id| {
                    let dep_key = dep_id.as_system()?;
                    Some(dg_system_idx_map[&dep_key])
                })
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
                let is_descendant = hier_results_reachable[index(row, col, hg_node_count)];
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
                let is_ancestor = hier_results_reachable[index(row, col, hg_node_count)];
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
        ignored_ambiguities: &BTreeSet<ComponentId>,
        schedule_label: InternedScheduleLabel,
    ) -> Result<(), ScheduleBuildError> {
        if !self.uninit.is_empty() {
            return Err(ScheduleBuildError::Uninitialized);
        }

        // move systems out of old schedule
        for ((key, system), conditions) in schedule
            .system_ids
            .drain(..)
            .zip(schedule.systems.drain(..))
            .zip(schedule.system_conditions.drain(..))
        {
            self.systems[key].inner = Some(system);
            self.system_conditions[key] = conditions;
        }

        for (key, conditions) in schedule
            .set_ids
            .drain(..)
            .zip(schedule.set_conditions.drain(..))
        {
            self.system_set_conditions[key] = conditions;
        }

        *schedule = self.build_schedule(world, schedule_label, ignored_ambiguities)?;

        // move systems into new schedule
        for &key in &schedule.system_ids {
            let system = self.systems[key].inner.take().unwrap();
            let conditions = core::mem::take(&mut self.system_conditions[key]);
            schedule.systems.push(system);
            schedule.system_conditions.push(conditions);
        }

        for &key in &schedule.set_ids {
            let conditions = core::mem::take(&mut self.system_set_conditions[key]);
            schedule.set_conditions.push(conditions);
        }

        Ok(())
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
        schedule_graph.add_system_inner(config).unwrap()
    }
}

impl ProcessScheduleConfig for InternedSystemSet {
    fn process_config(schedule_graph: &mut ScheduleGraph, config: ScheduleConfig<Self>) -> NodeId {
        schedule_graph.configure_set_inner(config).unwrap()
    }
}

/// Used to select the appropriate reporting function.
pub enum ReportCycles {
    /// When sets contain themselves
    Hierarchy,
    /// When the graph is no longer a DAG
    Dependency,
}

// methods for reporting errors
impl ScheduleGraph {
    fn get_node_name(&self, id: &NodeId) -> String {
        self.get_node_name_inner(id, self.settings.report_sets)
    }

    #[inline]
    fn get_node_name_inner(&self, id: &NodeId, report_sets: bool) -> String {
        match *id {
            NodeId::System(key) => {
                let name = self.systems[key].get().unwrap().system.name();
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
                    set.name()
                }
            }
        }
    }

    fn anonymous_set_name(&self, id: &NodeId) -> String {
        format!(
            "({})",
            self.hierarchy
                .graph
                .edges_directed(*id, Outgoing)
                // never get the sets of the members or this will infinite recurse when the report_sets setting is on.
                .map(|(_, member_id)| self.get_node_name_inner(&member_id, false))
                .reduce(|a, b| format!("{a}, {b}"))
                .unwrap_or_default()
        )
    }

    fn get_node_kind(&self, id: &NodeId) -> &'static str {
        match id {
            NodeId::System(_) => "system",
            NodeId::Set(_) => "system set",
        }
    }

    /// If [`ScheduleBuildSettings::hierarchy_detection`] is [`LogLevel::Ignore`] this check
    /// is skipped.
    fn optionally_check_hierarchy_conflicts(
        &self,
        transitive_edges: &[(NodeId, NodeId)],
        schedule_label: InternedScheduleLabel,
    ) -> Result<(), ScheduleBuildError> {
        if self.settings.hierarchy_detection == LogLevel::Ignore || transitive_edges.is_empty() {
            return Ok(());
        }

        let message = self.get_hierarchy_conflicts_error_message(transitive_edges);
        match self.settings.hierarchy_detection {
            LogLevel::Ignore => unreachable!(),
            LogLevel::Warn => {
                error!("Schedule {schedule_label:?} has redundant edges:\n {message}");
                Ok(())
            }
            LogLevel::Error => Err(ScheduleBuildError::HierarchyRedundancy(message)),
        }
    }

    fn get_hierarchy_conflicts_error_message(
        &self,
        transitive_edges: &[(NodeId, NodeId)],
    ) -> String {
        let mut message = String::from("hierarchy contains redundant edge(s)");
        for (parent, child) in transitive_edges {
            writeln!(
                message,
                " -- {} `{}` cannot be child of set `{}`, longer path exists",
                self.get_node_kind(child),
                self.get_node_name(child),
                self.get_node_name(parent),
            )
            .unwrap();
        }

        message
    }

    /// Tries to topologically sort `graph`.
    ///
    /// If the graph is acyclic, returns [`Ok`] with the list of [`NodeId`] in a valid
    /// topological order. If the graph contains cycles, returns [`Err`] with the list of
    /// strongly-connected components that contain cycles (also in a valid topological order).
    ///
    /// # Errors
    ///
    /// If the graph contain cycles, then an error is returned.
    pub fn topsort_graph(
        &self,
        graph: &DiGraph,
        report: ReportCycles,
    ) -> Result<Vec<NodeId>, ScheduleBuildError> {
        // Tarjan's SCC algorithm returns elements in *reverse* topological order.
        let mut top_sorted_nodes = Vec::with_capacity(graph.node_count());
        let mut sccs_with_cycles = Vec::new();

        for scc in graph.iter_sccs() {
            // A strongly-connected component is a group of nodes who can all reach each other
            // through one or more paths. If an SCC contains more than one node, there must be
            // at least one cycle within them.
            top_sorted_nodes.extend_from_slice(&scc);
            if scc.len() > 1 {
                sccs_with_cycles.push(scc);
            }
        }

        if sccs_with_cycles.is_empty() {
            // reverse to get topological order
            top_sorted_nodes.reverse();
            Ok(top_sorted_nodes)
        } else {
            let mut cycles = Vec::new();
            for scc in &sccs_with_cycles {
                cycles.append(&mut simple_cycles_in_component(graph, scc));
            }

            let error = match report {
                ReportCycles::Hierarchy => ScheduleBuildError::HierarchyCycle(
                    self.get_hierarchy_cycles_error_message(&cycles),
                ),
                ReportCycles::Dependency => ScheduleBuildError::DependencyCycle(
                    self.get_dependency_cycles_error_message(&cycles),
                ),
            };

            Err(error)
        }
    }

    /// Logs details of cycles in the hierarchy graph.
    fn get_hierarchy_cycles_error_message(&self, cycles: &[Vec<NodeId>]) -> String {
        let mut message = format!("schedule has {} in_set cycle(s):\n", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            let mut names = cycle.iter().map(|id| self.get_node_name(id));
            let first_name = names.next().unwrap();
            writeln!(
                message,
                "cycle {}: set `{first_name}` contains itself",
                i + 1,
            )
            .unwrap();
            writeln!(message, "set `{first_name}`").unwrap();
            for name in names.chain(core::iter::once(first_name)) {
                writeln!(message, " ... which contains set `{name}`").unwrap();
            }
            writeln!(message).unwrap();
        }

        message
    }

    /// Logs details of cycles in the dependency graph.
    fn get_dependency_cycles_error_message(&self, cycles: &[Vec<NodeId>]) -> String {
        let mut message = format!("schedule has {} before/after cycle(s):\n", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            let mut names = cycle
                .iter()
                .map(|id| (self.get_node_kind(id), self.get_node_name(id)));
            let (first_kind, first_name) = names.next().unwrap();
            writeln!(
                message,
                "cycle {}: {first_kind} `{first_name}` must run before itself",
                i + 1,
            )
            .unwrap();
            writeln!(message, "{first_kind} `{first_name}`").unwrap();
            for (kind, name) in names.chain(core::iter::once((first_kind, first_name))) {
                writeln!(message, " ... which must run before {kind} `{name}`").unwrap();
            }
            writeln!(message).unwrap();
        }

        message
    }

    fn check_for_cross_dependencies(
        &self,
        dep_results: &CheckGraphResults,
        hier_results_connected: &HashSet<(NodeId, NodeId)>,
    ) -> Result<(), ScheduleBuildError> {
        for &(a, b) in &dep_results.connected {
            if hier_results_connected.contains(&(a, b)) || hier_results_connected.contains(&(b, a))
            {
                let name_a = self.get_node_name(&a);
                let name_b = self.get_node_name(&b);
                return Err(ScheduleBuildError::CrossDependency(name_a, name_b));
            }
        }

        Ok(())
    }

    fn check_order_but_intersect(
        &self,
        dep_results_connected: &HashSet<(NodeId, NodeId)>,
        set_system_sets: &HashMap<SystemSetKey, HashSet<SystemKey>>,
    ) -> Result<(), ScheduleBuildError> {
        // check that there is no ordering between system sets that intersect
        for (a, b) in dep_results_connected {
            let (NodeId::Set(a_key), NodeId::Set(b_key)) = (a, b) else {
                continue;
            };

            let a_systems = set_system_sets.get(a_key).unwrap();
            let b_systems = set_system_sets.get(b_key).unwrap();

            if !a_systems.is_disjoint(b_systems) {
                return Err(ScheduleBuildError::SetsHaveOrderButIntersect(
                    self.get_node_name(a),
                    self.get_node_name(b),
                ));
            }
        }

        Ok(())
    }

    fn check_system_type_set_ambiguity(
        &self,
        set_systems: &HashMap<SystemSetKey, Vec<SystemKey>>,
    ) -> Result<(), ScheduleBuildError> {
        for (&key, systems) in set_systems {
            let set = &self.system_sets[key];
            if set.is_system_type() {
                let instances = systems.len();
                let ambiguous_with = self.ambiguous_with.edges(NodeId::Set(key));
                let before = self
                    .dependency
                    .graph
                    .edges_directed(NodeId::Set(key), Incoming);
                let after = self
                    .dependency
                    .graph
                    .edges_directed(NodeId::Set(key), Outgoing);
                let relations = before.count() + after.count() + ambiguous_with.count();
                if instances > 1 && relations > 0 {
                    return Err(ScheduleBuildError::SystemTypeSetAmbiguity(
                        self.get_node_name(&NodeId::Set(key)),
                    ));
                }
            }
        }
        Ok(())
    }

    /// if [`ScheduleBuildSettings::ambiguity_detection`] is [`LogLevel::Ignore`], this check is skipped
    fn optionally_check_conflicts(
        &self,
        conflicts: &[(SystemKey, SystemKey, Vec<ComponentId>)],
        components: &Components,
        schedule_label: InternedScheduleLabel,
    ) -> Result<(), ScheduleBuildError> {
        if self.settings.ambiguity_detection == LogLevel::Ignore || conflicts.is_empty() {
            return Ok(());
        }

        let message = self.get_conflicts_error_message(conflicts, components);
        match self.settings.ambiguity_detection {
            LogLevel::Ignore => Ok(()),
            LogLevel::Warn => {
                warn!("Schedule {schedule_label:?} has ambiguities.\n{message}");
                Ok(())
            }
            LogLevel::Error => Err(ScheduleBuildError::Ambiguity(message)),
        }
    }

    fn get_conflicts_error_message(
        &self,
        ambiguities: &[(SystemKey, SystemKey, Vec<ComponentId>)],
        components: &Components,
    ) -> String {
        let n_ambiguities = ambiguities.len();

        let mut message = format!(
                "{n_ambiguities} pairs of systems with conflicting data access have indeterminate execution order. \
                Consider adding `before`, `after`, or `ambiguous_with` relationships between these:\n",
            );

        for (name_a, name_b, conflicts) in self.conflicts_to_string(ambiguities, components) {
            writeln!(message, " -- {name_a} and {name_b}").unwrap();

            if !conflicts.is_empty() {
                writeln!(message, "    conflict on: {conflicts:?}").unwrap();
            } else {
                // one or both systems must be exclusive
                let world = core::any::type_name::<World>();
                writeln!(message, "    conflict on: {world}").unwrap();
            }
        }

        message
    }

    /// convert conflicts to human readable format
    pub fn conflicts_to_string<'a>(
        &'a self,
        ambiguities: &'a [(SystemKey, SystemKey, Vec<ComponentId>)],
        components: &'a Components,
    ) -> impl Iterator<Item = (String, String, Vec<DebugName>)> + 'a {
        ambiguities
            .iter()
            .map(move |(system_a, system_b, conflicts)| {
                let name_a = self.get_node_name(&NodeId::System(*system_a));
                let name_b = self.get_node_name(&NodeId::System(*system_b));

                let conflict_names: Vec<_> = conflicts
                    .iter()
                    .map(|id| components.get_name(*id).unwrap())
                    .collect();

                (name_a, name_b, conflict_names)
            })
    }

    fn traverse_sets_containing_node(&self, id: NodeId, f: &mut impl FnMut(SystemSetKey) -> bool) {
        for (set_id, _) in self.hierarchy.graph.edges_directed(id, Incoming) {
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
            !self.system_sets[key].is_system_type() && sets.insert(key)
        });
        let mut sets: Vec<_> = sets
            .into_iter()
            .map(|key| self.get_node_name(&NodeId::Set(key)))
            .collect();
        sets.sort();
        sets
    }
}

/// Category of errors encountered during schedule construction.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ScheduleBuildError {
    /// A system set contains itself.
    #[error("System set `{0}` contains itself.")]
    HierarchyLoop(String),
    /// The hierarchy of system sets contains a cycle.
    #[error("System set hierarchy contains cycle(s).\n{0}")]
    HierarchyCycle(String),
    /// The hierarchy of system sets contains redundant edges.
    ///
    /// This error is disabled by default, but can be opted-in using [`ScheduleBuildSettings`].
    #[error("System set hierarchy contains redundant edges.\n{0}")]
    HierarchyRedundancy(String),
    /// A system (set) has been told to run before itself.
    #[error("System set `{0}` depends on itself.")]
    DependencyLoop(String),
    /// The dependency graph contains a cycle.
    #[error("System dependencies contain cycle(s).\n{0}")]
    DependencyCycle(String),
    /// Tried to order a system (set) relative to a system set it belongs to.
    #[error("`{0}` and `{1}` have both `in_set` and `before`-`after` relationships (these might be transitive). This combination is unsolvable as a system cannot run before or after a set it belongs to.")]
    CrossDependency(String, String),
    /// Tried to order system sets that share systems.
    #[error("`{0}` and `{1}` have a `before`-`after` relationship (which may be transitive) but share systems.")]
    SetsHaveOrderButIntersect(String, String),
    /// Tried to order a system (set) relative to all instances of some system function.
    #[error("Tried to order against `{0}` in a schedule that has more than one `{0}` instance. `{0}` is a `SystemTypeSet` and cannot be used for ordering if ambiguous. Use a different set without this restriction.")]
    SystemTypeSetAmbiguity(String),
    /// Systems with conflicting access have indeterminate run order.
    ///
    /// This error is disabled by default, but can be opted-in using [`ScheduleBuildSettings`].
    #[error("Systems with conflicting access have indeterminate run order.\n{0}")]
    Ambiguity(String),
    /// Tried to run a schedule before all of its systems have been initialized.
    #[error("Systems in schedule have not been initialized.")]
    Uninitialized,
}

/// Specifies how schedule construction should respond to detecting a certain kind of issue.
#[derive(Debug, Clone, PartialEq)]
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
    /// is only logged or also results in an [`Ambiguity`](ScheduleBuildError::Ambiguity) error.
    ///
    /// Defaults to [`LogLevel::Ignore`].
    pub ambiguity_detection: LogLevel,
    /// Determines whether the presence of redundant edges in the hierarchy of system sets is only
    /// logged or also results in a [`HierarchyRedundancy`](ScheduleBuildError::HierarchyRedundancy)
    /// error.
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
    use bevy_ecs_macros::ScheduleLabel;

    use crate::{
        error::{ignore, panic, DefaultErrorHandler, Result},
        prelude::{ApplyDeferred, Res, Resource},
        schedule::{
            tests::ResMut, IntoScheduleConfigs, Schedule, ScheduleBuildSettings, SystemSet,
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
}
