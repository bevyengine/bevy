use alloc::vec::Vec;
use core::{
    fmt::{self, Debug},
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut},
};

use bevy_platform::{
    collections::{HashMap, HashSet},
    hash::FixedHasher,
};
use fixedbitset::FixedBitSet;
use indexmap::IndexSet;
use thiserror::Error;

use crate::{
    error::Result,
    schedule::graph::{
        index, row_col, DiGraph, DiGraphToposortError,
        Direction::{Incoming, Outgoing},
        GraphNodeId, UnGraph,
    },
};

/// A directed acyclic graph structure.
#[derive(Clone)]
pub struct Dag<N: GraphNodeId, S: BuildHasher = FixedHasher> {
    /// The underlying directed graph.
    graph: DiGraph<N, S>,
    /// A cached topological ordering of the graph. This is recomputed when the
    /// graph is modified, and is not valid when `dirty` is true.
    toposort: Vec<N>,
    /// Whether the graph has been modified since the last topological sort.
    dirty: bool,
}

impl<N: GraphNodeId, S: BuildHasher> Dag<N, S> {
    /// Creates a new directed acyclic graph.
    pub fn new() -> Self
    where
        S: Default,
    {
        Self::default()
    }

    /// Read-only access to the underlying directed graph.
    #[must_use]
    pub fn graph(&self) -> &DiGraph<N, S> {
        &self.graph
    }

    /// Mutable access to the underlying directed graph. Marks the graph as dirty.
    #[must_use = "This function marks the graph as dirty, so it should be used."]
    pub fn graph_mut(&mut self) -> &mut DiGraph<N, S> {
        self.dirty = true;
        &mut self.graph
    }

    /// Returns whether the graph is dirty (i.e., has been modified since the
    /// last topological sort).
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns whether the graph is topologically sorted (i.e., not dirty).
    #[must_use]
    pub fn is_toposorted(&self) -> bool {
        !self.dirty
    }

