#![allow(warnings)]
//! Tools for controlling system execution.

mod condition;
mod descriptor;
mod graph_utils;
mod runner;
mod state;

pub use self::condition::*;
pub use self::descriptor::*;
use self::graph_utils::*;
pub use self::runner::*;
pub use self::state::*;

use crate::{
    change_detection::CHECK_TICK_THRESHOLD,
    component::ComponentId,
    query::Access,
    schedule::SystemLabelId,
    system::{AsSystemLabel, BoxedSystem},
    world::{World, WorldId},
};

use bevy_utils::{
    tracing::{error, warn},
    HashMap, HashSet,
};

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::VecDeque,
    fmt::{Debug, Write},
};

use fixedbitset::FixedBitSet;
use petgraph::{
    algo::tarjan_scc,
    dot::{Config as DotConfig, Dot},
    prelude::*,
};
use thiserror::Error;

enum GraphType {
    Dependency,
    Hierarchy,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rebuild {
    /// Both graph and schedule are up-to-date.
    None,
    /// Graph is up-to-date; schedule needs to be rebuilt.
    Schedule,
    /// Both graph and schedule need to be rebuilt.
    Graph,
}

/// A schematic for running a group of systems on a world.
pub(crate) struct DependencyGraph {
    /// Union of all component access from this set's descendants.
    component_access: Access<ComponentId>,
    /// The dependency graph.
    base: DiGraphMap<NodeId, ()>,
    /// A flattened version of the dependency graph (only system nodes and system-system edges).
    flat: DiGraphMap<NodeId, ()>,
    /// Cached topological ordering of `base`.
    base_topsort: Vec<NodeId>,
    /// Cached topological ordering of `flat`.
    flat_topsort: Vec<NodeId>,
    /// Says if the dependency graph (or just the schedule) needs to be rebuilt.
    rebuild: Rebuild,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self {
            component_access: Access::default(),
            base: DiGraphMap::new(),
            flat: DiGraphMap::new(),
            base_topsort: vec![],
            flat_topsort: vec![],
            rebuild: Rebuild::None,
        }
    }
}

/// Cached, executable form of a system set.
pub struct Schedule {
    // All elements are sorted in topological order.
    systems: Vec<RefCell<BoxedSystem>>,
    system_conditions: Vec<RefCell<Vec<BoxedRunCondition>>>,
    set_conditions: Vec<RefCell<Vec<BoxedRunCondition>>>,
    system_ids: Vec<NodeId>,
    set_ids: Vec<NodeId>,
    system_deps: Vec<(usize, Vec<usize>)>,
    sets_of_systems: Vec<FixedBitSet>,
    systems_of_sets: Vec<FixedBitSet>,
}

// SAFETY: the cell-wrapped data is only accessed by the runner thread
unsafe impl Sync for Schedule {}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            systems: vec![],
            system_conditions: vec![],
            set_conditions: vec![],
            system_ids: vec![],
            set_ids: vec![],
            system_deps: vec![],
            sets_of_systems: vec![],
            systems_of_sets: vec![],
        }
    }
}

/// A collection of systems and their dependency graphs.
pub struct Systems {
    world_id: Option<WorldId>,
    next_id: u64,

    index: HashMap<SystemLabelId, NodeId>,
    systems: HashMap<NodeId, Option<BoxedSystem>>,
    conditions: HashMap<NodeId, Option<Vec<BoxedRunCondition>>>,
    schedules: HashMap<NodeId, Option<Schedule>>,
    graphs: HashMap<NodeId, DependencyGraph>,

    hier: DiGraphMap<NodeId, ()>,
    uninit_nodes: Vec<NodeInfo>,
}

impl Default for Systems {
    fn default() -> Self {
        Self {
            world_id: None,
            next_id: 0,

            index: HashMap::new(),
            systems: HashMap::new(),
            conditions: HashMap::new(),
            schedules: HashMap::new(),
            graphs: HashMap::new(),

            hier: DiGraphMap::new(),
            uninit_nodes: Vec::new(),
        }
    }
}

