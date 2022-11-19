use crate::schedule_v3::set::*;
use bevy_utils::{
    petgraph::{graphmap::NodeTrait, prelude::*},
    HashMap, HashSet,
};

use fixedbitset::FixedBitSet;

use std::fmt::Debug;

/// Unique identifier for a system or system set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum NodeId {
    System(u64),
    Set(u64),
}

impl NodeId {
    /// Returns `true` if the identified node is a system.
    pub const fn is_system(&self) -> bool {
        matches!(self, NodeId::System(_))
    }

    /// Returns `true` if the identified node is a system set.
    pub const fn is_set(&self) -> bool {
        matches!(self, NodeId::Set(_))
    }
}

/// Specifies what kind of edge should be inserted in the dependency graph.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub(crate) enum DependencyEdgeKind {
    /// A node that should be preceded.
    Before,
    /// A node that should be succeeded.
    After,
}

/// Configures ambiguity detection for a single system.
#[derive(Clone, Debug, Default)]
pub(crate) enum Ambiguity {
    #[default]
    Check,
    /// Ignore warnings with systems in any of these system sets.
    IgnoreWithSet(HashSet<BoxedSystemSet>),
    /// Ignore all warnings.
    IgnoreAll,
}

#[derive(Clone)]
pub(crate) struct GraphInfo {
    pub(crate) sets: HashSet<BoxedSystemSet>,
    pub(crate) dependencies: HashSet<(DependencyEdgeKind, BoxedSystemSet)>,
    pub(crate) ambiguous_with: Ambiguity,
}

#[derive(Clone)]
pub(crate) struct IndexedGraphInfo {
    pub(crate) sets: HashSet<NodeId>,
    pub(crate) edges: HashSet<(DependencyEdgeKind, NodeId)>,
}

/// Converts 2D row-major pair of indices into a 1D array index.
pub(crate) fn index(row: usize, col: usize, num_cols: usize) -> usize {
    debug_assert!(col < num_cols);
    (row * num_cols) + col
}

/// Converts a 1D array index into a 2D row-major pair of indices.
pub(crate) fn row_col(index: usize, num_cols: usize) -> (usize, usize) {
    (index / num_cols, index % num_cols)
}

pub(crate) struct CheckGraphResults<V> {
    // Pairs of nodes that have a path connecting them.
    pub(crate) connected: HashSet<(V, V)>,
    // Pairs of nodes that don't have a path connecting them.
    pub(crate) disconnected: HashSet<(V, V)>,
    // Edges that are redundant because a longer path exists.
    pub(crate) transitive_edges: Vec<(V, V)>,
    // Boolean reachability matrix for the graph.
    pub(crate) reachable: FixedBitSet,
    // Variant of the graph with the fewest possible edges.
    pub(crate) tred: DiGraphMap<V, ()>,
    // Variant of the graph with the most possible edges.
    pub(crate) tcls: DiGraphMap<V, ()>,
}

impl<V: NodeTrait + Debug> Default for CheckGraphResults<V> {
    fn default() -> Self {
        Self {
            connected: HashSet::new(),
            disconnected: HashSet::new(),
            transitive_edges: Vec::new(),
            reachable: FixedBitSet::new(),
            tred: DiGraphMap::new(),
            tcls: DiGraphMap::new(),
        }
    }
}

pub(crate) fn check_graph<V>(
    graph: &DiGraphMap<V, ()>,
    topological_order: &[V],
) -> CheckGraphResults<V>
where
    V: NodeTrait + Debug,
{
    if graph.node_count() == 0 {
        return CheckGraphResults::default();
    }

    let n = graph.node_count();
    let mut map = HashMap::with_capacity(n);
    let mut tsorted = DiGraphMap::<V, ()>::new();
    // iterate nodes in topological order
    for (i, &node) in topological_order.iter().enumerate() {
        map.insert(node, i);
        tsorted.add_node(node.clone());
        // insert nodes as successors to their predecessors
        for pred in graph.neighbors_directed(node, Direction::Incoming) {
            tsorted.add_edge(pred, node, ());
        }
    }

    let mut tred = DiGraphMap::<V, ()>::new();
    let mut tcls = DiGraphMap::<V, ()>::new();

    let mut connected = HashSet::new();
    let mut disconnected = HashSet::new();
    let mut transitive_edges = Vec::new();

    let mut visited = FixedBitSet::with_capacity(n);
    let mut reachable = FixedBitSet::with_capacity(n * n);

    // iterate nodes in topological order
    for node in tsorted.nodes() {
        tred.add_node(node);
        tcls.add_node(node);
    }

    // iterate nodes in reverse topological order
    for a in tsorted.nodes().rev() {
        let index_a = *map.get(&a).unwrap();
        // iterate their successors in topological order
        for b in tsorted.neighbors_directed(a, Direction::Outgoing) {
            let index_b = *map.get(&b).unwrap();
            debug_assert!(index_a < index_b);
            if !visited[index_b] {
                // edge <a, b> is not redundant
                tred.add_edge(a, b, ());
                tcls.add_edge(a, b, ());
                reachable.set(index(index_a, index_b, n), true);

                let successors = tcls
                    .neighbors_directed(b, Direction::Outgoing)
                    .collect::<Vec<_>>();
                for c in successors.into_iter() {
                    let index_c = *map.get(&c).unwrap();
                    debug_assert!(index_b < index_c);
                    if !visited[index_c] {
                        visited.set(index_c, true);
                        tcls.add_edge(a, c, ());
                        reachable.set(index(index_a, index_c, n), true);
                    }
                }
            } else {
                transitive_edges.push((a, b));
            }
        }

        visited.clear();
    }

    for i in 0..(n - 1) {
        // reachable is upper triangular because the nodes are in topological order
        for index in index(i, i + 1, n)..=index(i, n - 1, n) {
            let (a, b) = row_col(index, n);
            let pair = (topological_order[a], topological_order[b]);
            if !reachable[index] {
                disconnected.insert(pair);
            } else {
                connected.insert(pair);
            }
        }
    }

    // Fill diagonal.
    for i in 0..n {
        reachable.set(index(i, i, n), true);
    }

    CheckGraphResults {
        connected,
        disconnected,
        transitive_edges,
        reachable,
        tred,
        tcls,
    }
}