    /// Ensures the graph is topologically sorted, recomputing the toposort if
    /// the graph is dirty.
    ///
    /// # Errors
    ///
    /// Returns [`DiGraphToposortError`] if the DAG is dirty and cannot be
    /// topologically sorted.
    pub fn ensure_toposorted(&mut self) -> Result<(), DiGraphToposortError<N>> {
        if self.dirty {
            // recompute the toposort, reusing the existing allocation
            self.toposort = self.graph.toposort(core::mem::take(&mut self.toposort))?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Returns the cached toposort if the graph is not dirty, otherwise returns
    /// `None`.
    #[must_use = "This method only returns a cached value and does not compute anything."]
    pub fn get_toposort(&self) -> Option<&[N]> {
        if self.dirty {
            None
        } else {
            Some(&self.toposort)
        }
    }

    /// Returns a topological ordering of the graph, computing it if the graph
    /// is dirty.
    ///
    /// # Errors
    ///
    /// Returns [`DiGraphToposortError`] if the DAG is dirty and cannot be
    /// topologically sorted.
    pub fn toposort(&mut self) -> Result<&[N], DiGraphToposortError<N>> {
        self.ensure_toposorted()?;
        Ok(&self.toposort)
    }

    /// Returns both the topological ordering and the underlying graph,
    /// computing the toposort if the graph is dirty.
    ///
    /// This function is useful to avoid multiple borrow issues when both
    /// the graph and the toposort are needed.
    ///
    /// # Errors
    ///
    /// Returns [`DiGraphToposortError`] if the DAG is dirty and cannot be
    /// topologically sorted.
    pub fn toposort_and_graph(
        &mut self,
    ) -> Result<(&[N], &DiGraph<N, S>), DiGraphToposortError<N>> {
        self.ensure_toposorted()?;
        Ok((&self.toposort, &self.graph))
    }

    /// Processes a DAG and computes various properties about it.
    ///
    /// See [`DagAnalysis::new`] for details on what is computed.
    ///
    /// # Note
    ///
    /// If the DAG is dirty, this method will first attempt to topologically sort it.
    ///
    /// # Errors
    ///
    /// Returns [`DiGraphToposortError`] if the DAG is dirty and cannot be
    /// topologically sorted.
    ///
    pub fn analyze(&mut self) -> Result<DagAnalysis<N, S>, DiGraphToposortError<N>>
    where
        S: Default,
    {
        let (toposort, graph) = self.toposort_and_graph()?;
        Ok(DagAnalysis::new(graph, toposort))
    }

    /// Replaces the current graph with its transitive reduction based on the
    /// provided analysis.
    ///
    /// # Note
    ///
    /// The given [`DagAnalysis`] must have been generated from this DAG.
    pub fn remove_redundant_edges(&mut self, analysis: &DagAnalysis<N, S>)
    where
        S: Clone,
    {
        // We don't need to mark the graph as dirty, since transitive reduction
        // is guaranteed to have the same topological ordering as the original graph.
        self.graph = analysis.transitive_reduction.clone();
    }

    /// Groups nodes in this DAG by a key type `K`, collecting value nodes `V`
    /// under all of their ancestor key nodes. `num_groups` hints at the
    /// expected number of groups, for memory allocation optimization.
    ///
    /// The node type `N` must be convertible into either a key type `K` or
    /// a value type `V` via the [`TryInto`] trait.
    ///
    /// # Errors
    ///
    /// Returns [`DiGraphToposortError`] if the DAG is dirty and cannot be
    /// topologically sorted.
    pub fn group_by_key<K, V>(
        &mut self,
        num_groups: usize,
    ) -> Result<DagGroups<K, V, S>, DiGraphToposortError<N>>
    where
        N: TryInto<K, Error = V>,
        K: Eq + Hash,
        V: Clone + Eq + Hash,
        S: BuildHasher + Default,
    {
        let (toposort, graph) = self.toposort_and_graph()?;
        Ok(DagGroups::with_capacity(num_groups, graph, toposort))
    }

    /// Converts from one [`GraphNodeId`] type to another. If the conversion fails,
    /// it returns the error from the target type's [`TryFrom`] implementation.
    ///
    /// Nodes must uniquely convert from `N` to `T` (i.e. no two `N` can convert
    /// to the same `T`). The resulting DAG must be re-topologically sorted.
    ///
    /// # Errors
    ///
    /// If the conversion fails, it returns an error of type `N::Error`.
    pub fn try_convert<T>(self) -> Result<Dag<T, S>, N::Error>
    where
        N: TryInto<T>,
        T: GraphNodeId,
        S: Default,
    {
        Ok(Dag {
            graph: self.graph.try_convert()?,
            toposort: Vec::new(),
            dirty: true,
        })
    }
}

impl<N: GraphNodeId, S: BuildHasher> Deref for Dag<N, S> {
    type Target = DiGraph<N, S>;

    fn deref(&self) -> &Self::Target {
        self.graph()
    }
}

impl<N: GraphNodeId, S: BuildHasher> DerefMut for Dag<N, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.graph_mut()
    }
}

impl<N: GraphNodeId, S: BuildHasher + Default> Default for Dag<N, S> {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            toposort: Default::default(),
            dirty: false,
        }
    }
}

impl<N: GraphNodeId, S: BuildHasher> Debug for Dag<N, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dirty {
            f.debug_struct("Dag")
                .field("graph", &self.graph)
                .field("dirty", &self.dirty)
                .finish()
        } else {
            f.debug_struct("Dag")
                .field("graph", &self.graph)
                .field("toposort", &self.toposort)
                .finish()
        }
    }
}