// public API
impl Systems {
    /// Constructs an empty `Systems`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a system.
    pub fn add_system<P>(&mut self, system: impl IntoScheduledSystem<P>) {
        assert!(
            self.systems.len() <= CHECK_TICK_THRESHOLD as usize,
            "too many systems"
        );

        let ScheduledSystem {
            system,
            mut info,
            conditions,
        } = system.schedule();

        if info.name().is_none() {
            info.name = Some(*system.default_labels().first().unwrap());
        }

        let name = info.name().unwrap();
        assert!(!self.index.contains_key(name), "name already used");

        let id = NodeId::System(self.next_id);
        self.next_id.checked_add(1).expect("integer overflow");

        self.index.insert(*name, id);
        self.systems.insert(id, Some(system));
        self.conditions.insert(id, Some(conditions));
        self.uninit_nodes.push(info);
    }

    /// Adds a system set.
    pub fn add_set(&mut self, system_set: impl IntoScheduledSystemSet) {
        let ScheduledSystemSet { info, conditions } = system_set.schedule();
        let name = info.name().unwrap();
        assert!(!self.index.contains_key(name), "name already used");

        let id = NodeId::Set(self.next_id);
        self.next_id.checked_add(1).expect("integer overflow");

        self.index.insert(*name, id);
        self.conditions.insert(id, Some(conditions));
        self.graphs.insert(id, DependencyGraph::default());
        self.schedules.insert(id, None);
        self.uninit_nodes.push(info);
    }

    /// Adds multiple systems and system sets at once and returns their identifiers.
    pub fn add_systems(&mut self, nodes: impl IntoIterator<Item = Scheduled>) {
        nodes.into_iter().map(|node| match node {
            Scheduled::System(system) => self.add_system(system),
            Scheduled::Set(set) => self.add_set(set),
        });
    }

    /// Returns `true` if a system or system set named `label` exists.
    pub fn contains<M>(&self, label: impl AsSystemLabel<M>) -> bool {
        self.index.contains_key(&label.as_system_label())
    }
}

// internal API
impl Systems {
    fn update<M>(&mut self, label: impl AsSystemLabel<M>, world: &mut World) -> Result<()> {
        if let Some(world_id) = self.world_id {
            assert!(world_id == world.id(), "wrong world");
        } else {
            self.world_id = Some(world.id());
        }

        if !self.uninit_nodes.is_empty() {
            self.initialize(world)?;
        }

        let node = self.index[&label.as_system_label()];
        if self.graphs.contains_key(&node) {
            self.rebuild_dependency_graph(node)?;
        }

        Ok(())
    }

    fn initialize(&mut self, world: &mut World) -> Result<()> {
        // check for errors
        for info in self.uninit_nodes.iter() {
            for set_label in info.sets().iter() {
                if info.name() == Some(set_label) {
                    return Err(ScheduleBuildError::HierarchyLoop);
                }
                if self.index.get(set_label).is_none() {
                    return Err(ScheduleBuildError::UnknownSetLabel);
                }
                if self.index.get(set_label).unwrap().is_system() {
                    return Err(ScheduleBuildError::InvalidSetLabel);
                }
            }
            for (_, node_label) in info.edges().iter() {
                if info.name() == Some(node_label) {
                    return Err(ScheduleBuildError::DependencyLoop);
                }
                if self.index.get(node_label).is_none() {
                    return Err(ScheduleBuildError::UnknownDependency);
                }
            }
        }

        // convert labels to integers
        let indexed = self
            .uninit_nodes
            .drain(..)
            .map(|info| {
                let node = self.index[info.name().unwrap()];
                let sets = info
                    .sets()
                    .iter()
                    .map(|label| self.index[label])
                    .collect::<HashSet<_>>();
                let edges = info
                    .edges()
                    .iter()
                    .map(|(order, label)| (*order, self.index[label]))
                    .collect::<Vec<_>>();

                (node, IndexedNodeInfo { sets, edges })
            })
            .collect::<HashMap<_, _>>();

        // initialize systems
        for (node, _info) in indexed.iter() {
            if node.is_system() {
                let system = self.systems.get_mut(&node).unwrap();
                system.as_mut().unwrap().initialize(world);
            }

            let conditions = self.conditions.get_mut(&node).unwrap();
            conditions
                .iter_mut()
                .flatten()
                .for_each(|system| system.initialize(world));
        }

        // add nodes to graphs
        for (&node, info) in indexed.iter() {
            self.hier.add_node(node);
            for &set in info.sets().iter() {
                self.hier.add_edge(set, node, ());
            }

            for &set in info.sets().iter() {
                let graph = self.graphs.get_mut(&set).unwrap();
                graph.base.add_node(node);
            }

            for &(order, other_node) in info.edges().iter() {
                let (a, b) = match order {
                    Order::Before => (node, other_node),
                    Order::After => (other_node, node),
                };

                let other_sets = indexed[&other_node].sets();
                if info.sets().is_disjoint(other_sets) {
                    // TODO: detect when satisfiable
                    return Err(ScheduleBuildError::CrossDependency);
                } else {
                    for set in info.sets().intersection(other_sets) {
                        let graph = self.graphs.get_mut(&set).unwrap();
                        graph.base.add_edge(a, b, ());
                    }
                }
            }
        }

        for (&node, info) in indexed.iter() {
            self.mark_dependency_graphs_for_rebuild(node);
        }

        Ok(())
    }

