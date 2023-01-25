/// `Vec` implementation of a graph
pub mod list;
/// `HashMap` implementation of a graph
pub mod map;

/// Adjacency storage enum helper: `Directed` or `Undirected`
pub mod adjacency_storage;
/// An edge between nodes that store data of type `E`.
pub mod edge;
/// The `NodeIdx` and `EdgeIdx` structs
pub mod keys;

use slotmap::HopSlotMap;

use crate::{error::GraphError, utils::wrapped_iterator::WrappedIterator};

use self::{
    edge::{Edge, EdgeMut, EdgeRef},
    keys::{EdgeIdx, NodeIdx},
};

// NOTE: There should always be a common function and if needed a more precise function which the common function wraps.
//       Example: `edges_between` is `trait Graph` general and has support for Simple- and Multigraphs.
//                `edge_between` is only available for `SimpleGraph` but is also called from `edges_between`.

/// A trait with all the common functions for a graph
pub trait Graph<N, E> {
    /// Iterator fix because TAIT not available
    type NodeIndices<'n>: Iterator<Item = NodeIdx>
    where
        Self: 'n,
        E: 'n;
    /// Iterator fix because TAIT not available
    type Nodes<'n>: Iterator<Item = &'n N>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type NodesMut<'n>: Iterator<Item = &'n mut N>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type EdgeIndices<'e>: Iterator<Item = EdgeIdx>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type Edges<'e>: Iterator<Item = EdgeRef<'e, E>>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type EdgesMut<'e>: Iterator<Item = EdgeMut<'e, E>>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type EdgesOf<'e>: Iterator<Item = EdgeRef<'e, E>> + WrappedIterator<&'e EdgeIdx>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type EdgesOfMut<'e>: Iterator<Item = EdgeMut<'e, E>> + WrappedIterator<&'e EdgeIdx>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type Neighbors<'n>: Iterator<Item = &'n N> + WrappedIterator<&'n NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type NeighborsMut<'n>: Iterator<Item = &'n mut N> + WrappedIterator<&'n NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type Isolated<'n>: Iterator<Item = &'n N> + WrappedIterator<NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type IsolatedMut<'n>: Iterator<Item = &'n mut N> + WrappedIterator<NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type IncomingEdgesOf<'e>: Iterator<Item = EdgeRef<'e, E>> + WrappedIterator<&'e EdgeIdx>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type IncomingEdgesOfMut<'e>: Iterator<Item = EdgeMut<'e, E>> + WrappedIterator<&'e EdgeIdx>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type OutgoingEdgesOf<'e>: Iterator<Item = EdgeRef<'e, E>> + WrappedIterator<&'e EdgeIdx>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type OutgoingEdgesOfMut<'e>: Iterator<Item = EdgeMut<'e, E>> + WrappedIterator<&'e EdgeIdx>
    where
        Self: 'e,
        E: 'e;
    /// Iterator fix because TAIT not available
    type InNeighbors<'n>: Iterator<Item = &'n N> + WrappedIterator<&'n NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type InNeighborsMut<'n>: Iterator<Item = &'n mut N> + WrappedIterator<&'n NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type OutNeighbors<'n>: Iterator<Item = &'n N> + WrappedIterator<&'n NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type OutNeighborsMut<'n>: Iterator<Item = &'n mut N> + WrappedIterator<&'n NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type Sources<'n>: Iterator<Item = &'n N> + WrappedIterator<NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type SourcesMut<'n>: Iterator<Item = &'n mut N> + WrappedIterator<NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type Sinks<'n>: Iterator<Item = &'n N> + WrappedIterator<NodeIdx>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type SinksMut<'n>: Iterator<Item = &'n mut N> + WrappedIterator<NodeIdx>
    where
        Self: 'n,
        N: 'n;

    /// Creates a new graph
    fn new() -> Self
    where
        Self: Sized;

    /// Constructs a new, empty graph with the specified node and edge capacity.
    /// The graph will be able to hold exactly `node_capacity` nodes and `edge_capacity` edges
    /// elements without reallocating.
    ///
    /// If the capacites are zero, the graph will not allocate.
    fn with_capacity(node_capacity: usize, edge_capacity: usize) -> Self;

