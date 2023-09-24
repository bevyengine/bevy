use std::fmt::Debug;

use bevy_utils::{
    petgraph::{algo::TarjanScc, graphmap::NodeTrait, prelude::*},
    HashMap, HashSet,
};
use fixedbitset::FixedBitSet;

use crate::schedule::set::*;

/// Unique identifier for a system or system set stored in a [`ScheduleGraph`].
///
/// [`ScheduleGraph`]: super::ScheduleGraph
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeId {
    /// Identifier for a system.
    System(usize),
    /// Identifier for a system set.
    Set(usize),
}

impl NodeId {
    /// Returns the internal integer value.
    pub(crate) fn index(&self) -> usize {
        match self {
            NodeId::System(index) | NodeId::Set(index) => *index,
        }
    }

    /// Returns `true` if the identified node is a system.
    pub const fn is_system(&self) -> bool {
        matches!(self, NodeId::System(_))
    }

    /// Returns `true` if the identified node is a system set.
    pub const fn is_set(&self) -> bool {
        matches!(self, NodeId::Set(_))
    }
}

/// Specifies what kind of edge should be added to the dependency graph.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub(crate) enum DependencyKind {
    /// A node that should be preceded.
    Before,
    /// A node that should be succeeded.
    After,
}

/// An edge to be added to the dependency graph.
#[derive(Clone)]
pub(crate) struct Dependency {
    pub(crate) kind: DependencyKind,
    pub(crate) set: BoxedSystemSet,
}

impl Dependency {
    pub fn new(kind: DependencyKind, set: BoxedSystemSet) -> Self {
        Self { kind, set }
    }
}

/// Configures ambiguity detection for a single system.
#[derive(Clone, Debug, Default)]
pub(crate) enum Ambiguity {
    #[default]
    Check,
    /// Ignore warnings with systems in any of these system sets. May contain duplicates.
    IgnoreWithSet(Vec<BoxedSystemSet>),
    /// Ignore all warnings.
    IgnoreAll,
}

#[derive(Clone, Default)]
pub(crate) struct GraphInfo {
    pub(crate) sets: Vec<BoxedSystemSet>,
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) ambiguous_with: Ambiguity,
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

/// Stores the results of the graph analysis.
pub(crate) struct CheckGraphResults<V> {
    /// Boolean reachability matrix for the graph.
    pub(crate) reachable: FixedBitSet,
    /// Pairs of nodes that have a path connecting them.
    pub(crate) connected: HashSet<(V, V)>,
    /// Pairs of nodes that don't have a path connecting them.
    pub(crate) disconnected: Vec<(V, V)>,
    /// Edges that are redundant because a longer path exists.
    pub(crate) transitive_edges: Vec<(V, V)>,
    /// Variant of the graph with no transitive edges.
    pub(crate) transitive_reduction: DiGraphMap<V, ()>,
    /// Variant of the graph with all possible transitive edges.
    // TODO: this will very likely be used by "if-needed" ordering
    #[allow(dead_code)]
    pub(crate) transitive_closure: DiGraphMap<V, ()>,
}

impl<V: NodeTrait + Debug> Default for CheckGraphResults<V> {
    fn default() -> Self {
        Self {
            reachable: FixedBitSet::new(),
            connected: HashSet::new(),
            disconnected: Vec::new(),
            transitive_edges: Vec::new(),
            transitive_reduction: DiGraphMap::new(),
            transitive_closure: DiGraphMap::new(),
        }
    }
}

/// Processes a DAG and computes its:
/// - transitive reduction (along with the set of removed edges)
/// - transitive closure
/// - reachability matrix (as a bitset)
/// - pairs of nodes connected by a path
/// - pairs of nodes not connected by a path
///
/// The algorithm implemented comes from
/// ["On the calculation of transitive reduction-closure of orders"][1] by Habib, Morvan and Rampon.
///
/// [1]: https://doi.org/10.1016/0012-365X(93)90164-O
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

    // build a copy of the graph where the nodes and edges appear in topsorted order
    let mut map = HashMap::with_capacity(n);
    let mut topsorted = DiGraphMap::<V, ()>::new();
    // iterate nodes in topological order
    for (i, &node) in topological_order.iter().enumerate() {
        map.insert(node, i);
        topsorted.add_node(node);
        // insert nodes as successors to their predecessors
        for pred in graph.neighbors_directed(node, Direction::Incoming) {
            topsorted.add_edge(pred, node, ());
        }
    }

    let mut reachable = FixedBitSet::with_capacity(n * n);
    let mut connected = HashSet::new();
    let mut disconnected = Vec::new();

    let mut transitive_edges = Vec::new();
    let mut transitive_reduction = DiGraphMap::<V, ()>::new();
    let mut transitive_closure = DiGraphMap::<V, ()>::new();

    let mut visited = FixedBitSet::with_capacity(n);

    // iterate nodes in topological order
    for node in topsorted.nodes() {
        transitive_reduction.add_node(node);
        transitive_closure.add_node(node);
    }

    // iterate nodes in reverse topological order
    for a in topsorted.nodes().rev() {
        let index_a = *map.get(&a).unwrap();
        // iterate their successors in topological order
        for b in topsorted.neighbors_directed(a, Direction::Outgoing) {
            let index_b = *map.get(&b).unwrap();
            debug_assert!(index_a < index_b);
            if !visited[index_b] {
                // edge <a, b> is not redundant
                transitive_reduction.add_edge(a, b, ());
                transitive_closure.add_edge(a, b, ());
                reachable.insert(index(index_a, index_b, n));

                let successors = transitive_closure
                    .neighbors_directed(b, Direction::Outgoing)
                    .collect::<Vec<_>>();
                for c in successors {
                    let index_c = *map.get(&c).unwrap();
                    debug_assert!(index_b < index_c);
                    if !visited[index_c] {
                        visited.insert(index_c);
                        transitive_closure.add_edge(a, c, ());
                        reachable.insert(index(index_a, index_c, n));
                    }
                }
            } else {
                // edge <a, b> is redundant
                transitive_edges.push((a, b));
            }
        }

        visited.clear();
    }

    // partition pairs of nodes into "connected by path" and "not connected by path"
    for i in 0..(n - 1) {
        // reachable is upper triangular because the nodes were topsorted
        for index in index(i, i + 1, n)..=index(i, n - 1, n) {
            let (a, b) = row_col(index, n);
            let pair = (topological_order[a], topological_order[b]);
            if reachable[index] {
                connected.insert(pair);
            } else {
                disconnected.push(pair);
            }
        }
    }

    // fill diagonal (nodes reach themselves)
    // for i in 0..n {
    //     reachable.set(index(i, i, n), true);
    // }

    CheckGraphResults {
        reachable,
        connected,
        disconnected,
        transitive_edges,
        transitive_reduction,
        transitive_closure,
    }
}