    fn mark_dependency_graphs_for_rebuild(&mut self, node: NodeId) {
        let mut bfs_queue = VecDeque::new();
        bfs_queue.push_back(node);
        let mut visited = HashSet::new();
        visited.insert(node);

        while let Some(node) = bfs_queue.pop_front() {
            if node.is_set() {
                let graph = self.graphs.get_mut(&node).unwrap();
                match graph.rebuild {
                    Rebuild::Graph => continue,
                    _ => graph.rebuild = Rebuild::Graph,
                }
            }

            for parent in self.hier.neighbors_directed(node, Direction::Incoming) {
                assert!(parent.is_set());
                if !visited.contains(&parent) {
                    visited.insert(parent);
                    bfs_queue.push_back(parent);
                }
            }
        }
    }

    fn rebuild_dependency_graph(&mut self, set: NodeId) -> Result<()> {
        assert!(set.is_set());
        let graph = &self.graphs[&set];
        if graph.rebuild == Rebuild::None {
            return Ok(());
        }

        // construct sub-hierarchy of nodes rooted at `set` using BFS
        let mut hier = DiGraphMap::<NodeId, ()>::new();
        let mut bfs_queue = VecDeque::new();
        bfs_queue.push_back(set);
        let mut visited = HashSet::new();
        visited.insert(set);

        while let Some(node) = bfs_queue.pop_front() {
            for child in self.hier.neighbors_directed(node, Direction::Outgoing) {
                hier.add_edge(node, child, ());
                if child.is_set() {
                    if !visited.contains(&child) {
                        visited.insert(child);
                        bfs_queue.push_back(child);
                    }
                }
            }
        }

        // topsort
        let hier_scc = tarjan_scc(&hier);
        self.check_graph_cycles(set, &hier, &hier_scc, GraphType::Hierarchy)?;
        let hier_topsort = hier_scc.into_iter().flatten().rev().collect::<Vec<_>>();
        let hier_topsort_sets = hier_topsort
            .iter()
            .cloned()
            .filter_map(|node| if node.is_set() { Some(node) } else { None })
            .collect::<Vec<_>>();
        let result = check_graph(&hier, &hier_topsort);

        if graph.rebuild == Rebuild::Graph {
            self.check_hierarchy(&result.transitive_edges)?;

            // no paths should exist between a pair of sets that share nodes
            let mut pairs = HashMap::new();
            for node in hier.nodes() {
                let parents = hier
                    .neighbors_directed(node, Direction::Incoming)
                    .collect::<Vec<_>>();

                for (i, &a) in parents.iter().enumerate() {
                    assert!(a.is_set());
                    for &b in parents.iter().skip(i + 1) {
                        pairs
                            .entry(sort_pair(a, b))
                            .or_insert_with(HashSet::new)
                            .insert(node);
                    }
                }
            }
            self.check_ambiguous(&hier, &pairs, &result.ambiguities)?;

            let mut updates = Vec::with_capacity(hier_topsort_sets.len());
            for &set in hier_topsort_sets.iter().rev() {
                let graph = &self.graphs[&set];
                if graph.rebuild != Rebuild::Graph {
                    updates.push(None);
                    continue;
                }

                let base_scc = tarjan_scc(&graph.base);
                self.check_graph_cycles(set, &graph.base, &base_scc, GraphType::Dependency)?;

                let mut access = Access::default();
                let mut flat = DiGraphMap::<NodeId, ()>::new();
                for child in graph.base.nodes() {
                    match child {
                        NodeId::System(_) => {
                            let system = &self.systems[&child];
                            access.extend(&system.as_ref().unwrap().component_access());
                            flat.add_node(child);
                        }
                        NodeId::Set(_) => {
                            let subgraph = &self.graphs[&child];
                            access.extend(&subgraph.component_access);
                            flat.extend(subgraph.flat.all_edges());
                        }
                    }
                }

                for (a, b, _) in graph.base.all_edges() {
                    match (a, b) {
                        (NodeId::System(_), NodeId::System(_)) => {
                            flat.add_edge(a, b, ());
                        }
                        (NodeId::Set(_), NodeId::System(_)) => {
                            for aa in self.graphs[&a].flat.nodes() {
                                flat.add_edge(aa, b, ());
                            }
                        }
                        (NodeId::System(_), NodeId::Set(_)) => {
                            for bb in self.graphs[&b].flat.nodes() {
                                flat.add_edge(a, bb, ());
                            }
                        }
                        (NodeId::Set(_), NodeId::Set(_)) => {
                            for aa in self.graphs[&a].flat.nodes() {
                                for bb in self.graphs[&b].flat.nodes() {
                                    flat.add_edge(aa, bb, ());
                                }
                            }
                        }
                    }
                }

                let flat_scc = tarjan_scc(&flat);
                self.check_graph_cycles(set, &flat, &flat_scc, GraphType::Dependency)?;

                updates.push(Some((access, flat)));
            }

            for (&set, update) in hier_topsort_sets.iter().rev().zip(updates) {
                if let Some((access, flat)) = update {
                    let graph = self.graphs.get_mut(&set).unwrap();
                    graph.component_access = access;
                    graph.flat = flat;
                    graph.rebuild == Rebuild::Schedule;
                }
            }
        }

        let graph = &self.graphs[&set];
        debug_assert!(matches!(graph.rebuild, Rebuild::Schedule));
        let sys_count = graph.flat.node_count();
        let set_count = hier.node_count();

        let topsort_systems = graph.flat_topsort.clone();
        let topsort_sys_idx_map = topsort_systems
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, sys_id)| (sys_id, i))
            .collect::<HashMap<_, _>>();

        let (topsort_set_idxs, topsort_sets): (Vec<_>, Vec<_>) = hier_topsort
            .iter()
            .cloned()
            .enumerate()
            .filter_map(|(i, node)| if node.is_set() { Some((i, node)) } else { None })
            .unzip();
        let topsort_set_idx_map = topsort_sets
            .iter()
            .cloned()
            .zip(topsort_set_idxs.iter().cloned())
            .collect::<HashMap<_, _>>();

        // get the number of dependencies and the immediate dependents of each system
        // (needed to run systems in the correct order)
        let mut system_deps = Vec::with_capacity(sys_count);
        for &sys in topsort_systems.iter() {
            let num_dependencies = graph
                .flat
                .neighbors_directed(sys, Direction::Incoming)
                .count();
            let dependents = graph
                .flat
                .neighbors_directed(sys, Direction::Outgoing)
                .map(|other_sys| topsort_sys_idx_map[&other_sys])
                .collect::<Vec<_>>();
            system_deps.push((num_dependencies, dependents));
        }

        // get the descendant systems of each system set
        // (needed to skip systems when system set conditions return false)
        let mut systems_of_sets = Vec::with_capacity(set_count);
        for set in topsort_sets.iter() {
            let mut bitset = FixedBitSet::with_capacity(sys_count);
            let graph = &self.graphs[&set];
            for sys in graph.flat_topsort.iter() {
                let sys_idx = topsort_sys_idx_map[&sys];
                bitset.set(sys_idx, true);
            }
            systems_of_sets.push(bitset);
        }

        // get the ancestor sets of each system set
        // (needed to get the ancestor sets of each system)
        let mut sets_of_sets = Vec::with_capacity(set_count);
        // result's nodes are indexed in topological order, so its reachability matrix
        // is upper triangular (and the bitsets we want are its columns)
        for (new_col, &old_col) in topsort_set_idxs.iter().enumerate() {
            let mut bitset = FixedBitSet::with_capacity(set_count);
            for (new_row, &old_row) in topsort_set_idxs.iter().enumerate().take(new_col) {
                let is_ancestor = result.reachable[index(old_row, old_col, sys_count + set_count)];
                bitset.set(new_row, is_ancestor);
            }
            sets_of_sets.push(bitset);
        }

        // get the ancestor sets of each system
        // (needed to evaluate conditions in the correct order)
        let mut sets_of_systems = Vec::with_capacity(sys_count);
        for (sys_idx, &sys) in topsort_systems.iter().enumerate() {
            let mut bitset = FixedBitSet::with_capacity(set_count);
            for set in hier.neighbors_directed(sys, Direction::Incoming) {
                assert!(set.is_set());
                let set_idx = topsort_set_idx_map[&set];
                bitset.union_with(&sets_of_sets[set_idx]);
            }
            sets_of_systems.push(bitset);
        }

        let schedule = Schedule {
            systems: Vec::with_capacity(sys_count),
            system_conditions: Vec::with_capacity(sys_count),
            set_conditions: Vec::with_capacity(set_count),
            system_ids: topsort_systems,
            set_ids: topsort_sets,
            system_deps,
            sets_of_systems,
            systems_of_sets,
        };
        self.schedules.insert(set, Some(schedule));

        let graph = self.graphs.get_mut(&set).unwrap();
        graph.rebuild == Rebuild::None;

        Ok(())
    }
}