    /// Returns the number of nodes and edges the graph can hold without reallocating.
    fn capacity(&self) -> (usize, usize);

    /// Returns the number of nodes the graph can hold without reallocating.
    fn node_capacity(&self) -> usize;

    /// Returns the number of edges the graph can hold without reallocating.
    fn edge_capacity(&self) -> usize;

    /// Reserves capacity for at least `additional` more nodes to be inserted in the given `Self`.
    /// The collection may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_nodes`, the node capacity will be greater than or equal to
    /// `self.node_count() + additional`. Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///     
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    fn reserve_nodes(&mut self, additional: usize);

    /// Reserves capacity for at least `additional` more edges to be inserted in the given `Self`.
    /// The collection may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_edges`, the edge capacity will be greater than or equal to
    /// `self.edge_count() + additional`. Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///     
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    fn reserve_edges(&mut self, additional: usize);

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

    /// Removes all edges from the graph.
    fn clear_edges(&mut self);

    /// Removes all nodes and edges from the graph.
    fn clear(&mut self);

    /// Returns a reference to the specified node.
    fn get_node(&self, index: NodeIdx) -> Option<&N>;

    /// Returns a mutable reference to the specified node.
    fn get_node_mut(&mut self, index: NodeIdx) -> Option<&mut N>;

    /// Returns a raw edge handle to the specified edge.
    ///
    /// # Safety
    ///
    /// This function should only be called when you really know what you are doing.
    unsafe fn get_edge_raw(&mut self, index: EdgeIdx) -> Option<&mut Edge<E>>;

    /// Returns a reference to the specified edge.
    fn get_edge(&self, index: EdgeIdx) -> Option<EdgeRef<E>>;

    /// Returns a mutable reference to the specified edge.
    fn get_edge_mut(&mut self, index: EdgeIdx) -> Option<EdgeMut<E>>;

    /// Returns the number of edges connected to the specified node.
    ///
    /// In multi-graphs, edges that form self-loops add 2 to the degree.
    fn degree(&self, index: NodeIdx) -> usize;

    /// Returns an iterator over all `NodeIdx`s.
    fn node_indices(&self) -> Self::NodeIndices<'_>;

    /// Returns a immutable raw handle to the nodes slotmap.
    ///
    /// # Safety
    ///
    /// This function should only be called when you really know what you are doing.
    unsafe fn nodes_raw(&self) -> &HopSlotMap<NodeIdx, N>;

    /// Returns an iterator over all nodes.
    fn nodes(&self) -> Self::Nodes<'_>;

    /// Returns a mutable raw handle to the nodes slotmap.
    ///
    /// # Safety
    ///
    /// This function should only be called when you really know what you are doing.
    unsafe fn nodes_mut_raw(&mut self) -> &mut HopSlotMap<NodeIdx, N>;

    /// Returns a mutable iterator over all nodes.
    fn nodes_mut(&mut self) -> Self::NodesMut<'_>;

    /// Returns an iterator over all `EdgeIdx`s.
    fn edge_indices(&self) -> Self::EdgeIndices<'_>;

    /// Returns a immutable raw handle to the edges slotmap.
    ///
    /// # Safety
    ///
    /// This function should only be called when you really know what you are doing.
    unsafe fn edges_raw(&self) -> &HopSlotMap<EdgeIdx, Edge<E>>;

    /// Returns an iterator over all edges.
    fn edges(&self) -> Self::Edges<'_>;

    /// Returns a mutable raw handle to the edges slotmap.
    ///
    /// # Safety
    ///
    /// This function should only be called when you really know what you are doing.
    unsafe fn edges_mut_raw(&mut self) -> &mut HopSlotMap<EdgeIdx, Edge<E>>;

    /// Returns a mutable iterator over all edges.
    fn edges_mut(&mut self) -> Self::EdgesMut<'_>;

    /// Returns an iterator over the edges of the specified node.
    fn edges_of(&self, index: NodeIdx) -> Self::EdgesOf<'_>;

    /// Returns a mutable iterator over the edges of the specified node.
    fn edges_of_mut(&mut self, index: NodeIdx) -> Self::EdgesOfMut<'_>;

    /// Returns an iterator over the nodes the share an edge with the specified node.
    fn neighbors(&self, index: NodeIdx) -> Self::Neighbors<'_>;