/// Stores the results of a call to [`Dag::analyze`].
pub struct DagAnalysis<N: GraphNodeId, S: BuildHasher = FixedHasher> {
    /// Boolean reachability matrix for the graph.
    reachable: FixedBitSet,
    /// Pairs of nodes that have a path connecting them.
    connected: HashSet<(N, N), S>,
    /// Pairs of nodes that don't have a path connecting them.
    disconnected: Vec<(N, N)>,
    /// Edges that are redundant because a longer path exists.
    transitive_edges: Vec<(N, N)>,
    /// Variant of the graph with no transitive edges.
    transitive_reduction: DiGraph<N, S>,
    /// Variant of the graph with all possible transitive edges.
    transitive_closure: DiGraph<N, S>,
}

impl<N: GraphNodeId, S: BuildHasher> DagAnalysis<N, S> {
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
    pub fn new(graph: &DiGraph<N, S>, topological_order: &[N]) -> Self
    where
        S: Default,
    {
        if graph.node_count() == 0 {
            return DagAnalysis::default();
        }
        let n = graph.node_count();

        // build a copy of the graph where the nodes and edges appear in topsorted order
        let mut map = <HashMap<_, _>>::with_capacity_and_hasher(n, Default::default());
        let mut topsorted =
            DiGraph::<N>::with_capacity(topological_order.len(), graph.edge_count());

        // iterate nodes in topological order
        for (i, &node) in topological_order.iter().enumerate() {
            map.insert(node, i);
            topsorted.add_node(node);
            // insert nodes as successors to their predecessors
            for pred in graph.neighbors_directed(node, Incoming) {
                topsorted.add_edge(pred, node);
            }
        }

        let mut reachable = FixedBitSet::with_capacity(n * n);
        let mut connected = HashSet::default();
        let mut disconnected = Vec::default();
        let mut transitive_edges = Vec::default();
        let mut transitive_reduction = DiGraph::with_capacity(topsorted.node_count(), 0);
        let mut transitive_closure = DiGraph::with_capacity(topsorted.node_count(), 0);

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
            for b in topsorted.neighbors_directed(a, Outgoing) {
                let index_b = *map.get(&b).unwrap();
                debug_assert!(index_a < index_b);
                if !visited[index_b] {
                    // edge <a, b> is not redundant
                    transitive_reduction.add_edge(a, b);
                    transitive_closure.add_edge(a, b);
                    reachable.insert(index(index_a, index_b, n));

                    let successors = transitive_closure
                        .neighbors_directed(b, Outgoing)
                        .collect::<Vec<_>>();
                    for c in successors {
                        let index_c = *map.get(&c).unwrap();
                        debug_assert!(index_b < index_c);
                        if !visited[index_c] {
                            visited.insert(index_c);
                            transitive_closure.add_edge(a, c);
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

        DagAnalysis {
            reachable,
            connected,
            disconnected,
            transitive_edges,
            transitive_reduction,
            transitive_closure,
        }
    }

    /// Returns the reachability matrix.
    pub fn reachable(&self) -> &FixedBitSet {
        &self.reachable
    }

    /// Returns the set of node pairs that are connected by a path.
    pub fn connected(&self) -> &HashSet<(N, N), S> {
        &self.connected
    }

    /// Returns the list of node pairs that are not connected by a path.
    pub fn disconnected(&self) -> &[(N, N)] {
        &self.disconnected
    }

    /// Returns the list of redundant edges because a longer path exists.
    pub fn transitive_edges(&self) -> &[(N, N)] {
        &self.transitive_edges
    }

    /// Returns the transitive reduction of the graph.
    pub fn transitive_reduction(&self) -> &DiGraph<N, S> {
        &self.transitive_reduction
    }

    /// Returns the transitive closure of the graph.
    pub fn transitive_closure(&self) -> &DiGraph<N, S> {
        &self.transitive_closure
    }

    /// Checks if the graph has any redundant (transitive) edges.
    ///
    /// # Errors
    ///
    /// If there are redundant edges, returns a [`DagRedundancyError`]
    /// containing the list of redundant edges.
    pub fn check_for_redundant_edges(&self) -> Result<(), DagRedundancyError<N>>
    where
        S: Clone,
    {
        if self.transitive_edges.is_empty() {
            Ok(())
        } else {
            Err(DagRedundancyError(self.transitive_edges.clone()))
        }
    }

    /// Checks if there are any pairs of nodes that have a path in both this
    /// graph and another graph.
    ///
    /// # Errors
    ///
    /// Returns [`DagCrossDependencyError`] if any node pair is connected in
    /// both graphs.
    pub fn check_for_cross_dependencies(
        &self,
        other: &Self,
    ) -> Result<(), DagCrossDependencyError<N>> {
        for &(a, b) in &self.connected {
            if other.connected.contains(&(a, b)) || other.connected.contains(&(b, a)) {
                return Err(DagCrossDependencyError(a, b));
            }
        }

        Ok(())
    }

    /// Checks if any connected node pairs that are both keys have overlapping
    /// groups.
    ///
    /// # Errors
    ///
    /// If there are overlapping groups, returns a [`DagOverlappingGroupError`]
    /// containing the first pair of keys that have overlapping groups.
    pub fn check_for_overlapping_groups<K, V>(
        &self,
        groups: &DagGroups<K, V>,
    ) -> Result<(), DagOverlappingGroupError<K>>
    where
        N: TryInto<K>,
        K: Eq + Hash,
        V: Eq + Hash,
    {
        for &(a, b) in &self.connected {
            let (Ok(a_key), Ok(b_key)) = (a.try_into(), b.try_into()) else {
                continue;
            };
            let a_group = groups.get(&a_key).unwrap();
            let b_group = groups.get(&b_key).unwrap();
            if !a_group.is_disjoint(b_group) {
                return Err(DagOverlappingGroupError(a_key, b_key));
            }
        }
        Ok(())
    }
}

impl<N: GraphNodeId, S: BuildHasher + Default> Default for DagAnalysis<N, S> {
    fn default() -> Self {
        Self {
            reachable: Default::default(),
            connected: Default::default(),
            disconnected: Default::default(),
            transitive_edges: Default::default(),
            transitive_reduction: Default::default(),
            transitive_closure: Default::default(),
        }
    }
}

impl<N: GraphNodeId, S: BuildHasher> Debug for DagAnalysis<N, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DagAnalysis")
            .field("reachable", &self.reachable)
            .field("connected", &self.connected)
            .field("disconnected", &self.disconnected)
            .field("transitive_edges", &self.transitive_edges)
            .field("transitive_reduction", &self.transitive_reduction)
            .field("transitive_closure", &self.transitive_closure)
            .finish()
    }
}

/// A mapping of keys to groups of values in a [`Dag`].
pub struct DagGroups<K, V, S = FixedHasher>(HashMap<K, IndexSet<V, S>, S>);

impl<K: Eq + Hash, V: Clone + Eq + Hash, S: BuildHasher + Default> DagGroups<K, V, S> {
    /// Groups nodes in this DAG by a key type `K`, collecting value nodes `V`
    /// under all of their ancestor key nodes.
    ///
    /// The node type `N` must be convertible into either a key type `K` or
    /// a value type `V` via the [`TryInto`] trait.
    pub fn new<N>(graph: &DiGraph<N, S>, toposort: &[N]) -> Self
    where
        N: GraphNodeId + TryInto<K, Error = V>,
    {
        Self::with_capacity(0, graph, toposort)
    }

    /// Groups nodes in this DAG by a key type `K`, collecting value nodes `V`
    /// under all of their ancestor key nodes. `capacity` hints at the
    /// expected number of groups, for memory allocation optimization.
    ///
    /// The node type `N` must be convertible into either a key type `K` or
    /// a value type `V` via the [`TryInto`] trait.
    pub fn with_capacity<N>(capacity: usize, graph: &DiGraph<N, S>, toposort: &[N]) -> Self
    where
        N: GraphNodeId + TryInto<K, Error = V>,
    {
        let mut groups: HashMap<K, IndexSet<V, S>, S> =
            HashMap::with_capacity_and_hasher(capacity, Default::default());

        // Iterate in reverse topological order (bottom-up) so we hit children before parents.
        for &id in toposort.iter().rev() {
            let Ok(key) = id.try_into() else {
                continue;
            };

            let mut children = IndexSet::default();

            for node in graph.neighbors_directed(id, Outgoing) {
                match node.try_into() {
                    Ok(key) => {
                        // If the child is a key, this key inherits all of its children.
                        let key_children = groups.get(&key).unwrap();
                        children.extend(key_children.iter().cloned());
                    }
                    Err(value) => {
                        // If the child is a value, add it directly.
                        children.insert(value);
                    }
                }
            }

            groups.insert(key, children);
        }

        Self(groups)
    }
}

impl<K: GraphNodeId, V: GraphNodeId, S: BuildHasher> DagGroups<K, V, S> {
    /// Converts the given [`Dag`] into a flattened version where key nodes
    /// (`K`) are replaced by their associated value nodes (`V`). Edges to/from
    /// key nodes are redirected to connect their value nodes instead.
    ///
    /// The `collapse_group` function is called for each key node to customize
    /// how its group is collapsed.
    ///
    /// The resulting [`Dag`] will have only value nodes (`V`).
    pub fn flatten<N>(
        &self,
        dag: Dag<N>,
        mut collapse_group: impl FnMut(K, &IndexSet<V, S>, &Dag<N>, &mut Vec<(N, N)>),
    ) -> Dag<V>
    where
        N: GraphNodeId + TryInto<V, Error = K> + From<K> + From<V>,
    {
        let mut flattening = dag;
        let mut temp = Vec::new();

        for (&key, values) in self.iter() {
            // Call the user-provided function to handle collapsing the group.
            collapse_group(key, values, &flattening, &mut temp);

            if values.is_empty() {
                // Replace connections to the key node with connections between its neighbors.
                for a in flattening.neighbors_directed(N::from(key), Incoming) {
                    for b in flattening.neighbors_directed(N::from(key), Outgoing) {
                        temp.push((a, b));
                    }
                }
            } else {
                // Redirect edges to/from the key node to connect to its value nodes.
                for a in flattening.neighbors_directed(N::from(key), Incoming) {
                    for &value in values {
                        temp.push((a, N::from(value)));
                    }
                }
                for b in flattening.neighbors_directed(N::from(key), Outgoing) {
                    for &value in values {
                        temp.push((N::from(value), b));
                    }
                }
            }

            // Remove the key node from the graph.
            flattening.remove_node(N::from(key));
            // Add all previously collected edges.
            flattening.reserve_edges(temp.len());
            for (a, b) in temp.drain(..) {
                flattening.add_edge(a, b);
            }
        }

        // By this point, we should have removed all keys from the graph,
        // so this conversion should never fail.
        flattening
            .try_convert::<V>()
            .unwrap_or_else(|n| unreachable!("Flattened graph has a leftover key {n:?}"))
    }

    /// Converts an undirected graph by replacing key nodes (`K`) with their
    /// associated value nodes (`V`). Edges connected to key nodes are
    /// redirected to connect their value nodes instead.
    ///
    /// The resulting undirected graph will have only value nodes (`V`).
    pub fn flatten_undirected<N>(&self, graph: &UnGraph<N>) -> UnGraph<V>
    where
        N: GraphNodeId + TryInto<V, Error = K>,
    {
        let mut flattened = UnGraph::default();

        for (lhs, rhs) in graph.all_edges() {
            match (lhs.try_into(), rhs.try_into()) {
                (Ok(lhs), Ok(rhs)) => {
                    // Normal edge between two value nodes
                    flattened.add_edge(lhs, rhs);
                }
                (Err(lhs_key), Ok(rhs)) => {
                    // Edge from a key node to a value node, expand to all values in the key's group
                    let Some(lhs_group) = self.get(&lhs_key) else {
                        continue;
                    };
                    flattened.reserve_edges(lhs_group.len());
                    for &lhs in lhs_group {
                        flattened.add_edge(lhs, rhs);
                    }
                }
                (Ok(lhs), Err(rhs_key)) => {
                    // Edge from a value node to a key node, expand to all values in the key's group
                    let Some(rhs_group) = self.get(&rhs_key) else {
                        continue;
                    };
                    flattened.reserve_edges(rhs_group.len());
                    for &rhs in rhs_group {
                        flattened.add_edge(lhs, rhs);
                    }
                }
                (Err(lhs_key), Err(rhs_key)) => {
                    // Edge between two key nodes, expand to all combinations of their value nodes
                    let Some(lhs_group) = self.get(&lhs_key) else {
                        continue;
                    };
                    let Some(rhs_group) = self.get(&rhs_key) else {
                        continue;
                    };
                    flattened.reserve_edges(lhs_group.len() * rhs_group.len());
                    for &lhs in lhs_group {
                        for &rhs in rhs_group {
                            flattened.add_edge(lhs, rhs);
                        }
                    }
                }
            }
        }

        flattened
    }
}

impl<K, V, S> Deref for DagGroups<K, V, S> {
    type Target = HashMap<K, IndexSet<V, S>, S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, S> DerefMut for DagGroups<K, V, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V, S> Default for DagGroups<K, V, S>
where
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: Debug, V: Debug, S> Debug for DagGroups<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DagGroups").field(&self.0).finish()
    }
}

/// Error indicating that the graph has redundant edges.
#[derive(Error, Debug)]
#[error("DAG has redundant edges: {0:?}")]
pub struct DagRedundancyError<N: GraphNodeId>(pub Vec<(N, N)>);

/// Error indicating that two graphs both have a dependency between the same nodes.
#[derive(Error, Debug)]
#[error("DAG has a cross-dependency between nodes {0:?} and {1:?}")]
pub struct DagCrossDependencyError<N>(pub N, pub N);

/// Error indicating that the graph has overlapping groups between two keys.
#[derive(Error, Debug)]
#[error("DAG has overlapping groups between keys {0:?} and {1:?}")]
pub struct DagOverlappingGroupError<K>(pub K, pub K);

#[cfg(test)]
mod tests {
    use core::ops::DerefMut;

