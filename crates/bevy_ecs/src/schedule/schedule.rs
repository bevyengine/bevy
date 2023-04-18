use std::{
    fmt::{Debug, Write},
    result::Result,
};

use bevy_utils::default;
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::{
    petgraph::{algo::TarjanScc, prelude::*},
    thiserror::Error,
    tracing::{error, warn},
    HashMap, HashSet,
};

use fixedbitset::FixedBitSet;

use crate::{
    self as bevy_ecs,
    component::{ComponentId, Components, Tick},
    schedule::*,
    system::{BoxedSystem, Resource, System},
    world::World,
};

/// Resource that stores [`Schedule`]s mapped to [`ScheduleLabel`]s.
#[derive(Default, Resource)]
pub struct Schedules {
    inner: HashMap<BoxedScheduleLabel, Schedule>,
}

impl Schedules {
    /// Constructs an empty `Schedules` with zero initial capacity.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Inserts a labeled schedule into the map.
    ///
    /// If the map already had an entry for `label`, `schedule` is inserted,
    /// and the old schedule is returned. Otherwise, `None` is returned.
    pub fn insert(&mut self, label: impl ScheduleLabel, schedule: Schedule) -> Option<Schedule> {
        let label = label.dyn_clone();
        self.inner.insert(label, schedule)
    }

    /// Removes the schedule corresponding to the `label` from the map, returning it if it existed.
    pub fn remove(&mut self, label: &dyn ScheduleLabel) -> Option<Schedule> {
        self.inner.remove(label)
    }

    /// Removes the (schedule, label) pair corresponding to the `label` from the map, returning it if it existed.
    pub fn remove_entry(
        &mut self,
        label: &dyn ScheduleLabel,
    ) -> Option<(Box<dyn ScheduleLabel>, Schedule)> {
        self.inner.remove_entry(label)
    }

    /// Does a schedule with the provided label already exist?
    pub fn contains(&self, label: &dyn ScheduleLabel) -> bool {
        self.inner.contains_key(label)
    }

    /// Returns a reference to the schedule associated with `label`, if it exists.
    pub fn get(&self, label: &dyn ScheduleLabel) -> Option<&Schedule> {
        self.inner.get(label)
    }

    /// Returns a mutable reference to the schedule associated with `label`, if it exists.
    pub fn get_mut(&mut self, label: &dyn ScheduleLabel) -> Option<&mut Schedule> {
        self.inner.get_mut(label)
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
        // label used when trace feature is enabled
        #[allow(unused_variables)]
        for (label, schedule) in self.inner.iter_mut() {
            #[cfg(feature = "trace")]
            let name = format!("{label:?}");
            #[cfg(feature = "trace")]
            let _one_span = info_span!("check schedule ticks", name = &name).entered();
            schedule.check_change_ticks(change_tick);
        }
    }
}

