#![expect(
    clippy::module_inception,
    reason = "This instance of module inception is being discussed; see #17344."
)]
use alloc::borrow::Cow;
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_utils::{default, TypeIdMap};
use core::{
    any::{Any, TypeId},
    fmt::{Debug, Write},
};
use disqualified::ShortName;
use fixedbitset::FixedBitSet;
use log::{error, info, warn};
use pass::ScheduleBuildPassObj;
use thiserror::Error;
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::{
    component::{ComponentId, Components, Tick},
    error::default_error_handler,
    prelude::Component,
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
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
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
            schedule.check_change_ticks(change_tick);
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

        info!("{}", message);
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
/// # Example
/// Here is an example of a `Schedule` running a "Hello world" system:
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

    /// Get the `InternedScheduleLabel` for this `Schedule`.
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

        self.graph.ambiguous_with.add_edge(a_id, b_id);

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
    pub fn set_build_settings(&mut self, settings: ScheduleBuildSettings) -> &mut Self {
        if settings.auto_insert_apply_deferred {
            self.add_build_pass(passes::AutoInsertApplyDeferredPass::default());
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

        let error_handler = default_error_handler();

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
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for system in &mut self.executable.systems {
            if !is_apply_deferred(system) {
                system.check_change_tick(change_tick);
            }
        }

        for conditions in &mut self.executable.system_conditions {
            for system in conditions {
                system.check_change_tick(change_tick);
            }
        }

        for conditions in &mut self.executable.set_conditions {
            for system in conditions {
                system.check_change_tick(change_tick);
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
        for system in &mut self.executable.systems {
            system.apply_deferred(world);
        }
    }

    /// Returns an iterator over all systems in this schedule.
    ///
    /// Note: this method will return [`ScheduleNotInitialized`] if the
    /// schedule has never been initialized or run.
    pub fn systems(
        &self,
    ) -> Result<impl Iterator<Item = (NodeId, &ScheduleSystem)> + Sized, ScheduleNotInitialized>
    {
        if !self.executor_initialized {
            return Err(ScheduleNotInitialized);
        }

        let iter = self
            .executable
            .system_ids
            .iter()
            .zip(&self.executable.systems)
            .map(|(node_id, system)| (*node_id, system));

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

/// A [`ScheduleSystem`] stored in a [`ScheduleGraph`].
pub struct SystemNode {
    inner: Option<ScheduleSystem>,
}

impl SystemNode {
    /// Create a new [`SystemNode`]
    pub fn new(system: ScheduleSystem) -> Self {
        Self {
            inner: Some(system),
        }
    }

    /// Obtain a reference to the [`ScheduleSystem`] represented by this node.
    pub fn get(&self) -> Option<&ScheduleSystem> {
        self.inner.as_ref()
    }

    /// Obtain a mutable reference to the [`ScheduleSystem`] represented by this node.
    pub fn get_mut(&mut self) -> Option<&mut ScheduleSystem> {
        self.inner.as_mut()
    }
}

/// Metadata for a [`Schedule`].
///
/// The order isn't optimized; calling `ScheduleGraph::build_schedule` will return a
/// `SystemSchedule` where the order is optimized for execution.
#[derive(Default)]
pub struct ScheduleGraph {
    /// List of systems in the schedule
    pub systems: Vec<SystemNode>,
    /// List of conditions for each system, in the same order as `systems`
    pub system_conditions: Vec<Vec<BoxedCondition>>,
    /// List of system sets in the schedule
    system_sets: Vec<SystemSetNode>,
    /// List of conditions for each system set, in the same order as `system_sets`
    system_set_conditions: Vec<Vec<BoxedCondition>>,
    /// Map from system set to node id
    system_set_ids: HashMap<InternedSystemSet, NodeId>,
    /// Systems that have not been initialized yet; for system sets, we store the index of the first uninitialized condition
    /// (all the conditions after that index still need to be initialized)
    uninit: Vec<(NodeId, usize)>,
    /// Directed acyclic graph of the hierarchy (which systems/sets are children of which sets)
    hierarchy: Dag,
    /// Directed acyclic graph of the dependency (which systems/sets have to run before which other systems/sets)
    dependency: Dag,
    ambiguous_with: UnGraph,
    /// Nodes that are allowed to have ambiguous ordering relationship with any other systems.
    pub ambiguous_with_all: HashSet<NodeId>,
    conflicting_systems: Vec<(NodeId, NodeId, Vec<ComponentId>)>,
    anonymous_sets: usize,
    changed: bool,
    settings: ScheduleBuildSettings,

    passes: BTreeMap<TypeId, Box<dyn ScheduleBuildPassObj>>,
}

impl ScheduleGraph {
    /// Creates an empty [`ScheduleGraph`] with default settings.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            system_conditions: Vec::new(),
            system_sets: Vec::new(),
            system_set_conditions: Vec::new(),
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

    /// Returns the system at the given [`NodeId`], if it exists.
    pub fn get_system_at(&self, id: NodeId) -> Option<&ScheduleSystem> {
        if !id.is_system() {
            return None;
        }
        self.systems
            .get(id.index())
            .and_then(|system| system.inner.as_ref())
    }

    /// Returns `true` if the given system set is part of the graph. Otherwise, returns `false`.
    pub fn contains_set(&self, set: impl SystemSet) -> bool {
        self.system_set_ids.contains_key(&set.intern())
    }

    /// Returns the system at the given [`NodeId`].
    ///
    /// Panics if it doesn't exist.
    #[track_caller]
    pub fn system_at(&self, id: NodeId) -> &ScheduleSystem {
        self.get_system_at(id)
            .ok_or_else(|| format!("system with id {id:?} does not exist in this Schedule"))
            .unwrap()
    }

    /// Returns the set at the given [`NodeId`], if it exists.
    pub fn get_set_at(&self, id: NodeId) -> Option<&dyn SystemSet> {
        if !id.is_set() {
            return None;
        }
        self.system_sets.get(id.index()).map(|set| &*set.inner)
    }

    /// Returns the set at the given [`NodeId`].
    ///
    /// Panics if it doesn't exist.
    #[track_caller]
    pub fn set_at(&self, id: NodeId) -> &dyn SystemSet {
        self.get_set_at(id)
            .ok_or_else(|| format!("set with id {id:?} does not exist in this Schedule"))
            .unwrap()
    }

    /// Returns the conditions for the set at the given [`NodeId`], if it exists.
    pub fn get_set_conditions_at(&self, id: NodeId) -> Option<&[BoxedCondition]> {
        if !id.is_set() {
            return None;
        }
        self.system_set_conditions
            .get(id.index())
            .map(Vec::as_slice)
    }

    /// Returns the conditions for the set at the given [`NodeId`].
    ///
    /// Panics if it doesn't exist.
    #[track_caller]
    pub fn set_conditions_at(&self, id: NodeId) -> &[BoxedCondition] {
        self.get_set_conditions_at(id)
            .ok_or_else(|| format!("set with id {id:?} does not exist in this Schedule"))
            .unwrap()
    }

    /// Returns an iterator over all systems in this schedule, along with the conditions for each system.
    pub fn systems(&self) -> impl Iterator<Item = (NodeId, &ScheduleSystem, &[BoxedCondition])> {
        self.systems
            .iter()
            .zip(self.system_conditions.iter())
            .enumerate()
            .filter_map(|(i, (system_node, condition))| {
                let system = system_node.inner.as_ref()?;
                Some((NodeId::System(i), system, condition.as_slice()))
            })
    }

    /// Returns an iterator over all system sets in this schedule, along with the conditions for each
    /// system set.
    pub fn system_sets(&self) -> impl Iterator<Item = (NodeId, &dyn SystemSet, &[BoxedCondition])> {
        self.system_set_ids.iter().map(|(_, &node_id)| {
            let set_node = &self.system_sets[node_id.index()];
            let set = &*set_node.inner;
            let conditions = self.system_set_conditions[node_id.index()].as_slice();
            (node_id, set, conditions)
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
    pub fn conflicting_systems(&self) -> &[(NodeId, NodeId, Vec<ComponentId>)] {
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
        let id = NodeId::System(self.systems.len());

        // graph updates are immediate
        self.update_graphs(id, config.metadata)?;

        // system init has to be deferred (need `&mut World`)
        self.uninit.push((id, 0));
        self.systems.push(SystemNode::new(config.node));
        self.system_conditions.push(config.conditions);

        Ok(id)
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
            mut conditions,
        } = set;

        let id = match self.system_set_ids.get(&set) {
            Some(&id) => id,
            None => self.add_set(set),
        };

        // graph updates are immediate
        self.update_graphs(id, metadata)?;

        // system init has to be deferred (need `&mut World`)
        let system_set_conditions = &mut self.system_set_conditions[id.index()];
        self.uninit.push((id, system_set_conditions.len()));
        system_set_conditions.append(&mut conditions);

        Ok(id)
    }

    fn add_set(&mut self, set: InternedSystemSet) -> NodeId {
        let id = NodeId::Set(self.system_sets.len());
        self.system_sets.push(SystemSetNode::new(set));
        self.system_set_conditions.push(Vec::new());
        self.system_set_ids.insert(set, id);
        id
    }

    /// Checks that a system set isn't included in itself.
    /// If not present, add the set to the graph.
    fn check_hierarchy_set(
        &mut self,
        id: &NodeId,
        set: InternedSystemSet,
    ) -> Result<(), ScheduleBuildError> {
        match self.system_set_ids.get(&set) {
            Some(set_id) => {
                if id == set_id {
                    return Err(ScheduleBuildError::HierarchyLoop(self.get_node_name(id)));
                }
            }
            None => {
                self.add_set(set);
            }
        }

        Ok(())
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
        id: &NodeId,
        graph_info: &GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        for &set in &graph_info.hierarchy {
            self.check_hierarchy_set(id, set)?;
        }

        Ok(())
    }

    /// Checks that no system set is dependent on itself.
    /// Add all the sets from the [`GraphInfo`]'s dependencies to the graph.
    fn check_edges(
        &mut self,
        id: &NodeId,
        graph_info: &GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        for Dependency { set, .. } in &graph_info.dependencies {
            match self.system_set_ids.get(set) {
                Some(set_id) => {
                    if id == set_id {
                        return Err(ScheduleBuildError::DependencyLoop(self.get_node_name(id)));
                    }
                }
                None => {
                    self.add_set(*set);
                }
            }
        }

        if let Ambiguity::IgnoreWithSet(ambiguous_with) = &graph_info.ambiguous_with {
            for set in ambiguous_with {
                if !self.system_set_ids.contains_key(set) {
                    self.add_set(*set);
                }
            }
        }

        Ok(())
    }

    /// Update the internal graphs (hierarchy, dependency, ambiguity) by adding a single [`GraphInfo`]
    fn update_graphs(
        &mut self,
        id: NodeId,
        graph_info: GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        self.check_hierarchy_sets(&id, &graph_info)?;
        self.check_edges(&id, &graph_info)?;
        self.changed = true;

        let GraphInfo {
            hierarchy: sets,
            dependencies,
            ambiguous_with,
            ..
        } = graph_info;

        self.hierarchy.graph.add_node(id);
        self.dependency.graph.add_node(id);

        for set in sets.into_iter().map(|set| self.system_set_ids[&set]) {
            self.hierarchy.graph.add_edge(set, id);

            // ensure set also appears in dependency graph
            self.dependency.graph.add_node(set);
        }

        for (kind, set, options) in dependencies
            .into_iter()
            .map(|Dependency { kind, set, options }| (kind, self.system_set_ids[&set], options))
        {
            let (lhs, rhs) = match kind {
                DependencyKind::Before => (id, set),
                DependencyKind::After => (set, id),
            };
            self.dependency.graph.add_edge(lhs, rhs);
            for pass in self.passes.values_mut() {
                pass.add_dependency(lhs, rhs, &options);
            }

            // ensure set also appears in hierarchy graph
            self.hierarchy.graph.add_node(set);
        }

        match ambiguous_with {
            Ambiguity::Check => (),
            Ambiguity::IgnoreWithSet(ambiguous_with) => {
                for set in ambiguous_with
                    .into_iter()
                    .map(|set| self.system_set_ids[&set])
                {
                    self.ambiguous_with.add_edge(id, set);
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
        for (id, i) in self.uninit.drain(..) {
            match id {
                NodeId::System(index) => {
                    self.systems[index].get_mut().unwrap().initialize(world);
                    for condition in &mut self.system_conditions[index] {
                        condition.initialize(world);
                    }
                }
                NodeId::Set(index) => {
                    for condition in self.system_set_conditions[index].iter_mut().skip(i) {
                        condition.initialize(world);
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
    ) -> (HashMap<NodeId, Vec<NodeId>>, HashMap<NodeId, FixedBitSet>) {
        let mut set_systems: HashMap<NodeId, Vec<NodeId>> =
            HashMap::with_capacity_and_hasher(self.system_sets.len(), Default::default());
        let mut set_system_bitsets =
            HashMap::with_capacity_and_hasher(self.system_sets.len(), Default::default());
        for &id in hierarchy_topsort.iter().rev() {
            if id.is_system() {
                continue;
            }

            let mut systems = Vec::new();
            let mut system_bitset = FixedBitSet::with_capacity(self.systems.len());

            for child in hierarchy_graph.neighbors_directed(id, Outgoing) {
                match child {
                    NodeId::System(_) => {
                        systems.push(child);
                        system_bitset.insert(child.index());
                    }
                    NodeId::Set(_) => {
                        let child_systems = set_systems.get(&child).unwrap();
                        let child_system_bitset = set_system_bitsets.get(&child).unwrap();
                        systems.extend_from_slice(child_systems);
                        system_bitset.union_with(child_system_bitset);
                    }
                }
            }

            set_systems.insert(id, systems);
            set_system_bitsets.insert(id, system_bitset);
        }
        (set_systems, set_system_bitsets)
    }

    fn get_dependency_flattened(&mut self, set_systems: &HashMap<NodeId, Vec<NodeId>>) -> DiGraph {
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
                for a in dependency_flattened.neighbors_directed(set, Incoming) {
                    for b in dependency_flattened.neighbors_directed(set, Outgoing) {
                        temp.push((a, b));
                    }
                }
            } else {
                for a in dependency_flattened.neighbors_directed(set, Incoming) {
                    for &sys in systems {
                        temp.push((a, sys));
                    }
                }

                for b in dependency_flattened.neighbors_directed(set, Outgoing) {
                    for &sys in systems {
                        temp.push((sys, b));
                    }
                }
            }

            dependency_flattened.remove_node(set);
            for (a, b) in temp.drain(..) {
                dependency_flattened.add_edge(a, b);
            }
        }

        dependency_flattened
    }

    fn get_ambiguous_with_flattened(&self, set_systems: &HashMap<NodeId, Vec<NodeId>>) -> UnGraph {
        let mut ambiguous_with_flattened = UnGraph::default();
        for (lhs, rhs) in self.ambiguous_with.all_edges() {
            match (lhs, rhs) {
                (NodeId::System(_), NodeId::System(_)) => {
                    ambiguous_with_flattened.add_edge(lhs, rhs);
                }
                (NodeId::Set(_), NodeId::System(_)) => {
                    for &lhs_ in set_systems.get(&lhs).unwrap_or(&Vec::new()) {
                        ambiguous_with_flattened.add_edge(lhs_, rhs);
                    }
                }
                (NodeId::System(_), NodeId::Set(_)) => {
                    for &rhs_ in set_systems.get(&rhs).unwrap_or(&Vec::new()) {
                        ambiguous_with_flattened.add_edge(lhs, rhs_);
                    }
                }
                (NodeId::Set(_), NodeId::Set(_)) => {
                    for &lhs_ in set_systems.get(&lhs).unwrap_or(&Vec::new()) {
                        for &rhs_ in set_systems.get(&rhs).unwrap_or(&vec![]) {
                            ambiguous_with_flattened.add_edge(lhs_, rhs_);
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
    ) -> Vec<(NodeId, NodeId, Vec<ComponentId>)> {
        let mut conflicting_systems = Vec::new();
        for &(a, b) in flat_results_disconnected {
            if ambiguous_with_flattened.contains_edge(a, b)
                || self.ambiguous_with_all.contains(&a)
                || self.ambiguous_with_all.contains(&b)
            {
                continue;
            }

            let system_a = self.systems[a.index()].get().unwrap();
            let system_b = self.systems[b.index()].get().unwrap();
            if system_a.is_exclusive() || system_b.is_exclusive() {
                conflicting_systems.push((a, b, Vec::new()));
            } else {
                let access_a = system_a.component_access();
                let access_b = system_b.component_access();
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
        let dg_system_ids = dependency_flattened_dag.topsort.clone();
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
            .filter(|&(_i, id)| id.is_system())
            .collect::<Vec<_>>();

        let (hg_set_with_conditions_idxs, hg_set_ids): (Vec<_>, Vec<_>) = self
            .hierarchy
            .topsort
            .iter()
            .cloned()
            .enumerate()
            .filter(|&(_i, id)| {
                // ignore system sets that have no conditions
                // ignore system type sets (already covered, they don't have conditions)
                id.is_set() && !self.system_set_conditions[id.index()].is_empty()
            })
            .unzip();

        let sys_count = self.systems.len();
        let set_with_conditions_count = hg_set_ids.len();
        let hg_node_count = self.hierarchy.graph.node_count();

        // get the number of dependencies and the immediate dependents of each system
        // (needed by multi_threaded executor to run systems in the correct order)
        let mut system_dependencies = Vec::with_capacity(sys_count);
        let mut system_dependents = Vec::with_capacity(sys_count);
        for &sys_id in &dg_system_ids {
            let num_dependencies = dependency_flattened_dag
                .graph
                .neighbors_directed(sys_id, Incoming)
                .count();

            let dependents = dependency_flattened_dag
                .graph
                .neighbors_directed(sys_id, Outgoing)
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
            for &(col, sys_id) in &hg_systems {
                let idx = dg_system_idx_map[&sys_id];
                let is_descendant = hier_results_reachable[index(row, col, hg_node_count)];
                bitset.set(idx, is_descendant);
            }
        }

        let mut sets_with_conditions_of_systems =
            vec![FixedBitSet::with_capacity(set_with_conditions_count); sys_count];
        for &(col, sys_id) in &hg_systems {
            let i = dg_system_idx_map[&sys_id];
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
        for ((id, system), conditions) in schedule
            .system_ids
            .drain(..)
            .zip(schedule.systems.drain(..))
            .zip(schedule.system_conditions.drain(..))
        {
            self.systems[id.index()].inner = Some(system);
            self.system_conditions[id.index()] = conditions;
        }

        for (id, conditions) in schedule
            .set_ids
            .drain(..)
            .zip(schedule.set_conditions.drain(..))
        {
            self.system_set_conditions[id.index()] = conditions;
        }

        *schedule = self.build_schedule(world, schedule_label, ignored_ambiguities)?;

        // move systems into new schedule
        for &id in &schedule.system_ids {
            let system = self.systems[id.index()].inner.take().unwrap();
            let conditions = core::mem::take(&mut self.system_conditions[id.index()]);
            schedule.systems.push(system);
            schedule.system_conditions.push(conditions);
        }

        for &id in &schedule.set_ids {
            let conditions = core::mem::take(&mut self.system_set_conditions[id.index()]);
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
        let name = match id {
            NodeId::System(_) => {
                let name = self.systems[id.index()].get().unwrap().name().to_string();
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
            NodeId::Set(_) => {
                let set = &self.system_sets[id.index()];
                if set.is_anonymous() {
                    self.anonymous_set_name(id)
                } else {
                    set.name()
                }
            }
        };
        if self.settings.use_shortnames {
            ShortName(&name).to_string()
        } else {
            name
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
                error!(
                    "Schedule {schedule_label:?} has redundant edges:\n {}",
                    message
                );
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
        set_system_bitsets: &HashMap<NodeId, FixedBitSet>,
    ) -> Result<(), ScheduleBuildError> {
        // check that there is no ordering between system sets that intersect
        for (a, b) in dep_results_connected {
            if !(a.is_set() && b.is_set()) {
                continue;
            }

            let a_systems = set_system_bitsets.get(a).unwrap();
            let b_systems = set_system_bitsets.get(b).unwrap();

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
        set_systems: &HashMap<NodeId, Vec<NodeId>>,
    ) -> Result<(), ScheduleBuildError> {
        for (&id, systems) in set_systems {
            let set = &self.system_sets[id.index()];
            if set.is_system_type() {
                let instances = systems.len();
                let ambiguous_with = self.ambiguous_with.edges(id);
                let before = self.dependency.graph.edges_directed(id, Incoming);
                let after = self.dependency.graph.edges_directed(id, Outgoing);
                let relations = before.count() + after.count() + ambiguous_with.count();
                if instances > 1 && relations > 0 {
                    return Err(ScheduleBuildError::SystemTypeSetAmbiguity(
                        self.get_node_name(&id),
                    ));
                }
            }
        }
        Ok(())
    }

    /// if [`ScheduleBuildSettings::ambiguity_detection`] is [`LogLevel::Ignore`], this check is skipped
    fn optionally_check_conflicts(
        &self,
        conflicts: &[(NodeId, NodeId, Vec<ComponentId>)],
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
                warn!("Schedule {schedule_label:?} has ambiguities.\n{}", message);
                Ok(())
            }
            LogLevel::Error => Err(ScheduleBuildError::Ambiguity(message)),
        }
    }

    fn get_conflicts_error_message(
        &self,
        ambiguities: &[(NodeId, NodeId, Vec<ComponentId>)],
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
        ambiguities: &'a [(NodeId, NodeId, Vec<ComponentId>)],
        components: &'a Components,
    ) -> impl Iterator<Item = (String, String, Vec<Cow<'a, str>>)> + 'a {
        ambiguities
            .iter()
            .map(move |(system_a, system_b, conflicts)| {
                let name_a = self.get_node_name(system_a);
                let name_b = self.get_node_name(system_b);

                debug_assert!(system_a.is_system(), "{name_a} is not a system.");
                debug_assert!(system_b.is_system(), "{name_b} is not a system.");

                let conflict_names: Vec<_> = conflicts
                    .iter()
                    .map(|id| components.get_name(*id).unwrap())
                    .collect();

                (name_a, name_b, conflict_names)
            })
    }

    fn traverse_sets_containing_node(&self, id: NodeId, f: &mut impl FnMut(NodeId) -> bool) {
        for (set_id, _) in self.hierarchy.graph.edges_directed(id, Incoming) {
            if f(set_id) {
                self.traverse_sets_containing_node(set_id, f);
            }
        }
    }

    fn names_of_sets_containing_node(&self, id: &NodeId) -> Vec<String> {
        let mut sets = <HashSet<_>>::default();
        self.traverse_sets_containing_node(*id, &mut |set_id| {
            !self.system_sets[set_id.index()].is_system_type() && sets.insert(set_id)
        });
        let mut sets: Vec<_> = sets
            .into_iter()
            .map(|set_id| self.get_node_name(&set_id))
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
}
