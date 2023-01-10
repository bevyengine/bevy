pub mod algos;
pub mod error;
pub mod graphs;

use error::GraphResult;
use slotmap::new_key_type;

new_key_type! {
    pub struct NodeIdx;
    pub struct EdgeIdx;
}

impl EdgeIdx {
    #[inline]
    pub fn get<N, E>(self, graph: &impl Graph<N, E>) -> Option<&E> {
        graph.get_edge(self)
    }

    #[inline]
    pub fn get_mut<N, E>(self, graph: &mut impl Graph<N, E>) -> Option<&mut E> {
        graph.get_edge_mut(self)
    }
}

pub trait Graph<N, E> {
    fn new_node(&mut self, node: N) -> NodeIdx;

    fn node(&self, idx: NodeIdx) -> GraphResult<&N>;
    fn node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N>;

    fn len(&self) -> usize;

    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx;

    fn get_edge(&self, edge: EdgeIdx) -> Option<&E>;
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> Option<&mut E>;

    fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)>; // TODO: can we use other type than Vec? maybe directly iterator?
}

pub trait UndirectedGraph<N, E> {
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx;

    fn remove_edge_between(&mut self, node: NodeIdx, other: NodeIdx) -> Option<E>;
}

pub trait DirectedGraph<N, E> {
    fn new_edge(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> EdgeIdx;

    fn remove_edge_between(&mut self, from: NodeIdx, to: NodeIdx) -> Option<E>;
}