fn make_executor(kind: ExecutorKind) -> Box<dyn SystemExecutor> {
    match kind {
        ExecutorKind::Simple => Box::new(SimpleExecutor::new()),
        ExecutorKind::SingleThreaded => Box::new(SingleThreadedExecutor::new()),
        ExecutorKind::MultiThreaded => Box::new(MultiThreadedExecutor::new()),
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
    graph: ScheduleGraph,
    executable: SystemSchedule,
    executor: Box<dyn SystemExecutor>,
    executor_initialized: bool,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Constructs an empty `Schedule`.
    pub fn new() -> Self {
        Self {
            graph: ScheduleGraph::new(),
            executable: SystemSchedule::new(),
            executor: make_executor(ExecutorKind::default()),
            executor_initialized: false,
        }
    }

    /// Add a system to the schedule.
    #[deprecated(since = "0.11.0", note = "please use `add_systems` instead")]
    pub fn add_system<M>(&mut self, system: impl IntoSystemConfigs<M>) -> &mut Self {
        self.graph.add_systems_inner(system.into_configs(), false);
        self
    }

    /// Add a collection of systems to the schedule.
    pub fn add_systems<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.graph.add_systems_inner(systems.into_configs(), false);
        self
    }

    /// Configures a system set in this schedule, adding it if it does not exist.
    pub fn configure_set(&mut self, set: impl IntoSystemSetConfig) -> &mut Self {
        self.graph.configure_set(set);
        self
    }

    /// Configures a collection of system sets in this schedule, adding them if they does not exist.
    pub fn configure_sets(&mut self, sets: impl IntoSystemSetConfigs) -> &mut Self {
        self.graph.configure_sets(sets);
        self
    }

    /// Changes miscellaneous build settings.
    pub fn set_build_settings(&mut self, settings: ScheduleBuildSettings) -> &mut Self {
        self.graph.settings = settings;
        self
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

    /// Set whether the schedule applies buffers on final time or not. This is a catchall
    /// incase a system uses commands but was not explicitly ordered after a
    /// [`apply_system_buffers`](crate::prelude::apply_system_buffers). By default this
    /// setting is true, but may be disabled if needed.
    pub fn set_apply_final_buffers(&mut self, apply_final_buffers: bool) -> &mut Self {
        self.executor.set_apply_final_buffers(apply_final_buffers);
        self
    }

    /// Runs all systems in this schedule on the `world`, using its current execution strategy.
    pub fn run(&mut self, world: &mut World) {
        world.check_change_ticks();
        self.initialize(world).unwrap_or_else(|e| panic!("{e}"));
        self.executor.run(&mut self.executable, world);
    }

    /// Initializes any newly-added systems and conditions, rebuilds the executable schedule,
    /// and re-initializes the executor.
    ///
    /// Moves all systems and run conditions out of the [`ScheduleGraph`].
    pub fn initialize(&mut self, world: &mut World) -> Result<(), ScheduleBuildError> {
        if self.graph.changed {
            self.graph.initialize(world);
            self.graph
                .update_schedule(&mut self.executable, world.components())?;
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

    /// Iterates the change ticks of all systems in the schedule and clamps any older than
    /// [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for system in &mut self.executable.systems {
            system.check_change_tick(change_tick);
        }

        for conditions in &mut self.executable.system_conditions {
            for system in conditions.iter_mut() {
                system.check_change_tick(change_tick);
            }
        }

        for conditions in &mut self.executable.set_conditions {
            for system in conditions.iter_mut() {
                system.check_change_tick(change_tick);
            }
        }
    }

    /// Directly applies any accumulated system buffers (like [`Commands`](crate::prelude::Commands)) to the `world`.
    ///
    /// Like always, system buffers are applied in the "topological sort order" of the schedule graph.
    /// As a result, buffers from one system are only guaranteed to be applied before those of other systems
    /// if there is an explicit system ordering between the two systems.
    ///
    /// This is used in rendering to extract data from the main world, storing the data in system buffers,
    /// before applying their buffers in a different world.
    pub fn apply_system_buffers(&mut self, world: &mut World) {
        for system in &mut self.executable.systems {
            system.apply_buffers(world);
        }
    }
}

/// A directed acyclic graph structure.
#[derive(Default)]
pub struct Dag {
    /// A directed graph.
    graph: DiGraphMap<NodeId, ()>,
    /// A cached topological ordering of the graph.
    topsort: Vec<NodeId>,
}

impl Dag {
    fn new() -> Self {
        Self {
            graph: DiGraphMap::new(),
            topsort: Vec::new(),
        }
    }

    /// The directed graph of the stored systems, connected by their ordering dependencies.
    pub fn graph(&self) -> &DiGraphMap<NodeId, ()> {
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
    inner: BoxedSystemSet,
}

impl SystemSetNode {
    pub fn new(set: BoxedSystemSet) -> Self {
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

/// A [`BoxedSystem`] with metadata, stored in a [`ScheduleGraph`].
struct SystemNode {
    inner: Option<BoxedSystem>,
}

impl SystemNode {
    pub fn new(system: BoxedSystem) -> Self {
        Self {
            inner: Some(system),
        }
    }

    pub fn get(&self) -> Option<&BoxedSystem> {
        self.inner.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut BoxedSystem> {
        self.inner.as_mut()
    }
}

/// Metadata for a [`Schedule`].
#[derive(Default)]
pub struct ScheduleGraph {
    systems: Vec<SystemNode>,
    system_conditions: Vec<Option<Vec<BoxedCondition>>>,
    system_sets: Vec<SystemSetNode>,
    system_set_conditions: Vec<Option<Vec<BoxedCondition>>>,
    system_set_ids: HashMap<BoxedSystemSet, NodeId>,
    uninit: Vec<(NodeId, usize)>,
    hierarchy: Dag,
    dependency: Dag,
    dependency_flattened: Dag,
    ambiguous_with: UnGraphMap<NodeId, ()>,
    ambiguous_with_flattened: UnGraphMap<NodeId, ()>,
    ambiguous_with_all: HashSet<NodeId>,
    conflicting_systems: Vec<(NodeId, NodeId, Vec<ComponentId>)>,
    changed: bool,
    settings: ScheduleBuildSettings,
}

impl ScheduleGraph {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            system_conditions: Vec::new(),
            system_sets: Vec::new(),
            system_set_conditions: Vec::new(),
            system_set_ids: HashMap::new(),
            uninit: Vec::new(),
            hierarchy: Dag::new(),
            dependency: Dag::new(),
            dependency_flattened: Dag::new(),
            ambiguous_with: UnGraphMap::new(),
            ambiguous_with_flattened: UnGraphMap::new(),
            ambiguous_with_all: HashSet::new(),
            conflicting_systems: Vec::new(),
            changed: false,
            settings: default(),
        }
    }

    /// Returns the system at the given [`NodeId`], if it exists.
    pub fn get_system_at(&self, id: NodeId) -> Option<&dyn System<In = (), Out = ()>> {
        if !id.is_system() {
            return None;
        }
        self.systems
            .get(id.index())
            .and_then(|system| system.inner.as_deref())
    }

    /// Returns the system at the given [`NodeId`].
    ///
    /// Panics if it doesn't exist.
    #[track_caller]
    pub fn system_at(&self, id: NodeId) -> &dyn System<In = (), Out = ()> {
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

    /// Returns an iterator over all systems in this schedule.
    pub fn systems(
        &self,
    ) -> impl Iterator<Item = (NodeId, &dyn System<In = (), Out = ()>, &[BoxedCondition])> {
        self.systems
            .iter()
            .zip(self.system_conditions.iter())
            .enumerate()
            .filter_map(|(i, (system_node, condition))| {
                let system = system_node.inner.as_deref()?;
                let condition = condition.as_ref()?.as_slice();
                Some((NodeId::System(i), system, condition))
            })
    }

    /// Returns an iterator over all system sets in this schedule.
    pub fn system_sets(&self) -> impl Iterator<Item = (NodeId, &dyn SystemSet, &[BoxedCondition])> {
        self.system_set_ids.iter().map(|(_, node_id)| {
            let set_node = &self.system_sets[node_id.index()];
            let set = &*set_node.inner;
            let conditions = self.system_set_conditions[node_id.index()]
                .as_deref()
                .unwrap_or(&[]);
            (*node_id, set, conditions)
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

    /// Adds the systems to the graph. Returns a vector of all node ids contained the nested `SystemConfigs`
    /// if `ancestor_chained` is true. Also returns true if "densely chained", meaning that all nested items
    /// are linearly chained in the order they are defined
    fn add_systems_inner(
        &mut self,
        configs: SystemConfigs,
        ancestor_chained: bool,
    ) -> AddSystemsInnerResult {
        match configs {
            SystemConfigs::SystemConfig(config) => {
                let node_id = self.add_system_inner(config).unwrap();
                if ancestor_chained {
                    AddSystemsInnerResult {
                        densely_chained: true,
                        nodes: vec![node_id],
                    }
                } else {
                    AddSystemsInnerResult {
                        densely_chained: true,
                        nodes: Vec::new(),
                    }
                }
            }
            SystemConfigs::Configs {
                mut configs,
                collective_conditions,
                chained,
            } => {
                let more_than_one_entry = configs.len() > 1;
                if !collective_conditions.is_empty() {
                    if more_than_one_entry {
                        let set = AnonymousSet::new();
                        for config in &mut configs {
                            config.in_set_inner(set.dyn_clone());
                        }
                        let mut set_config = set.into_config();
                        set_config.conditions.extend(collective_conditions);
                        self.configure_set(set_config);
                    } else {
                        for condition in collective_conditions {
                            configs[0].run_if_inner(condition);
                        }
                    }
                }
                let mut config_iter = configs.into_iter();
                let mut nodes_in_scope = Vec::new();
                let mut densely_chained = true;
                if chained {
                    let Some(prev) = config_iter.next() else {
                        return AddSystemsInnerResult {
                            nodes: Vec::new(),
                            densely_chained: true
                        }
                    };
                    let mut previous_result = self.add_systems_inner(prev, true);
                    densely_chained = previous_result.densely_chained;
                    for current in config_iter {
                        let current_result = self.add_systems_inner(current, true);
                        densely_chained = densely_chained && current_result.densely_chained;
                        match (
                            previous_result.densely_chained,
                            current_result.densely_chained,
                        ) {
                            // Both groups are "densely" chained, so we can simplify the graph by only
                            // chaining the last in the previous list to the first in the current list
                            (true, true) => {
                                let last_in_prev = previous_result.nodes.last().unwrap();
                                let first_in_current = current_result.nodes.first().unwrap();
                                self.dependency.graph.add_edge(
                                    *last_in_prev,
                                    *first_in_current,
                                    (),
                                );
                            }
                            // The previous group is "densely" chained, so we can simplify the graph by only
                            // chaining the last item from the previous list to every item in the current list
                            (true, false) => {
                                let last_in_prev = previous_result.nodes.last().unwrap();
                                for current_node in &current_result.nodes {
                                    self.dependency.graph.add_edge(
                                        *last_in_prev,
                                        *current_node,
                                        (),
                                    );
                                }
                            }
                            // The current list is currently "densely" chained, so we can simplify the graph by
                            // only chaining every item in the previous list to the first item in the current list
                            (false, true) => {
                                let first_in_current = current_result.nodes.first().unwrap();
                                for previous_node in &previous_result.nodes {
                                    self.dependency.graph.add_edge(
                                        *previous_node,
                                        *first_in_current,
                                        (),
                                    );
                                }
                            }
                            // Neither of the lists are "densely" chained, so we must chain every item in the first
                            // list to every item in the second list
                            (false, false) => {
                                for previous_node in &previous_result.nodes {
                                    for current_node in &current_result.nodes {
                                        self.dependency.graph.add_edge(
                                            *previous_node,
                                            *current_node,
                                            (),
                                        );
                                    }
                                }
                            }
                        }

                        if ancestor_chained {
                            nodes_in_scope.append(&mut previous_result.nodes);
                        }

                        previous_result = current_result;
                    }

                    // ensure the last config's nodes are added
                    if ancestor_chained {
                        nodes_in_scope.append(&mut previous_result.nodes);
                    }
                } else {
                    for config in config_iter {
                        let result = self.add_systems_inner(config, ancestor_chained);
                        densely_chained = densely_chained && result.densely_chained;
                        if ancestor_chained {
                            nodes_in_scope.extend(result.nodes);
                        }
                    }

                    // an "unchained" SystemConfig is only densely chained if it has exactly one densely chained entry
                    if more_than_one_entry {
                        densely_chained = false;
                    }
                }

                AddSystemsInnerResult {
                    nodes: nodes_in_scope,
                    densely_chained,
                }
            }
        }
    }

    fn add_system_inner(&mut self, config: SystemConfig) -> Result<NodeId, ScheduleBuildError> {
        let id = NodeId::System(self.systems.len());

        // graph updates are immediate
        self.update_graphs(id, config.graph_info)?;

        // system init has to be deferred (need `&mut World`)
        self.uninit.push((id, 0));
        self.systems.push(SystemNode::new(config.system));
        self.system_conditions.push(Some(config.conditions));

        Ok(id)
    }

    fn configure_sets(&mut self, sets: impl IntoSystemSetConfigs) {
        let SystemSetConfigs { sets, chained } = sets.into_configs();
        let mut set_iter = sets.into_iter();
        if chained {
            let Some(prev) = set_iter.next() else { return };
            let mut prev_id = self.configure_set_inner(prev).unwrap();
            for next in set_iter {
                let next_id = self.configure_set_inner(next).unwrap();
                self.dependency.graph.add_edge(prev_id, next_id, ());
                prev_id = next_id;
            }
        } else {
            for set in set_iter {
                self.configure_set_inner(set).unwrap();
            }
        }
    }

    fn configure_set(&mut self, set: impl IntoSystemSetConfig) {
        self.configure_set_inner(set).unwrap();
    }

    fn configure_set_inner(
        &mut self,
        set: impl IntoSystemSetConfig,
    ) -> Result<NodeId, ScheduleBuildError> {
        let SystemSetConfig {
            set,
            graph_info,
            mut conditions,
        } = set.into_config();

        let id = match self.system_set_ids.get(&set) {
            Some(&id) => id,
            None => self.add_set(set.dyn_clone()),
        };

        // graph updates are immediate
        self.update_graphs(id, graph_info)?;

        // system init has to be deferred (need `&mut World`)
        let system_set_conditions =
            self.system_set_conditions[id.index()].get_or_insert_with(Vec::new);
        self.uninit.push((id, system_set_conditions.len()));
        system_set_conditions.append(&mut conditions);

        Ok(id)
    }

    fn add_set(&mut self, set: BoxedSystemSet) -> NodeId {
        let id = NodeId::Set(self.system_sets.len());
        self.system_sets.push(SystemSetNode::new(set.dyn_clone()));
        self.system_set_conditions.push(None);
        self.system_set_ids.insert(set, id);
        id
    }

    fn check_set(&mut self, id: &NodeId, set: &dyn SystemSet) -> Result<(), ScheduleBuildError> {
        match self.system_set_ids.get(set) {
            Some(set_id) => {
                if id == set_id {
                    return Err(ScheduleBuildError::HierarchyLoop(self.get_node_name(id)));
                }
            }
            None => {
                self.add_set(set.dyn_clone());
            }
        }

        Ok(())
    }

    fn check_sets(
        &mut self,
        id: &NodeId,
        graph_info: &GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        for set in &graph_info.sets {
            self.check_set(id, &**set)?;
        }

        if let Some(base_set) = &graph_info.base_set {
            self.check_set(id, &**base_set)?;
        }

        Ok(())
    }

    fn check_edges(
        &mut self,
        id: &NodeId,
        graph_info: &GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        for Dependency { kind: _, set } in &graph_info.dependencies {
            match self.system_set_ids.get(set) {
                Some(set_id) => {
                    if id == set_id {
                        return Err(ScheduleBuildError::DependencyLoop(self.get_node_name(id)));
                    }
                }
                None => {
                    self.add_set(set.dyn_clone());
                }
            }
        }

        if let Ambiguity::IgnoreWithSet(ambiguous_with) = &graph_info.ambiguous_with {
            for set in ambiguous_with {
                if !self.system_set_ids.contains_key(set) {
                    self.add_set(set.dyn_clone());
                }
            }
        }

        Ok(())
    }

    fn update_graphs(
        &mut self,
        id: NodeId,
        graph_info: GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        self.check_sets(&id, &graph_info)?;
        self.check_edges(&id, &graph_info)?;
        self.changed = true;

        let GraphInfo {
            sets,
            dependencies,
            ambiguous_with,
            ..
        } = graph_info;

        self.hierarchy.graph.add_node(id);
        self.dependency.graph.add_node(id);

        for set in sets.into_iter().map(|set| self.system_set_ids[&set]) {
            self.hierarchy.graph.add_edge(set, id, ());

            // ensure set also appears in dependency graph
            self.dependency.graph.add_node(set);
        }

        if !self.dependency.graph.contains_node(id) {
            self.dependency.graph.add_node(id);
        }

        for (kind, set) in dependencies
            .into_iter()
            .map(|Dependency { kind, set }| (kind, self.system_set_ids[&set]))
        {
            let (lhs, rhs) = match kind {
                DependencyKind::Before => (id, set),
                DependencyKind::After => (set, id),
            };
            self.dependency.graph.add_edge(lhs, rhs, ());

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
                    self.ambiguous_with.add_edge(id, set, ());
                }
            }
            Ambiguity::IgnoreAll => {
                self.ambiguous_with_all.insert(id);
            }
        }

        Ok(())
    }

    /// Initializes any newly-added systems and conditions by calling [`System::initialize`]
    pub fn initialize(&mut self, world: &mut World) {
        for (id, i) in self.uninit.drain(..) {
            match id {
                NodeId::System(index) => {
                    self.systems[index].get_mut().unwrap().initialize(world);
                    if let Some(v) = self.system_conditions[index].as_mut() {
                        for condition in v.iter_mut() {
                            condition.initialize(world);
                        }
                    }
                }
                NodeId::Set(index) => {
                    if let Some(v) = self.system_set_conditions[index].as_mut() {
                        for condition in v.iter_mut().skip(i) {
                            condition.initialize(world);
                        }
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
        components: &Components,
    ) -> Result<SystemSchedule, ScheduleBuildError> {
        // check hierarchy for cycles
        self.hierarchy.topsort = self
            .topsort_graph(&self.hierarchy.graph, ReportCycles::Hierarchy)
            .map_err(|_| ScheduleBuildError::HierarchyCycle)?;

        let hier_results = check_graph(&self.hierarchy.graph, &self.hierarchy.topsort);
        if self.settings.hierarchy_detection != LogLevel::Ignore
            && self.contains_hierarchy_conflicts(&hier_results.transitive_edges)
        {
            self.report_hierarchy_conflicts(&hier_results.transitive_edges);
            if matches!(self.settings.hierarchy_detection, LogLevel::Error) {
                return Err(ScheduleBuildError::HierarchyRedundancy);
            }
        }

        // remove redundant edges
        self.hierarchy.graph = hier_results.transitive_reduction;

        // check dependencies for cycles
        self.dependency.topsort = self
            .topsort_graph(&self.dependency.graph, ReportCycles::Dependency)
            .map_err(|_| ScheduleBuildError::DependencyCycle)?;

        // check for systems or system sets depending on sets they belong to
        let dep_results = check_graph(&self.dependency.graph, &self.dependency.topsort);
        for &(a, b) in dep_results.connected.iter() {
            if hier_results.connected.contains(&(a, b)) || hier_results.connected.contains(&(b, a))
            {
                let name_a = self.get_node_name(&a);
                let name_b = self.get_node_name(&b);
                return Err(ScheduleBuildError::CrossDependency(name_a, name_b));
            }
        }

        // map all system sets to their systems
        // go in reverse topological order (bottom-up) for efficiency
        let mut set_systems: HashMap<NodeId, Vec<NodeId>> =
            HashMap::with_capacity(self.system_sets.len());
        let mut set_system_bitsets = HashMap::with_capacity(self.system_sets.len());
        for &id in self.hierarchy.topsort.iter().rev() {
            if id.is_system() {
                continue;
            }

            let mut systems = Vec::new();
            let mut system_bitset = FixedBitSet::with_capacity(self.systems.len());

            for child in self
                .hierarchy
                .graph
                .neighbors_directed(id, Direction::Outgoing)
            {
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

        // check that there is no ordering between system sets that intersect
        for (a, b) in dep_results.connected.iter() {
            if !(a.is_set() && b.is_set()) {
                continue;
            }

            let a_systems = set_system_bitsets.get(a).unwrap();
            let b_systems = set_system_bitsets.get(b).unwrap();

            if !(a_systems.is_disjoint(b_systems)) {
                return Err(ScheduleBuildError::SetsHaveOrderButIntersect(
                    self.get_node_name(a),
                    self.get_node_name(b),
                ));
            }
        }

        // check that there are no edges to system-type sets that have multiple instances
        for (&id, systems) in set_systems.iter() {
            let set = &self.system_sets[id.index()];
            if set.is_system_type() {
                let instances = systems.len();
                let ambiguous_with = self.ambiguous_with.edges(id);
                let before = self
                    .dependency
                    .graph
                    .edges_directed(id, Direction::Incoming);
                let after = self
                    .dependency
                    .graph
                    .edges_directed(id, Direction::Outgoing);
                let relations = before.count() + after.count() + ambiguous_with.count();
                if instances > 1 && relations > 0 {
                    return Err(ScheduleBuildError::SystemTypeSetAmbiguity(
                        self.get_node_name(&id),
                    ));
                }
            }
        }

        // flatten: combine `in_set` with `before` and `after` information
        // have to do it like this to preserve transitivity
        let mut dependency_flattened = self.dependency.graph.clone();
        let mut temp = Vec::new();
        for (&set, systems) in set_systems.iter() {
            if systems.is_empty() {
                for a in dependency_flattened.neighbors_directed(set, Direction::Incoming) {
                    for b in dependency_flattened.neighbors_directed(set, Direction::Outgoing) {
                        temp.push((a, b));
                    }
                }
            } else {
                for a in dependency_flattened.neighbors_directed(set, Direction::Incoming) {
                    for &sys in systems {
                        temp.push((a, sys));
                    }
                }

                for b in dependency_flattened.neighbors_directed(set, Direction::Outgoing) {
                    for &sys in systems {
                        temp.push((sys, b));
                    }
                }
            }

            dependency_flattened.remove_node(set);
            for (a, b) in temp.drain(..) {
                dependency_flattened.add_edge(a, b, ());
            }
        }

        // topsort
        self.dependency_flattened.topsort = self
            .topsort_graph(&dependency_flattened, ReportCycles::Dependency)
            .map_err(|_| ScheduleBuildError::DependencyCycle)?;
        self.dependency_flattened.graph = dependency_flattened;

        let flat_results = check_graph(
            &self.dependency_flattened.graph,
            &self.dependency_flattened.topsort,
        );

        // remove redundant edges
        self.dependency_flattened.graph = flat_results.transitive_reduction;

        // flatten: combine `in_set` with `ambiguous_with` information
        let mut ambiguous_with_flattened = UnGraphMap::new();
        for (lhs, rhs, _) in self.ambiguous_with.all_edges() {
            match (lhs, rhs) {
                (NodeId::System(_), NodeId::System(_)) => {
                    ambiguous_with_flattened.add_edge(lhs, rhs, ());
                }
                (NodeId::Set(_), NodeId::System(_)) => {
                    for &lhs_ in set_systems.get(&lhs).unwrap_or(&Vec::new()) {
                        ambiguous_with_flattened.add_edge(lhs_, rhs, ());
                    }
                }
                (NodeId::System(_), NodeId::Set(_)) => {
                    for &rhs_ in set_systems.get(&rhs).unwrap_or(&Vec::new()) {
                        ambiguous_with_flattened.add_edge(lhs, rhs_, ());
                    }
                }
                (NodeId::Set(_), NodeId::Set(_)) => {
                    for &lhs_ in set_systems.get(&lhs).unwrap_or(&Vec::new()) {
                        for &rhs_ in set_systems.get(&rhs).unwrap_or(&vec![]) {
                            ambiguous_with_flattened.add_edge(lhs_, rhs_, ());
                        }
                    }
                }
            }
        }

        self.ambiguous_with_flattened = ambiguous_with_flattened;

        // check for conflicts
        let mut conflicting_systems = Vec::new();
        for &(a, b) in &flat_results.disconnected {
            if self.ambiguous_with_flattened.contains_edge(a, b)
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
                    let conflicts = access_a.get_conflicts(access_b);
                    conflicting_systems.push((a, b, conflicts));
                }
            }
        }

        if self.settings.ambiguity_detection != LogLevel::Ignore
            && self.contains_conflicts(&conflicting_systems)
        {
            self.report_conflicts(&conflicting_systems, components);
            if matches!(self.settings.ambiguity_detection, LogLevel::Error) {
                return Err(ScheduleBuildError::Ambiguity);
            }
        }
        self.conflicting_systems = conflicting_systems;

        // build the schedule
        let dg_system_ids = self.dependency_flattened.topsort.clone();
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
                id.is_set()
                    && self.system_set_conditions[id.index()]
                        .as_ref()
                        .filter(|v| !v.is_empty())
                        .is_some()
            })
            .unzip();

        let sys_count = self.systems.len();
        let set_with_conditions_count = hg_set_ids.len();
        let node_count = self.systems.len() + self.system_sets.len();

        // get the number of dependencies and the immediate dependents of each system
        // (needed by multi-threaded executor to run systems in the correct order)
        let mut system_dependencies = Vec::with_capacity(sys_count);
        let mut system_dependents = Vec::with_capacity(sys_count);
        for &sys_id in &dg_system_ids {
            let num_dependencies = self
                .dependency_flattened
                .graph
                .neighbors_directed(sys_id, Direction::Incoming)
                .count();

            let dependents = self
                .dependency_flattened
                .graph
                .neighbors_directed(sys_id, Direction::Outgoing)
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
                let is_descendant = hier_results.reachable[index(row, col, node_count)];
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
                let is_ancestor = hier_results.reachable[index(row, col, node_count)];
                bitset.set(idx, is_ancestor);
            }
        }

        Ok(SystemSchedule {
            systems: Vec::with_capacity(sys_count),
            system_conditions: Vec::with_capacity(sys_count),
            set_conditions: Vec::with_capacity(set_with_conditions_count),
            system_ids: dg_system_ids,
            set_ids: hg_set_ids,
            system_dependencies,
            system_dependents,
            sets_with_conditions_of_systems,
            systems_in_sets_with_conditions,
        })
    }

    fn update_schedule(
        &mut self,
        schedule: &mut SystemSchedule,
        components: &Components,
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
            self.system_conditions[id.index()] = Some(conditions);
        }

        for (id, conditions) in schedule
            .set_ids
            .drain(..)
            .zip(schedule.set_conditions.drain(..))
        {
            self.system_set_conditions[id.index()] = Some(conditions);
        }

        *schedule = self.build_schedule(components)?;

        // move systems into new schedule
        for &id in &schedule.system_ids {
            let system = self.systems[id.index()].inner.take().unwrap();
            let conditions = self.system_conditions[id.index()].take().unwrap();
            schedule.systems.push(system);
            schedule.system_conditions.push(conditions);
        }

        for &id in &schedule.set_ids {
            let conditions = self.system_set_conditions[id.index()].take().unwrap();
            schedule.set_conditions.push(conditions);
        }

        Ok(())
    }
}

/// Values returned by `ScheduleGraph::add_systems_inner`
struct AddSystemsInnerResult {
    /// All nodes contained inside this add_systems_inner call's SystemConfigs hierarchy
    nodes: Vec<NodeId>,
    /// True if and only if all nodes are "densely chained"
    densely_chained: bool,
}

/// Used to select the appropriate reporting function.
enum ReportCycles {
    Hierarchy,
    Dependency,
}

// methods for reporting errors
impl ScheduleGraph {
    fn get_node_name(&self, id: &NodeId) -> String {
        let mut name = match id {
            NodeId::System(_) => {
                let name = self.systems[id.index()].get().unwrap().name().to_string();
                if self.settings.report_sets {
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
            name = bevy_utils::get_short_name(&name);
        }
        name
    }

    fn anonymous_set_name(&self, id: &NodeId) -> String {
        format!(
            "({})",
            self.hierarchy
                .graph
                .edges_directed(*id, Direction::Outgoing)
                .map(|(_, member_id, _)| self.get_node_name(&member_id))
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

    fn contains_hierarchy_conflicts(&self, transitive_edges: &[(NodeId, NodeId)]) -> bool {
        if transitive_edges.is_empty() {
            return false;
        }

        true
    }

    fn report_hierarchy_conflicts(&self, transitive_edges: &[(NodeId, NodeId)]) {
        let mut message = String::from("hierarchy contains redundant edge(s)");
        for (parent, child) in transitive_edges {
            writeln!(
                message,
                " -- {:?} '{:?}' cannot be child of set '{:?}', longer path exists",
                self.get_node_kind(child),
                self.get_node_name(child),
                self.get_node_name(parent),
            )
            .unwrap();
        }

        error!("{}", message);
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
    fn topsort_graph(
        &self,
        graph: &DiGraphMap<NodeId, ()>,
        report: ReportCycles,
    ) -> Result<Vec<NodeId>, Vec<Vec<NodeId>>> {
        // Tarjan's SCC algorithm returns elements in *reverse* topological order.
        let mut tarjan_scc = TarjanScc::new();
        let mut top_sorted_nodes = Vec::with_capacity(graph.node_count());
        let mut sccs_with_cycles = Vec::new();

        tarjan_scc.run(graph, |scc| {
            // A strongly-connected component is a group of nodes who can all reach each other
            // through one or more paths. If an SCC contains more than one node, there must be
            // at least one cycle within them.
            if scc.len() > 1 {
                sccs_with_cycles.push(scc.to_vec());
            }
            top_sorted_nodes.extend_from_slice(scc);
        });

        if sccs_with_cycles.is_empty() {
            // reverse to get topological order
            top_sorted_nodes.reverse();
            Ok(top_sorted_nodes)
        } else {
            let mut cycles = Vec::new();
            for scc in &sccs_with_cycles {
                cycles.append(&mut simple_cycles_in_component(graph, scc));
            }

            match report {
                ReportCycles::Hierarchy => self.report_hierarchy_cycles(&cycles),
                ReportCycles::Dependency => self.report_dependency_cycles(&cycles),
            }

            Err(sccs_with_cycles)
        }
    }

    /// Logs details of cycles in the hierarchy graph.
    fn report_hierarchy_cycles(&self, cycles: &[Vec<NodeId>]) {
        let mut message = format!("schedule has {} in_set cycle(s):\n", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            let mut names = cycle.iter().map(|id| self.get_node_name(id));
            let first_name = names.next().unwrap();
            writeln!(
                message,
                "cycle {}: set '{first_name}' contains itself",
                i + 1,
            )
            .unwrap();
            writeln!(message, "set '{first_name}'").unwrap();
            for name in names.chain(std::iter::once(first_name)) {
                writeln!(message, " ... which contains set '{name}'").unwrap();
            }
            writeln!(message).unwrap();
        }

        error!("{}", message);
    }

    /// Logs details of cycles in the dependency graph.
    fn report_dependency_cycles(&self, cycles: &[Vec<NodeId>]) {
        let mut message = format!("schedule has {} before/after cycle(s):\n", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            let mut names = cycle
                .iter()
                .map(|id| (self.get_node_kind(id), self.get_node_name(id)));
            let (first_kind, first_name) = names.next().unwrap();
            writeln!(
                message,
                "cycle {}: {first_kind} '{first_name}' must run before itself",
                i + 1,
            )
            .unwrap();
            writeln!(message, "{first_kind} '{first_name}'").unwrap();
            for (kind, name) in names.chain(std::iter::once((first_kind, first_name))) {
                writeln!(message, " ... which must run before {kind} '{name}'").unwrap();
            }
            writeln!(message).unwrap();
        }

        error!("{}", message);
    }

    fn contains_conflicts(&self, conflicts: &[(NodeId, NodeId, Vec<ComponentId>)]) -> bool {
        if conflicts.is_empty() {
            return false;
        }

        true
    }

    fn report_conflicts(
        &self,
        ambiguities: &[(NodeId, NodeId, Vec<ComponentId>)],
        components: &Components,
    ) {
        let n_ambiguities = ambiguities.len();

        let mut string = format!(
            "{n_ambiguities} pairs of systems with conflicting data access have indeterminate execution order. \
            Consider adding `before`, `after`, or `ambiguous_with` relationships between these:\n",
        );

        for (system_a, system_b, conflicts) in ambiguities {
            let name_a = self.get_node_name(system_a);
            let name_b = self.get_node_name(system_b);

            debug_assert!(system_a.is_system(), "{name_a} is not a system.");
            debug_assert!(system_b.is_system(), "{name_b} is not a system.");

            writeln!(string, " -- {name_a} and {name_b}").unwrap();
            if !conflicts.is_empty() {
                let conflict_names: Vec<_> = conflicts
                    .iter()
                    .map(|id| components.get_name(*id).unwrap())
                    .collect();

                writeln!(string, "    conflict on: {conflict_names:?}").unwrap();
            } else {
                // one or both systems must be exclusive
                let world = std::any::type_name::<World>();
                writeln!(string, "    conflict on: {world}").unwrap();
            }
        }

        warn!("{}", string);
    }

    fn traverse_sets_containing_node(&self, id: NodeId, f: &mut impl FnMut(NodeId) -> bool) {
        for (set_id, _, _) in self.hierarchy.graph.edges_directed(id, Direction::Incoming) {
            if f(set_id) {
                self.traverse_sets_containing_node(set_id, f);
            }
        }
    }

    fn names_of_sets_containing_node(&self, id: &NodeId) -> Vec<String> {
        let mut sets = HashSet::new();
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
    #[error("`{0:?}` contains itself.")]
    HierarchyLoop(String),
    /// The hierarchy of system sets contains a cycle.
    #[error("System set hierarchy contains cycle(s).")]
    HierarchyCycle,
    /// The hierarchy of system sets contains redundant edges.
    ///
    /// This error is disabled by default, but can be opted-in using [`ScheduleBuildSettings`].
    #[error("System set hierarchy contains redundant edges.")]
    HierarchyRedundancy,
    /// A system (set) has been told to run before itself.
    #[error("`{0:?}` depends on itself.")]
    DependencyLoop(String),
    /// The dependency graph contains a cycle.
    #[error("System dependencies contain cycle(s).")]
    DependencyCycle,
    /// Tried to order a system (set) relative to a system set it belongs to.
    #[error("`{0:?}` and `{1:?}` have both `in_set` and `before`-`after` relationships (these might be transitive). This combination is unsolvable as a system cannot run before or after a set it belongs to.")]
    CrossDependency(String, String),
    /// Tried to order system sets that share systems.
    #[error("`{0:?}` and `{1:?}` have a `before`-`after` relationship (which may be transitive) but share systems.")]
    SetsHaveOrderButIntersect(String, String),
    /// Tried to order a system (set) relative to all instances of some system function.
    #[error("Tried to order against `fn {0:?}` in a schedule that has more than one `{0:?}` instance. `fn {0:?}` is a `SystemTypeSet` and cannot be used for ordering if ambiguous. Use a different set without this restriction.")]
    SystemTypeSetAmbiguity(String),
    /// Systems with conflicting access have indeterminate run order.
    ///
    /// This error is disabled by default, but can be opted-in using [`ScheduleBuildSettings`].
    #[error("Systems with conflicting access have indeterminate run order.")]
    Ambiguity,
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
    pub const fn new() -> Self {
        Self {
            ambiguity_detection: LogLevel::Ignore,
            hierarchy_detection: LogLevel::Warn,
            use_shortnames: true,
            report_sets: true,
        }
    }
}
