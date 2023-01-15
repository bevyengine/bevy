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
    fn new() -> Self
    where
        Self: Sized;

    fn count(&self) -> usize;

    ////////////////////////////
    // Nodes
    ////////////////////////////
    fn new_node(&mut self, node: N) -> NodeIdx;

    fn get_node(&self, idx: NodeIdx) -> GraphResult<&N>;
    /// # Safety
    ///
    /// This function should only be called when the Node for the `NodeIdx` exists.
    unsafe fn get_node_unchecked(&self, idx: NodeIdx) -> &N;
    fn get_node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N>;
    /// # Safety
    ///
    /// This function should only be called when the Node for the `NodeIdx` exists.
    unsafe fn get_node_unchecked_mut(&mut self, idx: NodeIdx) -> &mut N;

    fn remove_node(&mut self, node: NodeIdx) -> GraphResult<N>;

    fn has_node(&self, node: NodeIdx) -> bool;

    ////////////////////////////
    // Edges
    ////////////////////////////
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> GraphResult<EdgeIdx>;
    /// # Safety
    ///
    /// This function should only be called when the nodes exist and there is no equal edge.
    unsafe fn new_edge_unchecked(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx;

    fn get_edge(&self, edge: EdgeIdx) -> GraphResult<&E>;
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> GraphResult<&mut E>;

    fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E>;
    /// # Safety
    ///
    /// This function should only be called when the edge exists.
    unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E;

    fn edges_between(&self, from: NodeIdx, to: NodeIdx) -> GraphResult<Vec<EdgeIdx>>;
    /// # Safety
    ///
    /// This function should only be called when the nodes and the edges between exists.
    unsafe fn edges_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> Vec<EdgeIdx>;
    fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)>; // TODO: can we use other type than Vec? maybe directly iterator?

    ////////////////////////////
    // Algos
    ////////////////////////////
    #[inline]
    fn algo_bfs(&self, start: NodeIdx) -> BreadthFirstSearch {
        BreadthFirstSearch::new(start, self.count())
    }
    #[inline]
    fn algo_dfs(&self, start: NodeIdx) -> DepthFirstSearch {
        DepthFirstSearch::new(start, self.count())
    }
}

pub trait SimpleGraph<N, E>: Graph<N, E> {
    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> GraphResult<Option<EdgeIdx>>;
    /// # Safety
    ///
    /// This function should only be called when the nodes and the edge between exists.
    unsafe fn edge_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx;
}
