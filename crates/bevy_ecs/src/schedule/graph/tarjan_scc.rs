use super::DiGraph;
use super::NodeId;
use alloc::vec::Vec;
use core::hash::BuildHasher;
use core::num::NonZeroUsize;

#[derive(Copy, Clone, Debug)]
struct NodeData {
    rootindex: Option<NonZeroUsize>,
}

/// A reusable state for computing the *strongly connected components* using [Tarjan's algorithm][1].
///
/// This is based on [`TarjanScc`] from [`petgraph`].
///
/// [1]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
/// [`petgraph`]: https://docs.rs/petgraph/0.6.5/petgraph/
/// [`TarjanScc`]: https://docs.rs/petgraph/0.6.5/petgraph/algo/struct.TarjanScc.html
#[derive(Debug)]
pub(crate) struct TarjanScc {
    index: usize,
    componentcount: usize,
    nodes: Vec<NodeData>,
    stack: Vec<NodeId>,
}

impl Default for TarjanScc {
    fn default() -> Self {
        Self::new()
    }
}

impl TarjanScc {
    /// Creates a new `TarjanScc`
    pub(crate) fn new() -> Self {
        TarjanScc {
            index: 1,                   // Invariant: index < componentcount at all times.
            componentcount: usize::MAX, // Will hold if componentcount is initialized to number of nodes - 1 or higher.
            nodes: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// \[Generic\] Compute the *strongly connected components* using Algorithm 3 in
    /// [A Space-Efficient Algorithm for Finding Strongly Connected Components][1] by David J. Pierce,
    /// which is a memory-efficient variation of [Tarjan's algorithm][2].
    ///
    ///
    /// [1]: https://homepages.ecs.vuw.ac.nz/~djp/files/P05.pdf
    /// [2]: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
    ///
    /// Calls `f` for each strongly strongly connected component (scc).
    /// The order of node ids within each scc is arbitrary, but the order of
    /// the sccs is their postorder (reverse topological sort).
    ///
    /// For an undirected graph, the sccs are simply the connected components.
    ///
    /// This implementation is recursive and does one pass over the nodes.
    pub(crate) fn run<S: BuildHasher>(&mut self, g: &DiGraph<S>, mut f: impl FnMut(&[NodeId])) {
        self.nodes.clear();
        self.nodes
            .resize(g.node_count(), NodeData { rootindex: None });

        for n in g.nodes() {
            let visited = self.nodes[g.to_index(n)].rootindex.is_some();
            if !visited {
                self.visit(n, g, &mut f);
            }
        }

        debug_assert!(self.stack.is_empty());
    }

    fn visit<S: BuildHasher>(&mut self, v: NodeId, g: &DiGraph<S>, f: &mut impl FnMut(&[NodeId])) {
        macro_rules! node {
            ($node:expr) => {
                self.nodes[g.to_index($node)]
            };
        }

        let node_v = &mut node![v];
        debug_assert!(node_v.rootindex.is_none());

        let mut v_is_local_root = true;
        let v_index = self.index;
        node_v.rootindex = NonZeroUsize::new(v_index);
        self.index += 1;

        for w in g.neighbors(v) {
            if node![w].rootindex.is_none() {
                self.visit(w, g, f);
            }
            if node![w].rootindex < node![v].rootindex {
                node![v].rootindex = node![w].rootindex;
                v_is_local_root = false;
            }
        }

        if v_is_local_root {
            // Pop the stack and generate an SCC.
            let mut indexadjustment = 1;
            let c = NonZeroUsize::new(self.componentcount);
            let nodes = &mut self.nodes;
            let start = self
                .stack
                .iter()
                .rposition(|&w| {
                    if nodes[g.to_index(v)].rootindex > nodes[g.to_index(w)].rootindex {
                        true
                    } else {
                        nodes[g.to_index(w)].rootindex = c;
                        indexadjustment += 1;
                        false
                    }
                })
                .map(|x| x + 1)
                .unwrap_or_default();
            nodes[g.to_index(v)].rootindex = c;
            self.stack.push(v); // Pushing the component root to the back right before getting rid of it is somewhat ugly, but it lets it be included in f.
            f(&self.stack[start..]);
            self.stack.truncate(start);
            self.index -= indexadjustment; // Backtrack index back to where it was before we ever encountered the component.
            self.componentcount -= 1;
        } else {
            self.stack.push(v); // Stack is filled up when backtracking, unlike in Tarjans original algorithm.
        }
    }
}
