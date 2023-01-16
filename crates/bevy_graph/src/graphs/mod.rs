pub mod multi;
pub mod simple;

pub mod edge;
pub mod impl_graph;
pub mod keys;

use crate::algos::{bfs::BreadthFirstSearch, dfs::DepthFirstSearch};
use crate::error::GraphResult;

use self::keys::{EdgeIdx, NodeIdx};

// NOTE: There should always be a general API function and a more precise API function for one problem with multiple signatures needed.
//       Example: `edges_between` is `trait Graph` general and has support for Simple- and Multigraphs.
//                `edge_between` is only available for `SimpleGraph` but is also called from `edges_between`.

pub trait Graph<N, E> {
    /// Creates a new graph
    fn new() -> Self
    where
        Self: Sized;

    fn count(&self) -> usize;

    ////////////////////////////
    // Nodes
    ////////////////////////////

    /// Creates a new node with the given value in the graph and returns its `NodeIdx`
    fn new_node(&mut self, node: N) -> NodeIdx;

    /// Returns a reference to the value of a given node
    fn get_node(&self, idx: NodeIdx) -> GraphResult<&N>;

    /// Returns a reference to the value of a given node
    ///
    /// # Safety
    ///
    /// This function should only be called when the Node for the `NodeIdx` exists
    unsafe fn get_node_unchecked(&self, idx: NodeIdx) -> &N;

    /// Returns a mutable reference to the value of a given node
    fn get_node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N>;

    /// Returns a mutable reference to the value of a given node
    ///
    /// # Safety
    ///
    /// This function should only be called when the Node for the `NodeIdx` exists
    unsafe fn get_node_unchecked_mut(&mut self, idx: NodeIdx) -> &mut N;

    /// Removes a node from the graph by its `NodeIdx`
    fn remove_node(&mut self, node: NodeIdx) -> GraphResult<N>;

    /// Returns true as long as the node from the given `NodeIdx` is preset
    fn has_node(&self, node: NodeIdx) -> bool;

    ////////////////////////////
    // Edges
    ////////////////////////////

    /// Creates a new edge between the two nodes and with the given value in the graph and returns its `EdgeIdx`
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> GraphResult<EdgeIdx>;

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
    /// # let graph = SimpleMapGraph::new();
    /// let from = graph.new_node();
    /// let to = graph.new_node();
    /// let edge = graph.new_edge(from, to, 12).unwrap();
    /// assert_eq!(edge.get(&graph), &12);
    /// ```
    fn get_edge(&self, edge: EdgeIdx) -> GraphResult<&E>;

    /// Returns a mutable reference to the value of a given edge
    ///
    /// ## Inline helper
    ///
    /// This function can also be directly called by the `EdgeIdx`:
    /// ```
    /// # use bevy_graph::graphs::{Graph, simple::SimpleMapGraph};
    /// # let graph = SimpleMapGraph::new();
    /// let from = graph.new_node();
    /// let to = graph.new_node();
    /// let edge = graph.new_edge(from, to, 12).unwrap();
    /// assert_eq!(edge.get_mut(&graph), &12);
    /// ```
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> GraphResult<&mut E>;

    /// Remove an edge by its `EdgeIdx` and returns the edge data
    ///
    /// ## Inline helper
    ///
    /// This function can also be directly called by the `EdgeIdx`:
    /// ```
    /// # use bevy_graph::graphs::{Graph, simple::SimpleMapGraph};
    /// # let graph = SimpleMapGraph::new();
    /// let from = graph.new_node();
    /// let to = graph.new_node();
    /// let edge = graph.new_edge(from, to, 12).unwrap();
    /// assert_eq!(edge.remove(&graph).unwrap(), 12);
    /// ```
    fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E>;

    /// Remove an edge by its `EdgeIdx` and returns the edge data
    ///
    /// ## Inline helper
    ///
    /// This function can also be directly called by the `EdgeIdx`:
    /// ```
    /// # use bevy_graph::graphs::{Graph, simple::SimpleMapGraph};
    /// # let graph = SimpleMapGraph::new();
    /// let from = graph.new_node();
    /// let to = graph.new_node();
    /// let edge = graph.new_edge(from, to, 12).unwrap();
    /// assert_eq!(edge.remove(&graph).unwrap(), 12);
    /// ```
    ///
    /// # Safety
    ///
    /// This function should only be called when the edge exists
    unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E;

    /// Returns a `Vec` of all edges between two nodes as `EdgeIdx`
    fn edges_between(&self, from: NodeIdx, to: NodeIdx) -> GraphResult<Vec<EdgeIdx>>;

    /// Returns a `Vec` of all edges between two nodes as `EdgeIdx`
    ///
    /// # Safety
    ///
    /// This function should only be called when the nodes and the edges between exists
    unsafe fn edges_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> Vec<EdgeIdx>;

    /// Returns a `Vec` of all edges the node is outgoing from.
    fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)>; // TODO: can we use other type than Vec? maybe directly iterator?

    ////////////////////////////
    // Algos
    ////////////////////////////

    /// Makes a `BreadthFirstSearch` over this graph
    #[inline]
    fn algo_bfs(&self, start: NodeIdx) -> BreadthFirstSearch {
        BreadthFirstSearch::new(start, self.count())
    }

    /// Makes a `DepthFirstSearch` over this graph
    #[inline]
    fn algo_dfs(&self, start: NodeIdx) -> DepthFirstSearch {
        DepthFirstSearch::new(start, self.count())
    }
}

pub trait SimpleGraph<N, E>: Graph<N, E> {
    /// Returns an edge between two nodes as `EdgeIdx`
    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> GraphResult<Option<EdgeIdx>>;

    /// Returns an edge between two nodes as `EdgeIdx`
    ///
    /// # Safety
    ///
    /// This function should only be called when the nodes and the edge between exists.
    unsafe fn edge_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx;
}
