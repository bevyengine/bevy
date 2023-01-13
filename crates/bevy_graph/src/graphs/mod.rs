pub mod simple;

pub mod edge;
pub mod impl_graph;
pub mod keys;

use crate::algos::bfs::BreadthFirstSearch;
use crate::error::GraphResult;

use self::keys::{EdgeIdx, NodeIdx};

pub trait Graph<N, E> {
    fn count(&self) -> usize;

    // Nodes
    fn new_node(&mut self, node: N) -> NodeIdx;

    fn get_node(&self, idx: NodeIdx) -> GraphResult<&N>;
    fn get_node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N>;

    fn remove_node(&mut self, node: NodeIdx) -> GraphResult<N>;

    fn has_node(&self, node: NodeIdx) -> bool;

    // Edges
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> GraphResult<EdgeIdx>;
    unsafe fn new_edge_unchecked(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx;

    fn get_edge(&self, edge: EdgeIdx) -> GraphResult<&E>;
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> GraphResult<&mut E>;

    fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E>;

    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx;
    fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)>; // TODO: can we use other type than Vec? maybe directly iterator?

    // Algos
    #[inline]
    fn algo_bfs(&self, start: NodeIdx) -> BreadthFirstSearch {
        BreadthFirstSearch::new(start, self.count())
    }
}