/// Returns the simple cycles in a strongly-connected component of a directed graph.
///
/// The algorithm implemented comes from
/// ["Finding all the elementary circuits of a directed graph"][1] by D. B. Johnson.
///
/// [1]: https://doi.org/10.1137/0204007
pub fn simple_cycles_in_component<N>(graph: &DiGraphMap<N, ()>, scc: &[N]) -> Vec<Vec<N>>
where
    N: NodeTrait + Debug,
{
    let mut cycles = vec![];
    let mut sccs = vec![scc.to_vec()];

    while let Some(mut scc) = sccs.pop() {
        // only look at nodes and edges in this strongly-connected component
        let mut subgraph = DiGraphMap::new();
        for &node in &scc {
            subgraph.add_node(node);
        }

        for &node in &scc {
            for successor in graph.neighbors(node) {
                if subgraph.contains_node(successor) {
                    subgraph.add_edge(node, successor, ());
                }
            }
        }

        // path of nodes that may form a cycle
        let mut path = Vec::with_capacity(subgraph.node_count());
        // we mark nodes as "blocked" to avoid finding permutations of the same cycles
        let mut blocked = HashSet::with_capacity(subgraph.node_count());
        // connects nodes along path segments that can't be part of a cycle (given current root)
        // those nodes can be unblocked at the same time
        let mut unblock_together: HashMap<N, HashSet<N>> =
            HashMap::with_capacity(subgraph.node_count());
        // stack for unblocking nodes
        let mut unblock_stack = Vec::with_capacity(subgraph.node_count());
        // nodes can be involved in multiple cycles
        let mut maybe_in_more_cycles: HashSet<N> = HashSet::with_capacity(subgraph.node_count());
        // stack for DFS
        let mut stack = Vec::with_capacity(subgraph.node_count());

        // we're going to look for all cycles that begin and end at this node
        let root = scc.pop().unwrap();
        // start a path at the root
        path.clear();
        path.push(root);
        // mark this node as blocked
        blocked.insert(root);

        // DFS
        stack.clear();
        stack.push((root, subgraph.neighbors(root)));
        while !stack.is_empty() {
            let (ref node, successors) = stack.last_mut().unwrap();
            if let Some(next) = successors.next() {
                if next == root {
                    // found a cycle
                    maybe_in_more_cycles.extend(path.iter());
                    cycles.push(path.clone());
                } else if !blocked.contains(&next) {
                    // first time seeing `next` on this path
                    maybe_in_more_cycles.remove(&next);
                    path.push(next);
                    blocked.insert(next);
                    stack.push((next, subgraph.neighbors(next)));
                    continue;
                } else {
                    // not first time seeing `next` on this path
                }
            }

            if successors.peekable().peek().is_none() {
                if maybe_in_more_cycles.contains(node) {
                    unblock_stack.push(*node);
                    // unblock this node's ancestors
                    while let Some(n) = unblock_stack.pop() {
                        if blocked.remove(&n) {
                            let unblock_predecessors =
                                unblock_together.entry(n).or_insert_with(HashSet::new);
                            unblock_stack.extend(unblock_predecessors.iter());
                            unblock_predecessors.clear();
                        }
                    }
                } else {
                    // if its descendants can be unblocked later, this node will be too
                    for successor in subgraph.neighbors(*node) {
                        unblock_together
                            .entry(successor)
                            .or_insert_with(HashSet::new)
                            .insert(*node);
                    }
                }

                // remove node from path and DFS stack
                path.pop();
                stack.pop();
            }
        }

        // remove node from subgraph
        subgraph.remove_node(root);

        // divide remainder into smaller SCCs
        let mut tarjan_scc = TarjanScc::new();
        tarjan_scc.run(&subgraph, |scc| {
            if scc.len() > 1 {
                sccs.push(scc.to_vec());
            }
        });
    }

    cycles
}
