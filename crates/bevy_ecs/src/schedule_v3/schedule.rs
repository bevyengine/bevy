use std::{
    borrow::Cow,
    fmt::{Debug, Write},
    result::Result,
};

use bevy_utils::{
    petgraph::{algo::tarjan_scc, prelude::*},
    thiserror::Error,
    tracing::{error, warn},
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

/// Resource for storing [`Schedule`]s.
#[derive(Default, Resource)]
pub struct Schedules {
    inner: HashMap<BoxedScheduleLabel, Option<Schedule>>,
}

impl Schedules {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Insert a new labeled schedule into the map.
    ///
    /// # Errors
    ///
    /// TODO
    pub fn insert(
        &mut self,
        label: impl ScheduleLabel,
        schedule: Schedule,
    ) -> Result<(), InsertionError> {
        let label = label.dyn_clone();
        if self.inner.contains_key(&label) {
            return Err(InsertionError::AlreadyExists(label));
        }
        self.inner.insert(label, Some(schedule));
        Ok(())
    }

    /// Returns the schedule corresponding to the label.
    ///
    /// # Errors
    ///
    /// TODO
    pub(crate) fn check_out(
        &mut self,
        label: &dyn ScheduleLabel,
    ) -> Result<Schedule, ExtractionError> {
        let label = label.dyn_clone();
        match self.inner.get_mut(&label) {
            Some(container) => match container.take() {
                Some(schedule) => Ok(schedule),
                None => Err(ExtractionError::AlreadyExtracted(label)),
            },
            None => Err(ExtractionError::Unknown(label)),
        }
    }

    /// Re-inserts the schedule corresponding to the label into the map.
    ///
    /// The schedule must have been previously extracted using [`check_out`](#method.check_out).
    ///
    /// # Errors
    ///
    /// TODO
    pub(crate) fn check_in(
        &mut self,
        label: &dyn ScheduleLabel,
        schedule: Schedule,
    ) -> Result<(), ExtractionError> {
        let label = label.dyn_clone();
        match self.inner.get_mut(&label) {
            Some(container) => match container.take() {
                Some(_) => Err(ExtractionError::NotExtracted(label)),
                None => {
                    *container = Some(schedule);
                    Ok(())
                }
            },
            None => Err(ExtractionError::Unknown(label)),
        }
    }

    /// Iterates all system change ticks and clamps any older than [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    ///
    /// **Note:** Does nothing if the [`World`] counter has not been incremented at least [`CHECK_TICK_THRESHOLD`](crate::change_detection::CHECK_TICK_THRESHOLD)
    /// times since the previous pass.
    pub(crate) fn check_change_ticks(&mut self, change_tick: u32, last_check_tick: u32) {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("check ticks").entered();
        for schedule in self.inner.values_mut().flatten() {
            schedule.check_change_ticks(change_tick, last_check_tick);
        }
    }
}

/// TBD
pub struct Schedule {
    graph: ScheduleMeta,
    executable: SystemSchedule,
    executor: Box<dyn SystemExecutor>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            graph: ScheduleMeta::new(),
            executable: SystemSchedule::new(),
            executor: Box::new(MultiThreadedExecutor::new()),
        }
    }

    pub fn add_system<P>(&mut self, system: impl IntoSystemConfig<P>) {
        self.graph.add_system(system);
    }

    pub fn add_systems<P>(&mut self, systems: impl IntoSystemConfigs<P>) {
        self.graph.add_systems(systems);
    }

    pub fn configure_set(&mut self, set: impl IntoSystemSetConfig) {
        self.graph.configure_set(set);
    }

    pub fn configure_sets(&mut self, sets: impl IntoSystemSetConfigs) {
        self.graph.configure_sets(sets);
    }

    pub fn set_default_set(&mut self, set: impl SystemSet) {
        self.graph.set_default_set(set);
    }

    pub fn get_executor_kind(&self) -> ExecutorKind {
        self.executor.kind()
    }

    pub fn set_executor_kind(&mut self, executor: ExecutorKind) {
        self.executor = match executor {
            ExecutorKind::Simple => Box::new(SimpleExecutor::new()),
            ExecutorKind::SingleThreaded => Box::new(SingleThreadedExecutor::new()),
            ExecutorKind::MultiThreaded => Box::new(MultiThreadedExecutor::new()),
        };
    }

    pub fn run(&mut self, world: &mut World) {
        self.initialize(world).unwrap();
        self.executor.run(&mut self.executable, world);
    }

    pub(crate) fn initialize(&mut self, world: &mut World) -> Result<(), BuildError> {
        if self.graph.changed {
            self.graph.initialize(world);
            self.graph.update_schedule(&mut self.executable)?;
            self.executor.init(&self.executable);
            self.graph.changed = false;
        }

        Ok(())
    }

    /// Iterates all component change ticks and clamps any older than [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    ///
    /// **Note:** Does nothing if the [`World`] counter has not been incremented at least [`CHECK_TICK_THRESHOLD`](crate::change_detection::CHECK_TICK_THRESHOLD)
    /// times since the previous pass.
    pub(crate) fn check_change_ticks(&mut self, _change_tick: u32, _last_change_tick: u32) {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("check schedule ticks").entered();
        todo!();
    }
}

