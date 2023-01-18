use std::{
    fmt::{Debug, Write},
    result::Result,
};

use bevy_utils::default;
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::{
    petgraph::{algo::tarjan_scc, prelude::*},
    thiserror::Error,
    tracing::{error, info, warn},
    HashMap, HashSet,
};

use fixedbitset::FixedBitSet;

use crate::{
    self as bevy_ecs,
    component::ComponentId,
    schedule_v3::*,
    system::{BoxedSystem, Resource},
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
        let label: Box<dyn ScheduleLabel> = Box::new(label);
        if self.inner.contains_key(&label) {
            warn!("schedule with label {:?} already exists", label);
        }
        self.inner.insert(label, schedule)
    }

    /// Removes the schedule corresponding to the `label` from the map, returning it if it existed.
    pub fn remove(&mut self, label: &dyn ScheduleLabel) -> Option<Schedule> {
        if !self.inner.contains_key(label) {
            warn!("schedule with label {:?} not found", label);
        }
        self.inner.remove(label)
    }

    /// Returns a reference to the schedule associated with `label`, if it exists.
    pub fn get(&self, label: &dyn ScheduleLabel) -> Option<&Schedule> {
        self.inner.get(label)
    }

    /// Returns a mutable reference to the schedule associated with `label`, if it exists.
    pub fn get_mut(&mut self, label: &dyn ScheduleLabel) -> Option<&mut Schedule> {
        self.inner.get_mut(label)
    }

    /// Iterates the change ticks of all systems in all stored schedules and clamps any older than
    /// [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        #[cfg(feature = "trace")]
        let _all_span = info_span!("check stored schedule ticks").entered();
        // label used when trace feature is enabled
        #[allow(unused_variables)]
        for (label, schedule) in self.inner.iter_mut() {
            #[cfg(feature = "trace")]
            let name = format!("{:?}", label);
            #[cfg(feature = "trace")]
            let _one_span = info_span!("check schedule ticks", name = &name).entered();
            schedule.check_change_ticks(change_tick);
        }
    }
}

/// A collection of systems, and the metadata and executor needed to run them
/// in a certain order under certain conditions.
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
            executor: Box::new(MultiThreadedExecutor::new()),
            executor_initialized: false,
        }
    }

    /// Add a system to the schedule.
    pub fn add_system<P>(&mut self, system: impl IntoSystemConfig<P>) -> &mut Self {
        self.graph.add_system(system);
        self
    }

    /// Add a collection of systems to the schedule.
    pub fn add_systems<P>(&mut self, systems: impl IntoSystemConfigs<P>) -> &mut Self {
        self.graph.add_systems(systems);
        self
    }

    /// Configure a system set in this schedule.
    pub fn configure_set(&mut self, set: impl IntoSystemSetConfig) -> &mut Self {
        self.graph.configure_set(set);
        self
    }

    /// Configure a collection of system sets in this schedule.
    pub fn configure_sets(&mut self, sets: impl IntoSystemSetConfigs) -> &mut Self {
        self.graph.configure_sets(sets);
        self
    }

    /// Changes the system set that new systems and system sets will join by default
    /// if they aren't already part of one.
    pub fn set_default_set(&mut self, set: impl SystemSet) -> &mut Self {
        self.graph.set_default_set(set);
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
            self.executor = match executor {
                ExecutorKind::Simple => Box::new(SimpleExecutor::new()),
                ExecutorKind::SingleThreaded => Box::new(SingleThreadedExecutor::new()),
                ExecutorKind::MultiThreaded => Box::new(MultiThreadedExecutor::new()),
            };
            self.executor_initialized = false;
        }
        self
    }

    /// Runs all systems in this schedule on the `world`, using its current execution strategy.
    pub fn run(&mut self, world: &mut World) {
        world.check_change_ticks();
        self.initialize(world).unwrap();
        // TODO: label
        #[cfg(feature = "trace")]
        let _span = info_span!("schedule").entered();
        self.executor.run(&mut self.executable, world);
    }

    /// Initializes any newly-added systems and conditions, rebuilds the executable schedule,
    /// and re-initializes the executor.
    pub fn initialize(&mut self, world: &mut World) -> Result<(), ScheduleBuildError> {
        if self.graph.changed {
            self.graph.initialize(world);
            self.graph.update_schedule(&mut self.executable)?;
            self.graph.changed = false;
            self.executor_initialized = false;
        }

        if !self.executor_initialized {
            self.executor.init(&self.executable);
            self.executor_initialized = true;
        }

        Ok(())
    }

    /// Iterates the change ticks of all systems in the schedule and clamps any older than
    /// [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
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
}