impl Systems {
    pub fn export_system<M>(&mut self, label: impl AsSystemLabel<M>) -> BoxedSystem {
        let node = self.index[&label.as_system_label()];
        assert!(node.is_system());
        let mut system = self.systems.get_mut(&node).unwrap().take().unwrap();
        system
    }

    pub fn import_system<M>(&mut self, label: impl AsSystemLabel<M>, system: BoxedSystem) {
        let node = self.index[&label.as_system_label()];
        assert!(node.is_system());
        self.systems
            .get_mut(&node)
            .map(|container| container.insert(system));
    }

    pub fn export_schedule<M>(&mut self, label: impl AsSystemLabel<M>) -> Schedule {
        let node = self.index[&label.as_system_label()];
        assert!(node.is_set());
        assert!(matches!(self.graphs[&node].rebuild, Rebuild::None));

        let mut schedule = self.schedules.get_mut(&node).unwrap().take().unwrap();
        assert!(schedule.systems.is_empty());
        assert!(schedule.system_conditions.is_empty());
        assert!(schedule.set_conditions.is_empty());

        for sys_id in schedule.system_ids.iter() {
            let system = self.systems.get_mut(&sys_id).unwrap().take().unwrap();
            schedule.systems.push(RefCell::new(system));

            let conditions = self.conditions.get_mut(&sys_id).unwrap().take().unwrap();
            schedule.system_conditions.push(RefCell::new(conditions));
        }

        for set_id in schedule.set_ids.iter() {
            let conditions = self.conditions.get_mut(&set_id).unwrap().take().unwrap();
            schedule.set_conditions.push(RefCell::new(conditions));
        }

        schedule
    }

