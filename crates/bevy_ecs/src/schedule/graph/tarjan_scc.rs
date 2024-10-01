use super::DiGraph;
use super::NodeId;
use alloc::vec::Vec;
use core::hash::BuildHasher;
use core::num::NonZeroUsize;
use smallvec::SmallVec;

/// Create an iterator over *strongly connected components* using Algorithm 3 in
/// [A Space-Efficient Algorithm for Finding Strongly Connected Components][1] by David J. Pierce,
/// which is a memory-efficient variation of [Tarjan's algorithm][2].
///
///
/// [1]: https://homepages.ecs.vuw.ac.nz/~djp/files/P05.pdf
/// [2]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
///
/// Returns each strongly strongly connected component (scc).
/// The order of node ids within each scc is arbitrary, but the order of
/// the sccs is their postorder (reverse topological sort).
pub(crate) fn new_tarjan_scc<S: BuildHasher>(
    graph: &DiGraph<S>,
) -> impl Iterator<Item = SmallVec<[NodeId; 4]>> + '_ {
    // Create a list of all nodes we need to visit.
    let unchecked_nodes = graph.nodes();

    // For each node we need to visit, we also need to visit its neighbours.
    // Storing the iterator for each set of neighbours allows this list to be computed without
    // an additional allocation.
    let nodes = graph
        .nodes()
        .map(|node| NodeData {
            rootindex: None,
            neighbours: graph.neighbors(node),
        })
        .collect::<Vec<_>>();

    TarjanScc {
        graph,
        unchecked_nodes,
        index: 1,                   // Invariant: index < componentcount at all times.
        componentcount: usize::MAX, // Will hold if componentcount is initialized to number of nodes - 1 or higher.
        nodes,
        stack: Vec::new(),
        visitation_stack: Vec::new(),
        start: None,
        indexadjustment: None,
    }
}

struct NodeData<N: Iterator<Item = NodeId>> {
    rootindex: Option<NonZeroUsize>,
    neighbours: N,
}

/// A state for computing the *strongly connected components* using [Tarjan's algorithm][1].
///
/// This is based on [`TarjanScc`] from [`petgraph`].
///
/// [1]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
/// [`petgraph`]: https://docs.rs/petgraph/0.6.5/petgraph/
/// [`TarjanScc`]: https://docs.rs/petgraph/0.6.5/petgraph/algo/struct.TarjanScc.html
struct TarjanScc<'graph, Hasher, AllNodes, Neighbours>
where
    Hasher: BuildHasher,
    AllNodes: Iterator<Item = NodeId>,
    Neighbours: Iterator<Item = NodeId>,
{
    /// Source of truth [`DiGraph`]
    graph: &'graph DiGraph<Hasher>,
    /// An [`Iterator`] of [`NodeId`]s from the `graph` which may not have been visited yet.
    unchecked_nodes: AllNodes,
    /// The index of the next SCC
    index: usize,
    /// A count of potentially remaining SCCs
    componentcount: usize,
    /// Information about each [`NodeId`], including a possible SCC index and an
    /// [`Iterator`] of possibly unvisited neighbours.
    nodes: Vec<NodeData<Neighbours>>,
    /// A stack of [`NodeId`]s where a SCC will be found starting at the top of the stack.
    stack: Vec<NodeId>,
    /// A stack of [`NodeId`]s which need to be visited to determine which SCC they belong to.
    visitation_stack: Vec<(NodeId, bool)>,
    /// An index into the `stack` indicating the starting point of a SCC.
    start: Option<usize>,
    /// An adjustment to the `index` which will be applied once the current SCC is found.
    indexadjustment: Option<usize>,
}