    use crate::schedule::graph::{index, Dag, Direction, GraphNodeId, UnGraph};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TestNode(u32);

    impl GraphNodeId for TestNode {
        type Adjacent = (TestNode, Direction);
        type Edge = (TestNode, TestNode);

        fn kind(&self) -> &'static str {
            "test node"
        }
    }

    #[test]
    fn mark_dirty() {
        {
            let mut dag = Dag::<TestNode>::new();
            dag.add_node(TestNode(1));
            assert!(dag.is_dirty());
        }
        {
            let mut dag = Dag::<TestNode>::new();
            dag.add_edge(TestNode(1), TestNode(2));
            assert!(dag.is_dirty());
        }
        {
            let mut dag = Dag::<TestNode>::new();
            dag.deref_mut();
            assert!(dag.is_dirty());
        }
        {
            let mut dag = Dag::<TestNode>::new();
            let _ = dag.graph_mut();
            assert!(dag.is_dirty());
        }
    }

    #[test]
    fn toposort() {
        let mut dag = Dag::<TestNode>::new();
        dag.add_edge(TestNode(1), TestNode(2));
        dag.add_edge(TestNode(2), TestNode(3));
        dag.add_edge(TestNode(1), TestNode(3));

        assert_eq!(
            dag.toposort().unwrap(),
            &[TestNode(1), TestNode(2), TestNode(3)]
        );
        assert_eq!(
            dag.get_toposort().unwrap(),
            &[TestNode(1), TestNode(2), TestNode(3)]
        );
    }

