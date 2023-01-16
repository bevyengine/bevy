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

    /// Returns `true` if the edges in the graph are undirected.
    fn is_undirected(&self) -> bool;

    /// Returns the number of nodes in the graph.
    fn node_count(&self) -> usize;

    /// Returns the number of edges in the graph.
    fn edge_count(&self) -> usize;

    /// Returns `true` if the graph has no nodes.
    fn is_empty(&self) -> bool {
        self.node_count() == 0
    }
    ////////////////////////////
    // Nodes
    ////////////////////////////

    /// Creates a new node with the given value in the graph and returns its `NodeIdx`
    fn new_node(&mut self, node: N) -> NodeIdx;

    /// Returns a reference to the value of a given node
    fn get_node(&self, idx: NodeIdx) -> Result<&N, GraphError>;

    /// Returns a reference to the value of a given node
    ///
    /// # Safety
    ///
    /// This function should only be called when the Node for the `NodeIdx` exists
    unsafe fn get_node_unchecked(&self, idx: NodeIdx) -> &N;

    /// Returns a mutable reference to the value of a given node
    fn get_node_mut(&mut self, idx: NodeIdx) -> Result<&mut N, GraphError>;

    /// Returns a mutable reference to the value of a given node
    ///
    /// # Safety
    ///
    /// This function should only be called when the Node for the `NodeIdx` exists
    unsafe fn get_node_unchecked_mut(&mut self, idx: NodeIdx) -> &mut N;

    /// Removes a node from the graph by its `NodeIdx`
    fn remove_node(&mut self, node: NodeIdx) -> Result<N, GraphError>;

    /// Returns true as long as the node from the given `NodeIdx` is preset
    fn has_node(&self, node: NodeIdx) -> bool;

    ////////////////////////////
    // Edges
    ////////////////////////////

    /// Creates a new edge between the two nodes and with the given value in the graph and returns its `EdgeIdx`
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> Result<EdgeIdx, GraphError>;

    /// Creates a new edge between the two nodes and with the given value in the graph and returns its `EdgeIdx`
    ///
    /// # Safety
    ///
    /// This function should only be called when the nodes exist and there is no equal edge.
    unsafe fn new_edge_unchecked(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx;

    /// Returns a reference to the value of a given edge
    ///
    /// ## Inline helper
    ///
    /// This function can also be directly called by the `EdgeIdx`:
    /// ```
    /// # use bevy_graph::graphs::{Graph, simple::SimpleMapGraph};
    /// # let mut graph = SimpleMapGraph::<(), i32, true>::new();
    /// let from = graph.new_node(());
    /// let to = graph.new_node(());
    /// let edge = graph.new_edge(from, to, 12).unwrap();
    /// assert_eq!(edge.get(&graph).unwrap(), &12);
    /// ```
    fn get_edge(&self, edge: EdgeIdx) -> Result<&E, GraphError>;

    /// Returns a mutable reference to the value of a given edge
    ///
    /// ## Inline helper
    ///
    /// This function can also be directly called by the `EdgeIdx`:
    /// ```
    /// # use bevy_graph::graphs::{Graph, simple::SimpleMapGraph};
    /// # let mut graph = SimpleMapGraph::<(), i32, true>::new();
    /// let from = graph.new_node(());
    /// let to = graph.new_node(());
    /// let edge = graph.new_edge(from, to, 12).unwrap();
    /// assert_eq!(edge.get_mut(&mut graph).unwrap(), &12);
    /// ```
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> Result<&mut E, GraphError>;

    /// Remove an edge by its `EdgeIdx` and returns the edge data
    ///
    /// ## Inline helper
    ///
    /// This function can also be directly called by the `EdgeIdx`:
    /// ```
    /// # use bevy_graph::graphs::{Graph, simple::SimpleMapGraph};
    /// # let mut graph = SimpleMapGraph::<(), i32, true>::new();
    /// let from = graph.new_node(());
    /// let to = graph.new_node(());
    /// let edge = graph.new_edge(from, to, 12).unwrap();
    /// assert_eq!(edge.remove(&mut graph).unwrap(), 12);
    /// ```
    fn remove_edge(&mut self, edge: EdgeIdx) -> Result<E, GraphError>;

    /// Remove an edge by its `EdgeIdx` and returns the edge data
    ///
    /// # Safety
    ///
    /// This function should only be called when the edge exists
    unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E;

    /// Returns a `Vec` of all edges between two nodes as `EdgeIdx`
    fn edges_between(&self, from: NodeIdx, to: NodeIdx) -> Result<Vec<EdgeIdx>, GraphError>;

    /// Returns a `Vec` of all edges between two nodes as `EdgeIdx`
    ///
    /// # Safety
    ///
    /// This function should only be called when the nodes and the edges between exists
    unsafe fn edges_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> Vec<EdgeIdx>;

    /// Returns a `Vec` of all edges the node is outgoing from.
    fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)>; // TODO: can we use other type than Vec? maybe directly iterator?
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
