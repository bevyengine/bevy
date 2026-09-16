use alloc::vec::Vec;
use core::{hash::BuildHasher, num::NonZeroUsize};

use smallvec::SmallVec;

use crate::schedule::graph::{DiGraph, GraphNodeId};

/// Create an iterator over *strongly connected components* using Algorithm 3 in
/// [A Space-Efficient Algorithm for Finding Strongly Connected Components][1] by David J. Pierce,
/// which is a memory-efficient variation of [Tarjan's algorithm][2].
///
///
/// [1]: https://homepages.ecs.vuw.ac.nz/~djp/files/P05.pdf
/// [2]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
///
/// Returns each strongly connected component (scc).
/// The order of node ids within each scc is arbitrary, but the order of
/// the sccs is their postorder (reverse topological sort).
pub(crate) fn new_tarjan_scc<N: GraphNodeId, S: BuildHasher>(
    graph: &DiGraph<N, S>,
) -> impl Iterator<Item = SmallVec<[N; 4]>> + '_ {
    // Create a list of all nodes we need to visit.
    let unchecked_nodes = graph.nodes();

    // For each node we need to visit, we also need to visit its neighbors.
    // Storing the iterator for each set of neighbors allows this list to be computed without
    // an additional allocation.
    let nodes = graph
        .nodes()
        .map(|node| NodeData {
            root_index: None,
            pending: None,
            neighbors: graph.neighbors(node),
        })
        .collect::<Vec<_>>();

    TarjanScc {
        graph,
        unchecked_nodes,
        index: 1,                    // Invariant: index < component_count at all times.
        component_count: usize::MAX, // Will hold if component_count is initialized to number of nodes - 1 or higher.
        nodes,
        stack: Vec::new(),
        visitation_stack: Vec::new(),
        start: None,
        index_adjustment: None,
    }
}

struct NodeData<N: GraphNodeId, Neighbors: Iterator<Item = N>> {
    root_index: Option<NonZeroUsize>,
    pending: Option<N>,
    neighbors: Neighbors,
}

/// A state for computing the *strongly connected components* using [Tarjan's algorithm][1].
///
/// This is based on [`TarjanScc`] from [`petgraph`].
///
/// [1]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
/// [`petgraph`]: https://docs.rs/petgraph/0.6.5/petgraph/
/// [`TarjanScc`]: https://docs.rs/petgraph/0.6.5/petgraph/algo/struct.TarjanScc.html
struct TarjanScc<'graph, N, Hasher, AllNodes, Neighbors>
where
    N: GraphNodeId,
    Hasher: BuildHasher,
    AllNodes: Iterator<Item = N>,
    Neighbors: Iterator<Item = N>,
{
    /// Source of truth [`DiGraph`]
    graph: &'graph DiGraph<N, Hasher>,
    /// An [`Iterator`] of [`GraphNodeId`]s from the `graph` which may not have been visited yet.
    unchecked_nodes: AllNodes,
    /// The index of the next SCC
    index: usize,
    /// A count of potentially remaining SCCs
    component_count: usize,
    /// Information about each [`GraphNodeId`], including a possible SCC index and an
    /// [`Iterator`] of possibly unvisited neighbors.
    nodes: Vec<NodeData<N, Neighbors>>,
    /// A stack of [`GraphNodeId`]s where a SCC will be found starting at the top of the stack.
    stack: Vec<N>,
    /// A stack of [`GraphNodeId`]s which need to be visited to determine which SCC they belong to.
    visitation_stack: Vec<(N, bool)>,
    /// An index into the `stack` indicating the starting point of a SCC.
    start: Option<usize>,
    /// An adjustment to the `index` which will be applied once the current SCC is found.
    index_adjustment: Option<usize>,
}