    /// Returns a mutable iterator over the nodes the share an edge with the specified node.
    fn neighbors_mut(&mut self, index: NodeIdx) -> Self::NeighborsMut<'_>;

    /// Returns an iterator over all nodes with zero degree.
    fn isolated(&self) -> Self::Isolated<'_>;

    /// Returns a mutable iterator over all nodes with zero degree.
    fn isolated_mut(&mut self) -> Self::IsolatedMut<'_>;

    /// Returns the number of edges going into the specified node.
    fn in_degree(&self, index: NodeIdx) -> usize;

    /// Returns the number of edges coming out of the specified node.
    fn out_degree(&self, index: NodeIdx) -> usize;

    /// Returns an iterator over the edge indices going into the specified node.
    fn incoming_edges_of(&self, index: NodeIdx) -> Self::IncomingEdgesOf<'_>;

    /// Returns a mutable iterator over the edges going into the specified node.
    fn incoming_edges_of_mut(&mut self, index: NodeIdx) -> Self::IncomingEdgesOfMut<'_>;

    /// Returns an iterator over the edge indices coming out of the specified node.
    fn outgoing_edges_of(&self, index: NodeIdx) -> Self::OutgoingEdgesOf<'_>;

    /// Returns a mutable iterator over the edges coming out of the specified node.
    fn outgoing_edges_of_mut(&mut self, index: NodeIdx) -> Self::OutgoingEdgesOfMut<'_>;

    /// Returns an iterator over the the specified node's direct predecessors.
    fn in_neighbors(&self, index: NodeIdx) -> Self::InNeighbors<'_>;

    /// Returns a mutable iterator over the the specified node's direct predecessors.
    fn in_neighbors_mut(&mut self, index: NodeIdx) -> Self::InNeighborsMut<'_>;

    /// Returns an iterator over the the specified node's direct successors.
    fn out_neighbors(&self, index: NodeIdx) -> Self::OutNeighbors<'_>;

    /// Returns a mutable iterator over the the specified node's direct successors.
    fn out_neighbors_mut(&mut self, index: NodeIdx) -> Self::OutNeighborsMut<'_>;

    /// Returns an iterator over all nodes with zero in-degree.
    fn sources(&self) -> Self::Sources<'_>;

    /// Returns a mutable iterator over all nodes with zero in-degree.
    fn sources_mut(&mut self) -> Self::SourcesMut<'_>;

    /// Returns an iterator over all nodes with zero out-degree.
    fn sinks(&self) -> Self::Sinks<'_>;

    /// Returns a mutable iterator over all nodes with zero out-degree.
    fn sinks_mut(&mut self) -> Self::SinksMut<'_>;
}

/// A more precise trait with functions special for simple graphs
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

/// A more precise trait with functions special for directed graphs
pub trait DirectedGraph<N, E>: Graph<N, E> {
    /// Iterator fix because TAIT not available
    type Ancestors<'n>: Iterator<Item = &'n N>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type AncestorsMut<'n>: Iterator<Item = &'n mut N>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type Descendants<'n>: Iterator<Item = &'n N>
    where
        Self: 'n,
        N: 'n;
    /// Iterator fix because TAIT not available
    type DescendantsMut<'n>: Iterator<Item = &'n mut N>
    where
        Self: 'n,
        N: 'n;

    /// Reverse the direction of all edges in the graph.
    fn reverse(&mut self);

    /// Reverse the direction of the specified edge.
    fn reverse_edge(&mut self, index: EdgeIdx);

    /// Returns an iterator that visits all nodes that can reach the specified node.
    fn ancestors(&self, index: NodeIdx) -> Self::Ancestors<'_>;

    /// Returns a mutable iterator that visits all nodes that can reach the specifed node.
    fn ancestors_mut(&mut self, index: NodeIdx) -> Self::AncestorsMut<'_>;

    /// Returns iterator that visits all nodes that are reachable from the specified node.
    fn descendants(&self, index: NodeIdx) -> Self::Descendants<'_>;

    /// Returns a mutable iterator that visits all nodes that are reachable from the specified node.
    fn descendants_mut(&mut self, index: NodeIdx) -> Self::DescendantsMut<'_>;
}
