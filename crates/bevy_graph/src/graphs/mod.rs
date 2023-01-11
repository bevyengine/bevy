pub mod simple;

pub mod edge;
pub mod keys;
pub mod trait_impl_util;

use crate::algos::bfs::BreadthFirstSearch;
use crate::error::GraphResult;

use self::keys::{EdgeIdx, NodeIdx};

#[allow(clippy::len_without_is_empty)]
pub trait Graph<N, E> {
    fn len(&self) -> usize;

    fn new_node(&mut self, node: N) -> NodeIdx;

    fn node(&self, idx: NodeIdx) -> GraphResult<&N>;
    fn node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N>;

    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx;

    fn get_edge(&self, edge: EdgeIdx) -> Option<&E>;
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> Option<&mut E>;

    fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E>;

    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx;
    fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)>; // TODO: can we use other type than Vec? maybe directly iterator?

    #[inline]
    fn algo_bfs(&self, start: NodeIdx) -> BreadthFirstSearch {
        BreadthFirstSearch::new(start, self.len())
    }
}