#[derive(Default)]
struct Dag {
    /// A directed graph.
    graph: DiGraphMap<NodeId, ()>,
    /// A cached topological ordering of the graph.
    topsort: Vec<NodeId>,
}

impl Dag {
    pub fn new() -> Self {
        Self {
            graph: DiGraphMap::new(),
            topsort: Vec::new(),
        }
    }
}

struct SystemSetMeta(BoxedSystemSet);

impl SystemSetMeta {
    pub fn new(set: BoxedSystemSet) -> Self {
        Self(set)
    }

    pub fn name(&self) -> Cow<'static, str> {
        format!("{:?}", &self.0).into()
    }

    pub fn is_system_type(&self) -> bool {
        self.0.is_system_type()
    }
}

enum UninitNode {
    System(BoxedSystem, Vec<BoxedCondition>),
    SystemSet(Vec<BoxedCondition>),
}

#[derive(Default)]
struct ScheduleMeta {
    system_set_ids: HashMap<BoxedSystemSet, NodeId>,
    system_sets: HashMap<NodeId, SystemSetMeta>,
    systems: HashMap<NodeId, BoxedSystem>,
    conditions: HashMap<NodeId, Vec<BoxedCondition>>,

    hierarchy: Dag,
    dependency: Dag,
    dependency_flattened: Dag,

    ambiguous_with: UnGraphMap<NodeId, ()>,
    ambiguous_with_flattened: UnGraphMap<NodeId, ()>,
    ambiguous_with_all: HashSet<NodeId>,

    default_set: Option<BoxedSystemSet>,
    next_node_id: u64,
    changed: bool,
    uninit: Vec<(NodeId, UninitNode)>,
}