impl<'graph, S: BuildHasher, A: Iterator<Item = NodeId>, N: Iterator<Item = NodeId>>
    TarjanScc<'graph, S, A, N>
{
    /// Compute the next *strongly connected component* using Algorithm 3 in
    /// [A Space-Efficient Algorithm for Finding Strongly Connected Components][1] by David J. Pierce,
    /// which is a memory-efficient variation of [Tarjan's algorithm][2].
    ///
    ///
    /// [1]: https://homepages.ecs.vuw.ac.nz/~djp/files/P05.pdf
    /// [2]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
    ///
    /// Returns `Some` for each strongly strongly connected component (scc).
    /// The order of node ids within each scc is arbitrary, but the order of
    /// the sccs is their postorder (reverse topological sort).
    fn next_scc(&mut self) -> Option<&[NodeId]> {
        // Cleanup from possible previous iteration
        if let (Some(start), Some(indexadjustment)) =
            (self.start.take(), self.indexadjustment.take())
        {
            self.stack.truncate(start);
            self.index -= indexadjustment; // Backtrack index back to where it was before we ever encountered the component.
            self.componentcount -= 1;
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

            let visited = self.nodes[self.graph.to_index(node)].rootindex.is_some();

            // If this node hasn't already been visited (e.g., it was the neighbour of a previously checked node)
            // add it to the visitation stack.
            if !visited {
                self.visitation_stack.push((node, true));
            }
        }
    }

    /// Attempt to find the starting point on the stack for a new SCC without visiting neighbours.
    /// If a visitation is required, this will return `None` and mark the required neighbour and the
    /// current node as in need of visitation again.
    /// If no SCC can be found in the current visitation stack, returns `None`.
    fn visit_once(&mut self, v: NodeId, mut v_is_local_root: bool) -> Option<usize> {
        let node_v = &mut self.nodes[self.graph.to_index(v)];

        if node_v.rootindex.is_none() {
            let v_index = self.index;
            node_v.rootindex = NonZeroUsize::new(v_index);
            self.index += 1;
        }

        while let Some(w) = self.nodes[self.graph.to_index(v)].neighbours.next() {
            // If a neighbour hasn't been visited yet...
            if self.nodes[self.graph.to_index(w)].rootindex.is_none() {
                // Push the current node and the neighbour back onto the visitation stack.
                // On the next execution of `visit_once`, the neighbour will be visited.
                self.visitation_stack.push((v, v_is_local_root));
                self.visitation_stack.push((w, true));

                return None;
            }

            if self.nodes[self.graph.to_index(w)].rootindex
                < self.nodes[self.graph.to_index(v)].rootindex
            {
                self.nodes[self.graph.to_index(v)].rootindex =
                    self.nodes[self.graph.to_index(w)].rootindex;
                v_is_local_root = false;
            }
        }

        if !v_is_local_root {
            self.stack.push(v); // Stack is filled up when backtracking, unlike in Tarjans original algorithm.
            return None;
        }

        // Pop the stack and generate an SCC.
        let mut indexadjustment = 1;
        let c = NonZeroUsize::new(self.componentcount);
        let nodes = &mut self.nodes;
        let start = self
            .stack
            .iter()
            .rposition(|&w| {
                if nodes[self.graph.to_index(v)].rootindex > nodes[self.graph.to_index(w)].rootindex
                {
                    true
                } else {
                    nodes[self.graph.to_index(w)].rootindex = c;
                    indexadjustment += 1;
                    false
                }
            })
            .map(|x| x + 1)
            .unwrap_or_default();
        nodes[self.graph.to_index(v)].rootindex = c;
        self.stack.push(v); // Pushing the component root to the back right before getting rid of it is somewhat ugly, but it lets it be included in f.

        self.start = Some(start);
        self.indexadjustment = Some(indexadjustment);

        Some(start)
    }
}

impl<'graph, S: BuildHasher, A: Iterator<Item = NodeId>, N: Iterator<Item = NodeId>> Iterator
    for TarjanScc<'graph, S, A, N>
{
    // It is expected that the `DiGraph` is sparse, and as such wont have many large SCCs.
    // Returning a `SmallVec` allows this iterator to skip allocation in cases where that
    // assumption holds.
    type Item = SmallVec<[NodeId; 4]>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = SmallVec::from_slice(self.next_scc()?);
        Some(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // There can be no more than the number of nodes in a graph worth of SCCs
        (0, Some(self.nodes.len()))
    }
}
