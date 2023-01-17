/// All graph types implementing a `MultiGraph`
pub mod multi;
/// All graph types implementing a `SimpleGraph`
pub mod simple;

/// An edge between nodes that store data of type `E`.
pub mod edge;
/// The `NodeIdx` and `EdgeIdx` structs
pub mod keys;

use crate::error::GraphError;

use self::keys::{EdgeIdx, NodeIdx};

// NOTE: There should always be a common function and if needed a more precise function which the common function wraps.
//       Example: `edges_between` is `trait Graph` general and has support for Simple- and Multigraphs.
//                `edge_between` is only available for `SimpleGraph` but is also called from `edges_between`.

/// A trait with all the common functions for a graph
pub trait Graph<N, E> {
    /// Creates a new graph
    fn new() -> Self
    where
        Self: Sized;

    /// Returns `true` if the edges in the graph are directed.
    fn is_directed(&self) -> bool;

    /// Returns `true` if the graph allows for more than one edge between a pair of nodes.
    fn is_multigraph(&self) -> bool;

    /// Returns the number of nodes in the graph.
    fn node_count(&self) -> usize;

    /// Returns the number of edges in the graph.
    fn edge_count(&self) -> usize;

    /// Returns `true` if the graph has no nodes.
    fn is_empty(&self) -> bool {
        self.node_count() == 0
    }

    /// Adds a node with the associated `value` and returns its [`NodeIdx`].
    fn add_node(&mut self, value: N) -> NodeIdx;

    /// Adds an edge between the specified nodes with the associated `value`.
    ///
    /// # Returns
    /// * `Ok`: `EdgeIdx` of the new edge
    /// * `Err`:
    ///     * `GraphError::NodeNotFound(NodeIdx)`: the given `src` or `dst` isn't preset in the graph
    ///     * `GraphError::ContainsEdgeBetween`: there is already an edge between those nodes (not allowed in `SimpleGraph`)
    ///     * `GraphError::Loop`: the `src` and `dst` nodes are equal, the edge would be a loop (not allowed in `SimpleGraph`)
    fn try_add_edge(&mut self, src: NodeIdx, dst: NodeIdx, value: E)
        -> Result<EdgeIdx, GraphError>;

    /// Adds an edge between the specified nodes with the associated `value`.
    ///
    /// # Panics
    ///
    /// look at the `Returns/Err` in the docs from [`Graph::try_add_edge`]
    #[inline]
    fn add_edge(&mut self, src: NodeIdx, dst: NodeIdx, value: E) -> EdgeIdx {
        self.try_add_edge(src, dst, value).unwrap()
    }

    /// Returns `true` if the `node` is preset in the graph.
    fn has_node(&self, node: NodeIdx) -> bool;

    /// Returns `true` if an edge between the specified nodes exists.
    ///
    /// # Panics
    ///
    /// Panics if `src` or `dst` do not exist.
    fn contains_edge_between(&self, src: NodeIdx, dst: NodeIdx) -> bool;

    /// Removes the specified node from the graph, returning its value if it existed.
    fn remove_node(&mut self, index: NodeIdx) -> Option<N>;

    /// Removes the specified edge from the graph, returning its value if it existed.
    fn remove_edge(&mut self, index: EdgeIdx) -> Option<E>;
}

/// A more precise trait with functions special for a simple graph
pub trait SimpleGraph<N, E>: Graph<N, E> {
    /// Returns an edge between two nodes as `EdgeIdx`
    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> Result<Option<EdgeIdx>, GraphError>;

    /// Returns an edge between two nodes as `EdgeIdx`
    ///
    /// # Safety
    ///
    /// This function should only be called when the nodes and the edge between exists.
    unsafe fn edge_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx;
}
