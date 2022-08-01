use bevy_utils::{HashMap, HashSet};

use bitvec::prelude::*;
use petgraph::{graphmap::NodeTrait, prelude::*};

use std::fmt::Debug;

/// Converts 2D row-major pair of indices into a 1D array index.
pub(crate) fn index(row: usize, col: usize, num_cols: usize) -> usize {
    assert!(col < num_cols);
    (row * num_cols) + col
}

/// Converts a 1D array index into a 2D row-major pair of indices.
pub(crate) fn row_col(index: usize, num_cols: usize) -> (usize, usize) {
    (index / num_cols, index % num_cols)
}

pub(crate) struct CheckGraphResults<V> {
    // Pairs of nodes whose relative order is unknown.
    pub(crate) ambiguities: HashSet<(V, V)>,
    // Edges that are redundant because a longer path exists.
    pub(crate) transitive_edges: Vec<(V, V)>,
    // Boolean reachability matrices for the graph.
    pub(crate) reachable: BitVec<usize, Lsb0>,
    // Variant of the graph with the fewest possible edges.
    pub(crate) tred: DiGraphMap<V, ()>,
    // Variant of the graph with the most possible edges.
    pub(crate) tcls: DiGraphMap<V, ()>,
}

impl<V: NodeTrait + Debug> Default for CheckGraphResults<V> {
    fn default() -> Self {
        Self {
            ambiguities: HashSet::new(),
            transitive_edges: Vec::new(),
            reachable: BitVec::new(),
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
    let n = graph.node_count();

    if n == 0 {
        return CheckGraphResults::default();
    }

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
    let mut ambiguities = HashSet::new();
    let mut transitive_edges = Vec::new();

    let mut visited = BitVec::<usize, Lsb0>::with_capacity(n);
    unsafe {
        visited.set_len(n);
    }
    visited.fill(false);

    let mut reachable = BitVec::<usize, Lsb0>::with_capacity(n * n);
    unsafe {
        reachable.set_len(n * n);
    }
    reachable.fill(false);

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

        visited.fill(false);
    }

    for i in 0..(n - 1) {
        // reachable is upper triangular because the nodes are in topological order
        for index in reachable[index(i, i + 1, n)..index(i, n - 1, n)].iter_zeros() {
            let (a, b) = row_col(index, n);
            ambiguities.insert((topological_order[a], topological_order[b]));
        }
    }

    // Fill diagonal.
    for i in 0..n {
        reachable.set(index(i, i, n), true);
    }

    CheckGraphResults {
        ambiguities,
        transitive_edges,
        reachable,
        tred,
        tcls,
    }
}