    #[test]
    fn analyze() {
        let mut dag1 = Dag::<TestNode>::new();
        dag1.add_edge(TestNode(1), TestNode(2));
        dag1.add_edge(TestNode(2), TestNode(3));
        dag1.add_edge(TestNode(1), TestNode(3)); // redundant edge

        let analysis1 = dag1.analyze().unwrap();

        assert!(analysis1.reachable().contains(index(0, 1, 3)));
        assert!(analysis1.reachable().contains(index(1, 2, 3)));
        assert!(analysis1.reachable().contains(index(0, 2, 3)));

        assert!(analysis1.connected().contains(&(TestNode(1), TestNode(2))));
        assert!(analysis1.connected().contains(&(TestNode(2), TestNode(3))));
        assert!(analysis1.connected().contains(&(TestNode(1), TestNode(3))));

        assert!(!analysis1
            .disconnected()
            .contains(&(TestNode(2), TestNode(1))));
        assert!(!analysis1
            .disconnected()
            .contains(&(TestNode(3), TestNode(2))));
        assert!(!analysis1
            .disconnected()
            .contains(&(TestNode(3), TestNode(1))));

        assert!(analysis1
            .transitive_edges()
            .contains(&(TestNode(1), TestNode(3))));

        assert!(analysis1.check_for_redundant_edges().is_err());

        let mut dag2 = Dag::<TestNode>::new();
        dag2.add_edge(TestNode(3), TestNode(4));

        let analysis2 = dag2.analyze().unwrap();

        assert!(analysis2.check_for_redundant_edges().is_ok());
        assert!(analysis1.check_for_cross_dependencies(&analysis2).is_ok());

        let mut dag3 = Dag::<TestNode>::new();
        dag3.add_edge(TestNode(1), TestNode(2));

        let analysis3 = dag3.analyze().unwrap();

        assert!(analysis1.check_for_cross_dependencies(&analysis3).is_err());

        dag1.remove_redundant_edges(&analysis1);
        let analysis1 = dag1.analyze().unwrap();
        assert!(analysis1.check_for_redundant_edges().is_ok());
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    enum Node {
        Key(Key),
        Value(Value),
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct Key(u32);
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct Value(u32);

    impl GraphNodeId for Node {
        type Adjacent = (Node, Direction);
        type Edge = (Node, Node);

        fn kind(&self) -> &'static str {
            "node"
        }
    }

    impl TryInto<Key> for Node {
        type Error = Value;

        fn try_into(self) -> Result<Key, Value> {
            match self {
                Node::Key(k) => Ok(k),
                Node::Value(v) => Err(v),
            }
        }
    }

    impl TryInto<Value> for Node {
        type Error = Key;

        fn try_into(self) -> Result<Value, Key> {
            match self {
                Node::Value(v) => Ok(v),
                Node::Key(k) => Err(k),
            }
        }
    }

    impl GraphNodeId for Key {
        type Adjacent = (Key, Direction);
        type Edge = (Key, Key);

        fn kind(&self) -> &'static str {
            "key"
        }
    }