    pub fn import_schedule<M>(&mut self, label: impl AsSystemLabel<M>, mut schedule: Schedule) {
        let node = self.index[&label.as_system_label()];
        assert!(node.is_set());

        let sys_iter = schedule
            .system_ids
            .iter()
            .zip(schedule.systems.drain(..))
            .zip(schedule.system_conditions.drain(..))
            .map(|((a, b), c)| (a, b, c));

        let set_iter = schedule
            .set_ids
            .iter()
            .zip(schedule.set_conditions.drain(..));

        // If graph nodes associated with this schedule were removed while it was running,
        // their boxed systems will just be dropped here.
        // There is no risk of overwriting valid data since all nodes are unique (`u64`).
        for (sys, system, conditions) in sys_iter {
            self.systems
                .get_mut(&sys)
                .map(|container| container.insert(system.into_inner()));

            self.conditions
                .get_mut(&sys)
                .map(|container| container.insert(conditions.into_inner()));
        }

        for (set, conditions) in set_iter {
            self.conditions
                .get_mut(&set)
                .map(|container| container.insert(conditions.into_inner()));
        }

        self.schedules
            .get_mut(&node)
            .map(|container| container.insert(schedule));
    }
}

// error checking methods
impl Systems {
    fn get_node_name(&self, id: NodeId) -> Cow<'static, str> {
        match id {
            NodeId::System(_) => "some system".into(),
            NodeId::Set(_) => "some set".into(),
        }
    }

    fn check_hierarchy(&self, transitive_edges: &Vec<(NodeId, NodeId)>) -> Result<()> {
        if transitive_edges.is_empty() {
            return Ok(());
        }

        let mut message = String::from("system set hierarchy contains redundant edge(s)");
        for &(parent, child) in transitive_edges.iter() {
            writeln!(
                message,
                " -- {:?} '{:?}' is already under set '{:?}' from a longer path",
                child.type_str(),
                self.get_node_name(child),
                self.get_node_name(parent),
            )
            .unwrap();
        }

        error!("{}", message);
        Err(ScheduleBuildError::InvalidHierarchy)
    }

    fn check_ambiguous(
        &self,
        graph: &DiGraphMap<NodeId, ()>,
        pairs: &HashMap<(NodeId, NodeId), HashSet<NodeId>>,
        ambiguities: &HashSet<(NodeId, NodeId)>,
    ) -> Result<()> {
        let required_ambiguous = pairs.keys().cloned().collect();
        let actual_ambiguous = ambiguities
            .iter()
            .filter_map(|&(a, b)| {
                if a.is_set() && b.is_set() {
                    Some(sort_pair(a, b))
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        if actual_ambiguous.is_superset(&required_ambiguous) {
            return Ok(());
        }

        let mut message = String::from("path found between intersecting system sets");
        for &(a, b) in required_ambiguous.difference(&actual_ambiguous) {
            let shared_nodes = pairs.get(&(a, b)).unwrap();
            writeln!(
                message,
                " -- '{:?}' and '{:?}' share these nodes: {:?}",
                a, b, shared_nodes,
            )
            .unwrap();
        }

        error!("{}", message);
        Err(ScheduleBuildError::MissingAmbiguity)
    }

    fn check_conflicts(
        &self,
        pairs: &HashMap<(NodeId, NodeId), HashSet<NodeId>>,
        ambiguities: &HashSet<(NodeId, NodeId)>,
        set_id: NodeId,
        world: &World,
    ) -> Result<()> {
        let get_combined_access = |id| {
            let mut access = Access::default();
            match id {
                NodeId::System(_) => {
                    let system = &self.systems[&id];
                    access.extend(system.as_ref().unwrap().component_access());
                    for system in self.conditions[&id].iter().flatten() {
                        access.extend(system.component_access());
                    }
                }
                NodeId::Set(_) => {
                    access.extend(&self.graphs[&id].component_access);
                    for system in self.conditions[&id].iter().flatten() {
                        access.extend(system.component_access());
                    }
                }
            }

            access
        };

        let mut conflicting_pairs = vec![];
        for &(a, b) in ambiguities.iter() {
            if a.is_set() && b.is_set() && pairs.contains_key(&sort_pair(a, b)) {
                // NOTE: nodes in symmetric difference could have conflicts
                continue;
            }

            let conflicts = get_combined_access(a).get_conflicts(&get_combined_access(b));
            if !conflicts.is_empty() {
                conflicting_pairs.push((a, b, conflicts));
            }
        }

        if conflicting_pairs.is_empty() {
            return Ok(());
        }

        let mut message = String::new();
        writeln!(
            message,
            "system set '{:?}' contains {} pairs of nodes with unknown order and conflicting access",
            self.get_node_name(set_id),
            conflicting_pairs.len(),
        )
        .unwrap();

        for (i, (a, b, conflicts)) in conflicting_pairs.iter().enumerate() {
            let mut component_ids = conflicts
                .iter()
                .map(|id| world.components().get_info(*id).unwrap().name())
                .collect::<Vec<_>>();
            component_ids.sort_unstable();

            writeln!(
                message,
                " -- {}: {} {:?} and {} {:?} conflict on these components: {:?}",
                i,
                a.type_str(),
                self.get_node_name(*a),
                b.type_str(),
                self.get_node_name(*b),
                component_ids,
            )
            .unwrap();
        }

        warn!("{}", message);
        Err(ScheduleBuildError::Ambiguity)
    }

    fn check_graph_cycles(
        &self,
        set: NodeId,
        graph: &DiGraphMap<NodeId, ()>,
        strongly_connected_components: &Vec<Vec<NodeId>>,
        graph_type: GraphType,
    ) -> Result<()> {
        if strongly_connected_components.len() == graph.node_count() {
            return Ok(());
        }

        let lower_bound = strongly_connected_components
            .iter()
            .filter(|scc| scc.len() > 1)
            .count();

        let mut message = format!(
            "graph of system set '{:?}' contains at least {} cycle(s)",
            self.get_node_name(set),
            lower_bound,
        );
        writeln!(message, " -- these groups each contain at least 1 cycle:").unwrap();

        let iter = strongly_connected_components
            .iter()
            .filter(|scc| scc.len() > 1)
            .enumerate();

        for (i, scc) in iter {
            let ids = scc
                .iter()
                .map(|&node_id| self.get_node_name(node_id))
                .collect::<Vec<_>>();

            writeln!(message, " ---- {}: {:?}", i, ids).unwrap();
        }
        error!("{}", message);

        let error = match graph_type {
            GraphType::Dependency => ScheduleBuildError::DependencyCycle,
            GraphType::Hierarchy => ScheduleBuildError::HierarchyCycle,
        };

        Err(error)
    }
}

impl Systems {
    /// All system change ticks are scanned for risk of age overflow once the world counter
    /// has incremented at least [`CHECK_TICK_THRESHOLD`](crate::change_detection::CHECK_TICK_THRESHOLD)
    /// times since the previous `check_change_ticks` scan.
    ///
    /// During each scan, any change ticks older than [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE)
    /// are clamped to that age. This prevents false positives that would appear because of overflow.
    // TODO: parallelize
    pub fn check_change_ticks(&mut self, change_tick: u32, last_check_tick: u32) {
        if change_tick.wrapping_sub(last_check_tick) >= CHECK_TICK_THRESHOLD {
            #[cfg(feature = "trace")]
            let _span = bevy_utils::tracing::info_span!("check stored system ticks").entered();
            for system in self.systems.values_mut().flatten() {
                system.check_change_tick(change_tick);
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, ScheduleBuildError>;

/// Errors that make the system graph unsolvable (some of these can be suppressed).
#[derive(Error, Debug)]
pub enum ScheduleBuildError {
    /// A node was assigned to an unknown set.
    #[error("unknown set")]
    UnknownSetLabel,
    /// A node was ordered with an unknown node.
    #[error("unknown dependency")]
    UnknownDependency,
    /// A system's name was used as a set label.
    #[error("system label used as set label")]
    InvalidSetLabel,
    /// A set contains itself.
    #[error("set contains itself")]
    HierarchyLoop,
    /// System set hierarchy contains a cycle.
    #[error("set hierarchy contains cycle")]
    HierarchyCycle,
    /// System set hierarchy contains an invalid edge.
    #[error("set hierarchy contains transitive edge")]
    InvalidHierarchy,
    /// A node depends on itself.
    #[error("node depends on itself")]
    DependencyLoop,
    /// Dependency graph contains a cycle.
    #[error("dependency graph contains cycle")]
    DependencyCycle,
    /// Dependency graph has an edge between nodes that do not conflict.
    #[error("node depends on other node but no access conflict")]
    InvalidDependency,
    /// Dependency graph has an edge between nodes that have no set in common.
    #[error("node depends on other node under different set")]
    CrossDependency,
    /// Intersecting system sets were found to have a dependent/hierarchical relation.
    #[error("intersecting sets have dependent/hierarchical relation")]
    MissingAmbiguity,
    /// Parallel nodes have an access conflict.
    #[error("parallel nodes have access conflict")]
    Ambiguity,
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        change_detection::Mut,
        component::Component,
        schedule::SystemLabel,
        schedule_v3::*,
        system::{Local, Query, ResMut},
        world::World,
    };

    struct Order(pub Vec<usize>);

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[derive(SystemLabel)]
    enum TestSet {
        All,
        A,
        B,
        C,
        X,
    }

    #[derive(SystemLabel)]
    enum TestSystem {
        Foo,
        Bar,
        Baz,
    }

    fn exclusive(num: usize) -> impl FnMut(&mut World) {
        move |world| world.resource_mut::<Order>().0.push(num)
    }

    fn normal(num: usize) -> impl FnMut(ResMut<Order>) {
        move |mut resource: ResMut<Order>| resource.0.push(num)
    }

    #[test]
    fn system() {
        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        fn foo() {}
        systems.add_system(foo.named(TestSystem::Foo));

        let result = world.resource_scope(|world, mut systems: Mut<Systems>| {
            systems.update(TestSystem::Foo, world)
        });

        assert!(result.is_ok());
    }

    #[test]
    fn correct_order() {}

    #[test]
    fn invalid_set_label() {
        fn foo() {}
        fn bar() {}

        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::X);
        systems.add_system(foo.named(TestSystem::Foo));
        systems.add_system(foo.named(TestSystem::Bar).in_set(TestSystem::Foo));

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::X, world));

        assert!(matches!(result, Err(ScheduleBuildError::InvalidSetLabel)));
    }

    #[test]
    fn dependency_loop() {
        fn foo() {}

        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_system(foo.named(TestSystem::Foo).after(TestSystem::Foo));

        let result = world.resource_scope(|world, mut systems: Mut<Systems>| {
            systems.update(TestSystem::Foo, world)
        });

        assert!(matches!(result, Err(ScheduleBuildError::DependencyLoop)));
    }

    #[test]
    fn dependency_cycle() {
        fn foo() {}
        fn bar() {}
        fn baz() {}

        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::All);
        systems.add_system(
            foo.named(TestSystem::Foo)
                .in_set(TestSet::All)
                .after(TestSystem::Baz),
        );
        systems.add_system(
            bar.named(TestSystem::Bar)
                .in_set(TestSet::All)
                .after(TestSystem::Foo),
        );
        systems.add_system(
            baz.named(TestSystem::Baz)
                .in_set(TestSet::All)
                .after(TestSystem::Bar),
        );

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::All, world));

        assert!(matches!(result, Err(ScheduleBuildError::DependencyCycle)));
    }

    #[test]
    fn redundant_dependencies() {}

    #[test]
    fn cross_dependencies() {
        fn foo() {}
        fn bar() {}

        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::All);
        systems.add_set(TestSet::A.in_set(TestSet::All));
        systems.add_set(TestSet::B.in_set(TestSet::All));

        systems.add_system(foo.named(TestSystem::Foo).in_set(TestSet::A));
        systems.add_system(
            bar.named(TestSystem::Bar)
                .after(TestSystem::Foo)
                .in_set(TestSet::B),
        );

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::All, world));

        // Foo and Bar do not belong to the same set.
        // This isn't automatically *invalid*, but, like, just order A and B.
        assert!(matches!(result, Err(ScheduleBuildError::CrossDependency)));
    }

    #[test]
    fn hierarchy_loop() {
        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::X.in_set(TestSet::X));

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::X, world));

        assert!(matches!(result, Err(ScheduleBuildError::HierarchyLoop)));
    }

    #[test]
    fn hierarchy_invalid() {
        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::A);
        systems.add_set(TestSet::B.in_set(TestSet::A));
        systems.add_set(TestSet::C.in_set(TestSet::B).in_set(TestSet::A));

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::A, world));

        // A cannot be parent and grandparent to C at same time.
        assert!(matches!(result, Err(ScheduleBuildError::InvalidHierarchy)));
    }

    #[test]
    fn hierarchy_cycle() {
        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::A.in_set(TestSet::C));
        systems.add_set(TestSet::B.in_set(TestSet::A));
        systems.add_set(TestSet::C.in_set(TestSet::B));

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::A, world));

        assert!(matches!(result, Err(ScheduleBuildError::HierarchyCycle)));
    }

    #[test]
    fn missing_ambiguity() {
        fn foo() {}

        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::All);
        systems.add_set(TestSet::A.in_set(TestSet::All).before(TestSet::B));
        systems.add_set(TestSet::B.in_set(TestSet::All));

        systems.add_system(
            foo.named(TestSystem::Foo)
                .in_set(TestSet::A)
                .in_set(TestSet::B),
        );

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::All, world));

        // How is A supposed to run before B if Foo is in both A and B?
        assert!(matches!(result, Err(ScheduleBuildError::MissingAmbiguity)));
    }

    #[test]
    fn ambiguity() {
        fn foo_reader(_query: Query<&A>) {}
        fn bar_writer(_query: Query<&mut A>) {}

        let mut world = World::new();
        world.insert_resource(Systems::new());
        let mut systems = world.resource_mut::<Systems>();

        systems.add_set(TestSet::X);
        systems.add_system(foo_reader.named(TestSystem::Foo).in_set(TestSet::X));
        systems.add_system(bar_writer.named(TestSystem::Bar).in_set(TestSet::X));

        let result = world
            .resource_scope(|world, mut systems: Mut<Systems>| systems.update(TestSet::X, world));

        assert!(matches!(result, Err(ScheduleBuildError::Ambiguity)));
    }
}