/// A directed acylic graph structure.
#[derive(Default)]
struct Dag {
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
}

/// A [`SystemSet`] with metadata, stored in a [`ScheduleGraph`].
struct SystemSetNode {
    inner: BoxedSystemSet,
    /// `true` if this system set was modified with `configure_set`
    configured: bool,
}

impl SystemSetNode {
    pub fn new(set: BoxedSystemSet) -> Self {
        Self {
            inner: set,
            configured: false,
        }
    }

    pub fn name(&self) -> String {
        format!("{:?}", &self.inner)
    }

    pub fn is_system_type(&self) -> bool {
        self.inner.is_system_type()
    }
}

/// Metadata for a [`Schedule`].
#[derive(Default)]
struct ScheduleGraph {
    systems: Vec<Option<BoxedSystem>>,
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
    default_set: Option<BoxedSystemSet>,
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
            default_set: None,
            changed: false,
            settings: default(),
        }
    }

    fn set_default_set(&mut self, set: impl SystemSet) {
        assert!(
            !set.is_system_type(),
            "adding arbitrary systems to a system type set is not allowed"
        );
        self.default_set = Some(Box::new(set));
    }

    fn add_systems<P>(&mut self, systems: impl IntoSystemConfigs<P>) {
        let SystemConfigs { systems, chained } = systems.into_configs();
        let mut system_iter = systems.into_iter();
        if chained {
            let Some(prev) = system_iter.next() else { return };
            let mut prev_id = self.add_system_inner(prev).unwrap();
            for next in system_iter {
                let next_id = self.add_system_inner(next).unwrap();
                self.dependency.graph.add_edge(prev_id, next_id, ());
                prev_id = next_id;
            }
        } else {
            for system in system_iter {
                self.add_system_inner(system).unwrap();
            }
        }
    }

    fn add_system<P>(&mut self, system: impl IntoSystemConfig<P>) {
        self.add_system_inner(system).unwrap();
    }

    fn add_system_inner<P>(
        &mut self,
        system: impl IntoSystemConfig<P>,
    ) -> Result<NodeId, ScheduleBuildError> {
        let SystemConfig {
            system,
            mut graph_info,
            conditions,
        } = system.into_config();

        let id = NodeId::System(self.systems.len());

        if graph_info.sets.is_empty() {
            if let Some(default) = self.default_set.as_ref() {
                graph_info.sets.push(default.dyn_clone());
            }
        }

        // graph updates are immediate
        self.update_graphs(id, graph_info)?;

        // system init has to be deferred (need `&mut World`)
        self.uninit.push((id, 0));
        self.systems.push(Some(system));
        self.system_conditions.push(Some(conditions));

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
            mut graph_info,
            mut conditions,
        } = set.into_config();

        let id = match self.system_set_ids.get(&set) {
            Some(&id) => id,
            None => self.add_set(set.dyn_clone()),
        };

        let meta = &mut self.system_sets[id.index()];
        let already_configured = std::mem::replace(&mut meta.configured, true);

        // a system set can be configured multiple times, so this "default check"
        // should only happen the first time `configure_set` is called on it
        if !already_configured && graph_info.sets.is_empty() {
            if let Some(default) = self.default_set.as_ref() {
                info!("adding system set `{:?}` to default: `{:?}`", set, default);
                graph_info.sets.push(default.dyn_clone());
            }
        }

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

    fn check_sets(
        &mut self,
        id: &NodeId,
        graph_info: &GraphInfo,
    ) -> Result<(), ScheduleBuildError> {
        for set in &graph_info.sets {
            match self.system_set_ids.get(set) {
                Some(set_id) => {
                    if id == set_id {
                        return Err(ScheduleBuildError::HierarchyLoop(set.dyn_clone()));
                    }
                }
                None => {
                    self.add_set(set.dyn_clone());
                }
            }
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
                        return Err(ScheduleBuildError::DependencyLoop(set.dyn_clone()));
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
        } = graph_info;

        if !self.hierarchy.graph.contains_node(id) {
            self.hierarchy.graph.add_node(id);
        }

        for set in sets.into_iter().map(|set| self.system_set_ids[&set]) {
            self.hierarchy.graph.add_edge(set, id, ());
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
        }

        match ambiguous_with {
            Ambiguity::Check => (),
            Ambiguity::IgnoreWithSet(ambigous_with) => {
                for set in ambigous_with
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

    fn initialize(&mut self, world: &mut World) {
        for (id, i) in self.uninit.drain(..) {
            match id {
                NodeId::System(index) => {
                    self.systems[index].as_mut().unwrap().initialize(world);
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

    fn build_schedule(&mut self) -> Result<SystemSchedule, ScheduleBuildError> {
        // check hierarchy for cycles
        let hier_scc = tarjan_scc(&self.hierarchy.graph);
        if self.contains_cycles(&hier_scc) {
            self.report_cycles(&hier_scc);
            return Err(ScheduleBuildError::HierarchyCycle);
        }

        self.hierarchy.topsort = hier_scc.into_iter().flatten().rev().collect::<Vec<_>>();

        let hier_results = check_graph(&self.hierarchy.graph, &self.hierarchy.topsort);
        if self.contains_hierarchy_conflicts(&hier_results.transitive_edges) {
            self.report_hierarchy_conflicts(&hier_results.transitive_edges);
            if matches!(self.settings.hierarchy_detection, LogLevel::Error) {
                return Err(ScheduleBuildError::HierarchyRedundancy);
            }
        }

        // remove redundant edges
        self.hierarchy.graph = hier_results.transitive_reduction;

        // check dependencies for cycles
        let dep_scc = tarjan_scc(&self.dependency.graph);
        if self.contains_cycles(&dep_scc) {
            self.report_cycles(&dep_scc);
            return Err(ScheduleBuildError::DependencyCycle);
        }

        self.dependency.topsort = dep_scc.into_iter().flatten().rev().collect::<Vec<_>>();

        // nodes can have dependent XOR hierarchical relationship
        let dep_results = check_graph(&self.dependency.graph, &self.dependency.topsort);
        for &(a, b) in dep_results.connected.iter() {
            if hier_results.connected.contains(&(a, b)) || hier_results.connected.contains(&(b, a))
            {
                let name_a = self.get_node_name(&a);
                let name_b = self.get_node_name(&b);
                return Err(ScheduleBuildError::CrossDependency(name_a, name_b));
            }
        }

        // map system sets to all their member systems
        let mut systems_in_sets = HashMap::with_capacity(self.system_sets.len());
        // iterate in reverse topological order (bottom-up)
        for &id in self.hierarchy.topsort.iter().rev() {
            if id.is_system() {
                continue;
            }

            let set = id;
            systems_in_sets.insert(set, Vec::new());

            for child in self
                .hierarchy
                .graph
                .neighbors_directed(set, Direction::Outgoing)
            {
                match child {
                    NodeId::System(_) => {
                        systems_in_sets.get_mut(&set).unwrap().push(child);
                    }
                    NodeId::Set(_) => {
                        let [sys, child_sys] =
                            systems_in_sets.get_many_mut([&set, &child]).unwrap();
                        sys.extend_from_slice(child_sys);
                    }
                }
            }
        }

        // can't depend on or be ambiguous with system type sets that have many instances
        for (&set, systems) in systems_in_sets.iter() {
            let node = &self.system_sets[set.index()];
            if node.is_system_type() {
                let ambiguities = self.ambiguous_with.edges(set).count();
                let mut dependencies = 0;
                dependencies += self
                    .dependency
                    .graph
                    .edges_directed(set, Direction::Incoming)
                    .count();
                dependencies += self
                    .dependency
                    .graph
                    .edges_directed(set, Direction::Outgoing)
                    .count();
                if systems.len() > 1 && (ambiguities > 0 || dependencies > 0) {
                    return Err(ScheduleBuildError::SystemTypeSetAmbiguity(
                        node.inner.dyn_clone(),
                    ));
                }
            }
        }

        // flatten dependency graph
        let mut dependency_flattened = DiGraphMap::new();
        for id in self.dependency.graph.nodes() {
            if id.is_system() {
                dependency_flattened.add_node(id);
            }
        }

        for (lhs, rhs, _) in self.dependency.graph.all_edges() {
            match (lhs, rhs) {
                (NodeId::System(_), NodeId::System(_)) => {
                    dependency_flattened.add_edge(lhs, rhs, ());
                }
                (NodeId::Set(_), NodeId::System(_)) => {
                    for &lhs_ in &systems_in_sets[&lhs] {
                        dependency_flattened.add_edge(lhs_, rhs, ());
                    }
                }
                (NodeId::System(_), NodeId::Set(_)) => {
                    for &rhs_ in &systems_in_sets[&rhs] {
                        dependency_flattened.add_edge(lhs, rhs_, ());
                    }
                }
                (NodeId::Set(_), NodeId::Set(_)) => {
                    for &lhs_ in &systems_in_sets[&lhs] {
                        for &rhs_ in &systems_in_sets[&rhs] {
                            dependency_flattened.add_edge(lhs_, rhs_, ());
                        }
                    }
                }
            }
        }

        // check flattened dependencies for cycles
        let flat_scc = tarjan_scc(&dependency_flattened);
        if self.contains_cycles(&flat_scc) {
            self.report_cycles(&flat_scc);
            return Err(ScheduleBuildError::DependencyCycle);
        }

        self.dependency_flattened.graph = dependency_flattened;
        self.dependency_flattened.topsort =
            flat_scc.into_iter().flatten().rev().collect::<Vec<_>>();

        let flat_results = check_graph(
            &self.dependency_flattened.graph,
            &self.dependency_flattened.topsort,
        );

        // remove redundant edges
        self.dependency_flattened.graph = flat_results.transitive_reduction;

        // flatten allowed ambiguities
        let mut ambiguous_with_flattened = UnGraphMap::new();
        for (lhs, rhs, _) in self.ambiguous_with.all_edges() {
            match (lhs, rhs) {
                (NodeId::System(_), NodeId::System(_)) => {
                    ambiguous_with_flattened.add_edge(lhs, rhs, ());
                }
                (NodeId::Set(_), NodeId::System(_)) => {
                    for &lhs_ in &systems_in_sets[&lhs] {
                        ambiguous_with_flattened.add_edge(lhs_, rhs, ());
                    }
                }
                (NodeId::System(_), NodeId::Set(_)) => {
                    for &rhs_ in &systems_in_sets[&rhs] {
                        ambiguous_with_flattened.add_edge(lhs, rhs_, ());
                    }
                }
                (NodeId::Set(_), NodeId::Set(_)) => {
                    for &lhs_ in &systems_in_sets[&lhs] {
                        for &rhs_ in &systems_in_sets[&rhs] {
                            ambiguous_with_flattened.add_edge(lhs_, rhs_, ());
                        }
                    }
                }
            }
        }

        self.ambiguous_with_flattened = ambiguous_with_flattened;

        // check for conflicts
        let mut conflicting_systems = Vec::new();
        for &(a, b) in flat_results.disconnected.iter() {
            if self.ambiguous_with_flattened.contains_edge(a, b)
                || self.ambiguous_with_all.contains(&a)
                || self.ambiguous_with_all.contains(&b)
            {
                continue;
            }

            let system_a = self.systems[a.index()].as_ref().unwrap();
            let system_b = self.systems[b.index()].as_ref().unwrap();
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

        if self.contains_conflicts(&conflicting_systems) {
            self.report_conflicts(&conflicting_systems);
            if matches!(self.settings.ambiguity_detection, LogLevel::Error) {
                return Err(ScheduleBuildError::Ambiguity);
            }
        }

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

        let (hg_set_idxs, hg_set_ids): (Vec<_>, Vec<_>) = self
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
        let set_count = hg_set_ids.len();
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
        let mut systems_in_sets = vec![FixedBitSet::with_capacity(sys_count); set_count];
        for (i, &row) in hg_set_idxs.iter().enumerate() {
            let bitset = &mut systems_in_sets[i];
            for &(col, sys_id) in &hg_systems {
                let idx = dg_system_idx_map[&sys_id];
                let is_descendant = hier_results.reachable[index(row, col, node_count)];
                bitset.set(idx, is_descendant);
            }
        }

        let mut sets_of_systems = vec![FixedBitSet::with_capacity(set_count); sys_count];
        for &(col, sys_id) in &hg_systems {
            let i = dg_system_idx_map[&sys_id];
            let bitset = &mut sets_of_systems[i];
            for (idx, &row) in hg_set_idxs
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
            set_conditions: Vec::with_capacity(set_count),
            system_ids: dg_system_ids,
            set_ids: hg_set_ids,
            system_dependencies,
            system_dependents,
            sets_of_systems,
            systems_in_sets,
        })
    }

    fn update_schedule(&mut self, schedule: &mut SystemSchedule) -> Result<(), ScheduleBuildError> {
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
            self.systems[id.index()] = Some(system);
            self.system_conditions[id.index()] = Some(conditions);
        }

        for (id, conditions) in schedule
            .set_ids
            .drain(..)
            .zip(schedule.set_conditions.drain(..))
        {
            self.system_set_conditions[id.index()] = Some(conditions);
        }

        *schedule = self.build_schedule()?;

        // move systems into new schedule
        for &id in &schedule.system_ids {
            let system = self.systems[id.index()].take().unwrap();
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

// methods for reporting errors
impl ScheduleGraph {
    fn get_node_name(&self, id: &NodeId) -> String {
        match id {
            NodeId::System(_) => self.systems[id.index()]
                .as_ref()
                .unwrap()
                .name()
                .to_string(),
            NodeId::Set(_) => self.system_sets[id.index()].name(),
        }
    }

    fn get_node_kind(id: &NodeId) -> &'static str {
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
                Self::get_node_kind(child),
                self.get_node_name(child),
                self.get_node_name(parent),
            )
            .unwrap();
        }

        error!("{}", message);
    }

    fn contains_cycles(&self, strongly_connected_components: &[Vec<NodeId>]) -> bool {
        if strongly_connected_components
            .iter()
            .all(|scc| scc.len() == 1)
        {
            return false;
        }

        true
    }

    fn report_cycles(&self, strongly_connected_components: &[Vec<NodeId>]) {
        let components_with_cycles = strongly_connected_components
            .iter()
            .filter(|scc| scc.len() > 1)
            .cloned()
            .collect::<Vec<_>>();

        let mut message = format!(
            "schedule contains at least {} cycle(s)",
            components_with_cycles.len()
        );

        writeln!(message, " -- cycle(s) found within:").unwrap();
        for (i, scc) in components_with_cycles.into_iter().enumerate() {
            let names = scc
                .iter()
                .map(|id| self.get_node_name(id))
                .collect::<Vec<_>>();
            writeln!(message, " ---- {i}: {names:?}").unwrap();
        }

        error!("{}", message);
    }

    fn contains_conflicts(&self, conflicts: &[(NodeId, NodeId, Vec<ComponentId>)]) -> bool {
        if conflicts.is_empty() {
            return false;
        }

        true
    }

    fn report_conflicts(&self, ambiguities: &[(NodeId, NodeId, Vec<ComponentId>)]) {
        let mut string = String::from(
            "Some systems with conflicting access have indeterminate execution order. \
            Consider adding `before`, `after`, or `ambiguous_with` relationships between these:\n",
        );

        for (system_a, system_b, conflicts) in ambiguities {
            debug_assert!(system_a.is_system());
            debug_assert!(system_b.is_system());
            let name_a = self.get_node_name(system_a);
            let name_b = self.get_node_name(system_b);

            writeln!(string, " -- {name_a} and {name_b}").unwrap();
            if !conflicts.is_empty() {
                writeln!(string, "    conflict on: {conflicts:?}").unwrap();
            } else {
                // one or both systems must be exclusive
                let world = std::any::type_name::<World>();
                writeln!(string, "    conflict on: {world}").unwrap();
            }
        }

        warn!("{}", string);
    }
}

/// Category of errors encountered during schedule construction.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ScheduleBuildError {
    /// A system set contains itself.
    #[error("`{0:?}` contains itself.")]
    HierarchyLoop(BoxedSystemSet),
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
    DependencyLoop(BoxedSystemSet),
    /// The dependency graph contains a cycle.
    #[error("System dependencies contain cycle(s).")]
    DependencyCycle,
    /// Tried to order a system (set) relative to a system set it belongs to.
    #[error("`{0:?}` and `{1:?}` have both `in_set` and `before`-`after` relationships (these might be transitive). This combination is unsolvable as a system cannot run before or after a set it belongs to.")]
    CrossDependency(String, String),
    /// Tried to order a system (set) relative to all instances of some system function.
    #[error("Tried to order against `fn {0:?}` in a schedule that has more than one `{0:?}` instance. `fn {0:?}` is a `SystemTypeSet` and cannot be used for ordering if ambiguous. Use a different set without this restriction.")]
    SystemTypeSetAmbiguity(BoxedSystemSet),
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
pub enum LogLevel {
    /// Occurrences are logged only.
    Warn,
    /// Occurrences are logged and result in errors.
    Error,
}

/// Specifies miscellaneous settings for schedule construction.
pub struct ScheduleBuildSettings {
    ambiguity_detection: LogLevel,
    hierarchy_detection: LogLevel,
}

impl Default for ScheduleBuildSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleBuildSettings {
    pub const fn new() -> Self {
        Self {
            ambiguity_detection: LogLevel::Warn,
            hierarchy_detection: LogLevel::Warn,
        }
    }

    /// Determines whether the presence of ambiguities (systems with conflicting access but indeterminate order)
    /// is only logged or also results in an [`Ambiguity`](ScheduleBuildError::Ambiguity) error.
    pub fn with_ambiguity_detection(mut self, level: LogLevel) -> Self {
        self.ambiguity_detection = level;
        self
    }

    /// Determines whether the presence of redundant edges in the hierarchy of system sets is only
    /// logged or also results in a [`HierarchyRedundancy`](ScheduleBuildError::HierarchyRedundancy)
    /// error.
    pub fn with_hierarchy_detection(mut self, level: LogLevel) -> Self {
        self.hierarchy_detection = level;
        self
    }
}