impl ScheduleMeta {
    pub fn new() -> Self {
        Self {
            system_set_ids: HashMap::new(),
            system_sets: HashMap::new(),
            systems: HashMap::new(),
            conditions: HashMap::new(),
            hierarchy: Dag::new(),
            dependency: Dag::new(),
            dependency_flattened: Dag::new(),
            ambiguous_with: UnGraphMap::new(),
            ambiguous_with_flattened: UnGraphMap::new(),
            ambiguous_with_all: HashSet::new(),
            default_set: None,
            next_node_id: 0,
            changed: false,
            uninit: Vec::new(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_node_id;
        self.next_node_id = self.next_node_id.checked_add(1).unwrap();
        id
    }

    fn set_default_set(&mut self, set: impl SystemSet) {
        assert!(!set.is_system_type(), "invalid use of system type set");
        self.default_set = Some(set.dyn_clone());
    }

    fn add_systems<P>(&mut self, systems: impl IntoSystemConfigs<P>) {
        let SystemConfigs { systems, chained } = systems.into_configs();
        let mut iter = systems
            .into_iter()
            .map(|system| self.add_system_inner(system).unwrap());

        if chained {
            let ids = iter.collect::<Vec<_>>();
            self.chain(ids);
        } else {
            while iter.next().is_some() {}
        }
    }

    fn add_system<P>(&mut self, system: impl IntoSystemConfig<P>) {
        self.add_system_inner(system).unwrap();
    }

    fn add_system_inner<P>(
        &mut self,
        system: impl IntoSystemConfig<P>,
    ) -> Result<NodeId, BuildError> {
        let SystemConfig {
            system,
            mut graph_info,
            conditions,
        } = system.into_config();

        let id = NodeId::System(self.next_id());

        if graph_info.sets.is_empty() {
            if let Some(default) = self.default_set.as_ref() {
                graph_info.sets.insert(default.dyn_clone());
            }
        }

        // graph updates are immediate
        self.update_graphs(id, graph_info)?;

        // system init has to be deferred (need `&mut World`)
        self.uninit
            .push((id, UninitNode::System(system, conditions)));
        self.changed = true;

        Ok(id)
    }

    fn configure_sets(&mut self, sets: impl IntoSystemSetConfigs) {
        let SystemSetConfigs { sets, chained } = sets.into_configs();
        let mut iter = sets
            .into_iter()
            .map(|set| self.configure_set_inner(set).unwrap());

        if chained {
            let ids = iter.collect::<Vec<_>>();
            self.chain(ids);
        } else {
            while iter.next().is_some() {}
        }
    }

    fn configure_set(&mut self, set: impl IntoSystemSetConfig) {
        self.configure_set_inner(set).unwrap();
    }

    fn configure_set_inner(&mut self, set: impl IntoSystemSetConfig) -> Result<NodeId, BuildError> {
        let SystemSetConfig {
            set,
            mut graph_info,
            conditions,
        } = set.into_config();

        let id = match self.system_set_ids.get(&set) {
            Some(&id) => id,
            None => self.add_set(set),
        };

        // TODO: only if this is the first time configure_set has been called for this set
        if graph_info.sets.is_empty() {
            if let Some(default) = self.default_set.as_ref() {
                graph_info.sets.insert(default.dyn_clone());
            }
        }

        // graph updates are immediate
        self.update_graphs(id, graph_info)?;

        // system init has to be deferred (need `&mut World`)
        self.uninit.push((id, UninitNode::SystemSet(conditions)));
        self.changed = true;

        Ok(id)
    }

    fn add_set(&mut self, set: BoxedSystemSet) -> NodeId {
        let id = NodeId::Set(self.next_id());
        self.system_set_ids.insert(set.dyn_clone(), id);
        self.system_sets.insert(id, SystemSetMeta::new(set));
        id
    }

    fn chain(&mut self, nodes: Vec<NodeId>) {
        for pair in nodes.windows(2) {
            self.dependency.graph.add_edge(pair[0], pair[1], ());
        }
    }

    fn check_sets(&mut self, id: &NodeId, graph_info: &GraphInfo) -> Result<(), BuildError> {
        for set in graph_info.sets.iter() {
            match self.system_set_ids.get(set) {
                Some(set_id) => {
                    if id == set_id {
                        return Err(BuildError::HierarchyLoop(set.dyn_clone()));
                    }
                }
                None => {
                    self.add_set(set.dyn_clone());
                }
            }
        }

        Ok(())
    }

    fn check_edges(&mut self, id: &NodeId, graph_info: &GraphInfo) -> Result<(), BuildError> {
        for (_, set) in graph_info.dependencies.iter() {
            match self.system_set_ids.get(set) {
                Some(set_id) => {
                    if id == set_id {
                        return Err(BuildError::DependencyLoop(set.dyn_clone()));
                    }
                }
                None => {
                    self.add_set(set.dyn_clone());
                }
            }
        }

        if let Ambiguity::IgnoreWithSet(ambiguous_with) = &graph_info.ambiguous_with {
            for set in ambiguous_with.iter() {
                if !self.system_set_ids.contains_key(set) {
                    self.add_set(set.dyn_clone());
                }
            }
        }

        Ok(())
    }

    fn update_graphs(&mut self, id: NodeId, graph_info: GraphInfo) -> Result<(), BuildError> {
        self.check_sets(&id, &graph_info)?;
        self.check_edges(&id, &graph_info)?;

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

        for (edge_kind, set) in dependencies
            .into_iter()
            .map(|(edge_kind, set)| (edge_kind, self.system_set_ids[&set]))
        {
            let (lhs, rhs) = match edge_kind {
                DependencyEdgeKind::Before => (id, set),
                DependencyEdgeKind::After => (set, id),
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
        for (id, uninit) in self.uninit.drain(..) {
            match uninit {
                UninitNode::System(mut system, mut conditions) => {
                    debug_assert!(id.is_system());
                    system.initialize(world);
                    self.systems.insert(id, system);
                    for condition in &mut conditions {
                        condition.initialize(world);
                    }
                    self.conditions.insert(id, conditions);
                }
                UninitNode::SystemSet(mut conditions) => {
                    debug_assert!(id.is_set());
                    for condition in &mut conditions {
                        condition.initialize(world);
                    }
                    self.conditions
                        .entry(id)
                        .or_insert_with(Vec::new)
                        .append(&mut conditions);
                }
            }
        }
    }

    fn build_dependency_graph(&mut self) -> Result<(), BuildError> {
        // check hierarchy for cycles
        let hier_scc = tarjan_scc(&self.hierarchy.graph);
        if self.contains_cycles(&hier_scc) {
            self.report_cycles(&hier_scc);
            return Err(BuildError::HierarchyCycle);
        }

        self.hierarchy.topsort = hier_scc.into_iter().flatten().rev().collect::<Vec<_>>();

        let hier_results = check_graph(&self.hierarchy.graph, &self.hierarchy.topsort);
        if self.contains_hierarchy_conflicts(&hier_results.transitive_edges) {
            self.report_hierarchy_conflicts(&hier_results.transitive_edges);
            return Err(BuildError::HierarchyConflict);
        }

        // check dependencies for cycles
        let dep_scc = tarjan_scc(&self.dependency.graph);
        if self.contains_cycles(&dep_scc) {
            self.report_cycles(&dep_scc);
            return Err(BuildError::DependencyCycle);
        }

        self.dependency.topsort = dep_scc.into_iter().flatten().rev().collect::<Vec<_>>();

        // nodes can have dependent XOR hierarchical relationship
        let dep_results = check_graph(&self.dependency.graph, &self.dependency.topsort);
        for &(a, b) in dep_results.connected.iter() {
            if hier_results.connected.contains(&(a, b)) || hier_results.connected.contains(&(b, a))
            {
                let name_a = self.get_node_name(&a);
                let name_b = self.get_node_name(&b);
                return Err(BuildError::CrossDependency(name_a.into(), name_b.into()));
            }
        }

        // map system sets to all their member systems
        let mut systems = HashMap::with_capacity(self.system_sets.len());
        // iterate in reverse topological order (bottom-up)
        for &id in self.hierarchy.topsort.iter().rev() {
            if id.is_system() {
                continue;
            }

            let set = id;
            systems.insert(set, Vec::new());

            for child in self
                .hierarchy
                .graph
                .neighbors_directed(set, Direction::Outgoing)
            {
                match child {
                    NodeId::System(_) => {
                        systems.get_mut(&set).unwrap().push(child);
                    }
                    NodeId::Set(_) => {
                        let [sys, child_sys] = systems.get_many_mut([&set, &child]).unwrap();
                        sys.extend_from_slice(child_sys);
                    }
                }
            }
        }

        // can't depend on or be ambiguous with system type sets that have many instances
        for (&set, systems) in systems.iter() {
            if self.system_sets[&set].is_system_type() {
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
                    let type_set = self.system_sets[&set].0.dyn_clone();
                    return Err(BuildError::SystemTypeSetAmbiguity(type_set));
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
                    for &lhs_ in &systems[&lhs] {
                        dependency_flattened.add_edge(lhs_, rhs, ());
                    }
                }
                (NodeId::System(_), NodeId::Set(_)) => {
                    for &rhs_ in &systems[&rhs] {
                        dependency_flattened.add_edge(lhs, rhs_, ());
                    }
                }
                (NodeId::Set(_), NodeId::Set(_)) => {
                    for &lhs_ in &systems[&lhs] {
                        for &rhs_ in &systems[&rhs] {
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
            return Err(BuildError::DependencyCycle);
        }

        self.dependency_flattened.graph = dependency_flattened;
        self.dependency_flattened.topsort =
            flat_scc.into_iter().flatten().rev().collect::<Vec<_>>();

        let flat_results = check_graph(
            &self.dependency_flattened.graph,
            &self.dependency_flattened.topsort,
        );

        // flatten allowed ambiguities
        let mut ambiguous_with_flattened = UnGraphMap::new();
        for (lhs, rhs, _) in self.ambiguous_with.all_edges() {
            match (lhs, rhs) {
                (NodeId::System(_), NodeId::System(_)) => {
                    ambiguous_with_flattened.add_edge(lhs, rhs, ());
                }
                (NodeId::Set(_), NodeId::System(_)) => {
                    for &lhs_ in &systems[&lhs] {
                        ambiguous_with_flattened.add_edge(lhs_, rhs, ());
                    }
                }
                (NodeId::System(_), NodeId::Set(_)) => {
                    for &rhs_ in &systems[&rhs] {
                        ambiguous_with_flattened.add_edge(lhs, rhs_, ());
                    }
                }
                (NodeId::Set(_), NodeId::Set(_)) => {
                    for &lhs_ in &systems[&lhs] {
                        for &rhs_ in &systems[&rhs] {
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

            let system_a = self.systems.get(&a).unwrap();
            let system_b = self.systems.get(&b).unwrap();
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
            return Err(BuildError::Ambiguity);
        }

        Ok(())
    }

    fn build_schedule(&mut self) -> SystemSchedule {
        let sys_count = self.systems.len();
        let node_count = self.systems.len() + self.system_sets.len();

        let dg_system_ids = self.dependency_flattened.topsort.clone();
        let dg_system_idx_map = dg_system_ids
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect::<HashMap<_, _>>();

        // get the number of dependencies and the immediate dependents of each system
        // (needed to run systems in the correct order)
        let mut system_deps = Vec::with_capacity(sys_count);
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

            system_deps.push((num_dependencies, dependents));
        }

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
                id.is_set() && self.conditions.get(&id).filter(|v| !v.is_empty()).is_some()
            })
            .unzip();

        let set_count = hg_set_ids.len();
        let result = check_graph(&self.hierarchy.graph, &self.hierarchy.topsort);

        // get the rows and columns of the hierarchy graph's reachability matrix
        // (needed to we can evaluate conditions in the correct order)
        let mut sets_of_sets = vec![FixedBitSet::with_capacity(set_count); set_count];
        for (i, &row) in hg_set_idxs.iter().enumerate() {
            let bitset = &mut sets_of_sets[i];
            for (idx, &col) in hg_set_idxs.iter().enumerate().skip(i) {
                let is_descendant = result.reachable[index(row, col, node_count)];
                bitset.set(idx, is_descendant);
            }
        }

        let mut systems_of_sets = vec![FixedBitSet::with_capacity(sys_count); set_count];
        for (i, &row) in hg_set_idxs.iter().enumerate() {
            let bitset = &mut systems_of_sets[i];
            for &(col, sys_id) in &hg_systems {
                let idx = dg_system_idx_map[&sys_id];
                let is_descendant = result.reachable[index(row, col, node_count)];
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
                let is_ancestor = result.reachable[index(row, col, node_count)];
                bitset.set(idx, is_ancestor);
            }
        }

        SystemSchedule {
            systems: Vec::with_capacity(sys_count),
            system_conditions: Vec::with_capacity(sys_count),
            set_conditions: Vec::with_capacity(set_count),
            system_ids: dg_system_ids,
            set_ids: hg_set_ids,
            system_deps,
            sets_of_systems,
            sets_of_sets,
            systems_of_sets,
        }
    }

    fn update_schedule(&mut self, schedule: &mut SystemSchedule) -> Result<(), BuildError> {
        use std::cell::RefCell;

        if !self.uninit.is_empty() {
            return Err(BuildError::Uninitialized);
        }

        // move systems out of old schedule
        for ((id, system), conditions) in schedule
            .system_ids
            .drain(..)
            .zip(schedule.systems.drain(..))
            .zip(schedule.system_conditions.drain(..))
        {
            self.systems.insert(id, system.into_inner());
            self.conditions.insert(id, conditions.into_inner());
        }

        for (id, conditions) in schedule
            .set_ids
            .drain(..)
            .zip(schedule.set_conditions.drain(..))
        {
            self.conditions.insert(id, conditions.into_inner());
        }

        self.build_dependency_graph()?;
        *schedule = self.build_schedule();

        // move systems into new schedule
        for &id in &schedule.system_ids {
            let system = self.systems.remove(&id).unwrap();
            let conditions = self.conditions.remove(&id).unwrap();
            schedule.systems.push(RefCell::new(system));
            schedule.system_conditions.push(RefCell::new(conditions));
        }

        for &id in &schedule.set_ids {
            let conditions = self.conditions.remove(&id).unwrap();
            schedule.set_conditions.push(RefCell::new(conditions));
        }

        Ok(())
    }
}

impl ScheduleMeta {
    fn get_node_name(&self, id: &NodeId) -> Cow<'static, str> {
        match id {
            NodeId::System(_) => self.systems[id].name(),
            NodeId::Set(_) => self.system_sets[id].name(),
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
        // TODO: warn but fix with transitive reduction
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

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum InsertionError {
    #[error("schedule `{0:?}` already exists")]
    AlreadyExists(BoxedScheduleLabel),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ExtractionError {
    #[error("unknown schedule: `{0:?}`")]
    Unknown(BoxedScheduleLabel),
    #[error("schedule `{0:?}` is not available")]
    AlreadyExtracted(BoxedScheduleLabel),
    #[error("schedule `{0:?}` has not been extracted")]
    NotExtracted(BoxedScheduleLabel),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum BuildError {
    #[error("`{0:?}` contains itself")]
    HierarchyLoop(BoxedSystemSet),
    #[error("system set hierarchy contains cycle(s)")]
    HierarchyCycle,
    #[error("system set hierarchy contains conflicting relationships")]
    HierarchyConflict,
    #[error("`{0:?}` depends on itself")]
    DependencyLoop(BoxedSystemSet),
    #[error("dependencies contain cycle(s)")]
    DependencyCycle,
    #[error("`{0:?}` and `{1:?}` can have a hierarchical OR dependent relationship, not both")]
    CrossDependency(String, String),
    #[error("ambiguous relationship with `{0:?}`, multiple instances of this system exist")]
    SystemTypeSetAmbiguity(BoxedSystemSet),
    #[error("systems with conflicting access have indeterminate execution order")]
    Ambiguity,
    #[error("schedule not initialized")]
    Uninitialized,
}