impl<
        'graph,
        N: GraphNodeId,
        S: BuildHasher,
        A: Iterator<Item = N>,
        Neighbors: Iterator<Item = N>,
    > TarjanScc<'graph, N, S, A, Neighbors>
{
    /// Compute the next *strongly connected component* using Algorithm 3 in
    /// [A Space-Efficient Algorithm for Finding Strongly Connected Components][1] by David J. Pierce,
    /// which is a memory-efficient variation of [Tarjan's algorithm][2].
    ///
    ///
    /// [1]: https://homepages.ecs.vuw.ac.nz/~djp/files/P05.pdf
    /// [2]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
    ///
    /// Returns `Some` for each strongly connected component (scc).
    /// The order of node ids within each scc is arbitrary, but the order of
    /// the sccs is their postorder (reverse topological sort).
    fn next_scc(&mut self) -> Option<&[N]> {
        // Cleanup from possible previous iteration
        if let (Some(start), Some(index_adjustment)) =
            (self.start.take(), self.index_adjustment.take())
        {
            self.stack.truncate(start);
            self.index -= index_adjustment; // Backtrack index back to where it was before we ever encountered the component.
            self.component_count -= 1;
        }

        loop {
            // If there are items on the visitation stack, then we haven't finished visiting
            // the node at the bottom of the stack yet.
            // Must visit all nodes in the stack from top to bottom before visiting the next node.
            while let Some((v, v_is_local_root)) = self.visitation_stack.pop() {
                // If this visitation finds a complete SCC, return it immediately.
                if let Some(start) = self.visit_once(v, v_is_local_root) {
                    return Some(&self.stack[start..]);
                };
            }

            // Get the next node to check, otherwise we're done and can return None.
            let Some(node) = self.unchecked_nodes.next() else {
                break None;
            };

            let visited = self.nodes[self.graph.to_index(node)].root_index.is_some();

            // If this node hasn't already been visited (e.g., it was the neighbor of a previously checked node)
            // add it to the visitation stack.
            if !visited {
                self.visitation_stack.push((node, true));
            }
        }
    }

    /// Attempt to find the starting point on the stack for a new SCC without visiting neighbors.
    /// If a visitation is required, this will return `None` and mark the required neighbor and the
    /// current node as in need of visitation again.
    /// If no SCC can be found in the current visitation stack, returns `None`.
    fn visit_once(&mut self, v: N, mut v_is_local_root: bool) -> Option<usize> {
        let graph_index_v = self.graph.to_index(v);
        let node_v = &mut self.nodes[graph_index_v];

        if node_v.root_index.is_none() {
            let v_index = self.index;
            node_v.root_index = NonZeroUsize::new(v_index);
            self.index += 1;
        }

        if let Some(w) = node_v.pending.take() {
            let graph_index_w = self.graph.to_index(w);
            if self.nodes[graph_index_w].root_index < self.nodes[graph_index_v].root_index {
                self.nodes[graph_index_v].root_index = self.nodes[graph_index_w].root_index;
                v_is_local_root = false;
            }
        }

        while let Some(w) = self.nodes[graph_index_v].neighbors.next() {
            let graph_index_w = self.graph.to_index(w);
            // If a neighbor hasn't been visited yet...
            if self.nodes[graph_index_w].root_index.is_none() {
                // Push the current node and the neighbor back onto the visitation stack.
                // On the next execution of `visit_once`, the neighbor will be visited.
                self.visitation_stack.push((v, v_is_local_root));
                self.visitation_stack.push((w, true));
                // Due to the removal of w from the iterator, if there is no pending field,
                // `v` will not update based on the result of `w`. We need to handle it explicitly.
                self.nodes[graph_index_v].pending = Some(w);

                return None;
            }

            if self.nodes[graph_index_w].root_index < self.nodes[graph_index_v].root_index {
                self.nodes[graph_index_v].root_index = self.nodes[graph_index_w].root_index;
                v_is_local_root = false;
            }
        }

        if !v_is_local_root {
            self.stack.push(v); // Stack is filled up when backtracking, unlike in Tarjans original algorithm.
            return None;
        }

        // Pop the stack and generate an SCC.
        let mut index_adjustment = 1;
        let c = NonZeroUsize::new(self.component_count);
        let nodes = &mut self.nodes;
        let start = self
            .stack
            .iter()
            .rposition(|&w| {
                let graph_index_w = self.graph.to_index(w);
                if nodes[graph_index_v].root_index > nodes[graph_index_w].root_index {
                    true
                } else {
                    nodes[graph_index_w].root_index = c;
                    index_adjustment += 1;
                    false
                }
            })
            .map(|x| x + 1)
            .unwrap_or_default();
        nodes[graph_index_v].root_index = c;
        self.stack.push(v); // Pushing the component root to the back right before getting rid of it is somewhat ugly, but it lets it be included in f.

        self.start = Some(start);
        self.index_adjustment = Some(index_adjustment);

        Some(start)
    }
}

impl<
        'graph,
        N: GraphNodeId,
        S: BuildHasher,
        A: Iterator<Item = N>,
        Neighbors: Iterator<Item = N>,
    > Iterator for TarjanScc<'graph, N, S, A, Neighbors>
{
    // It is expected that the `DiGraph` is sparse, and as such wont have many large SCCs.
    // Returning a `SmallVec` allows this iterator to skip allocation in cases where that
    // assumption holds.
    type Item = SmallVec<[N; 4]>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = SmallVec::from_slice(self.next_scc()?);
        Some(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // There can be no more than the number of nodes in a graph worth of SCCs
        (0, Some(self.nodes.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::graph::Direction;

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Node(i32);

    impl GraphNodeId for Node {
        type Adjacent = (Node, Direction);
        type Edge = (Node, Node);
        fn kind(&self) -> &'static str {
            ""
        }
    }

    #[test]
    fn a_b_c_a() {
        let mut graph = DiGraph::<Node>::with_capacity(3, 3);
        graph.add_node(Node(1));
        graph.add_node(Node(2));
        graph.add_node(Node(3));
        graph.add_edge(Node(1), Node(2));
        graph.add_edge(Node(2), Node(3));
        graph.add_edge(Node(3), Node(1));

        let mut tarjan = new_tarjan_scc(&graph);
        let scc = tarjan.next().unwrap();
        let none = tarjan.next();
        assert_eq!(scc.len(), 3);
        assert!(scc.contains(&Node(1)));
        assert!(scc.contains(&Node(2)));
        assert!(scc.contains(&Node(3)));
        assert!(none.is_none());
    }
}