    impl GraphNodeId for Value {
        type Adjacent = (Value, Direction);
        type Edge = (Value, Value);

        fn kind(&self) -> &'static str {
            "value"
        }
    }

    impl From<Key> for Node {
        fn from(key: Key) -> Self {
            Node::Key(key)
        }
    }

    impl From<Value> for Node {
        fn from(value: Value) -> Self {
            Node::Value(value)
        }
    }

    #[test]
    fn group_by_key() {
        let mut dag = Dag::<Node>::new();
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(10)));
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(11)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(20)));
        dag.add_edge(Node::Key(Key(2)), Node::Key(Key(1)));
        dag.add_edge(Node::Value(Value(10)), Node::Value(Value(11)));

        let groups = dag.group_by_key::<Key, Value>(2).unwrap();
        assert_eq!(groups.len(), 2);

        let group_key1 = groups.get(&Key(1)).unwrap();
        assert!(group_key1.contains(&Value(10)));
        assert!(group_key1.contains(&Value(11)));

        let group_key2 = groups.get(&Key(2)).unwrap();
        assert!(group_key2.contains(&Value(10)));
        assert!(group_key2.contains(&Value(11)));
        assert!(group_key2.contains(&Value(20)));
    }

    #[test]
    fn flatten() {
        let mut dag = Dag::<Node>::new();
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(10)));
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(11)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(20)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(21)));
        dag.add_edge(Node::Value(Value(30)), Node::Key(Key(1)));
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(40)));

        let groups = dag.group_by_key::<Key, Value>(2).unwrap();
        let flattened = groups.flatten(dag, |_key, _values, _dag, _temp| {});

        assert!(flattened.contains_node(Value(10)));
        assert!(flattened.contains_node(Value(11)));
        assert!(flattened.contains_node(Value(20)));
        assert!(flattened.contains_node(Value(21)));
        assert!(flattened.contains_node(Value(30)));
        assert!(flattened.contains_node(Value(40)));

        assert!(flattened.contains_edge(Value(30), Value(10)));
        assert!(flattened.contains_edge(Value(30), Value(11)));
        assert!(flattened.contains_edge(Value(10), Value(40)));
        assert!(flattened.contains_edge(Value(11), Value(40)));
    }

    #[test]
    fn flatten_undirected() {
        let mut dag = Dag::<Node>::new();
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(10)));
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(11)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(20)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(21)));

        let groups = dag.group_by_key::<Key, Value>(2).unwrap();

        let mut ungraph = UnGraph::<Node>::default();
        ungraph.add_edge(Node::Value(Value(10)), Node::Value(Value(11)));
        ungraph.add_edge(Node::Key(Key(1)), Node::Value(Value(30)));
        ungraph.add_edge(Node::Value(Value(40)), Node::Key(Key(2)));
        ungraph.add_edge(Node::Key(Key(1)), Node::Key(Key(2)));

        let flattened = groups.flatten_undirected(&ungraph);

        assert!(flattened.contains_edge(Value(10), Value(11)));
        assert!(flattened.contains_edge(Value(10), Value(30)));
        assert!(flattened.contains_edge(Value(11), Value(30)));
        assert!(flattened.contains_edge(Value(40), Value(20)));
        assert!(flattened.contains_edge(Value(40), Value(21)));
        assert!(flattened.contains_edge(Value(10), Value(20)));
        assert!(flattened.contains_edge(Value(10), Value(21)));
        assert!(flattened.contains_edge(Value(11), Value(20)));
        assert!(flattened.contains_edge(Value(11), Value(21)));
    }

    #[test]
    fn overlapping_groups() {
        let mut dag = Dag::<Node>::new();
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(10)));
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(11)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(11))); // overlap
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(20)));
        dag.add_edge(Node::Key(Key(1)), Node::Key(Key(2)));

        let groups = dag.group_by_key::<Key, Value>(2).unwrap();
        let analysis = dag.analyze().unwrap();

        let result = analysis.check_for_overlapping_groups(&groups);
        assert!(result.is_err());
    }

    #[test]
    fn disjoint_groups() {
        let mut dag = Dag::<Node>::new();
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(10)));
        dag.add_edge(Node::Key(Key(1)), Node::Value(Value(11)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(20)));
        dag.add_edge(Node::Key(Key(2)), Node::Value(Value(21)));

        let groups = dag.group_by_key::<Key, Value>(2).unwrap();
        let analysis = dag.analyze().unwrap();

        let result = analysis.check_for_overlapping_groups(&groups);
        assert!(result.is_ok());
    }
}
