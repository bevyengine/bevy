pub mod graphs;

use slotmap::new_key_type;

new_key_type! {
    pub struct NodeIdx;
    pub struct EdgeIdx;
}

pub trait Graph<N, E> {
    fn new_node(&mut self, node: N) -> NodeIdx;

    fn node(&self, idx: NodeIdx) -> Option<&N>;
    fn node_mut(&mut self, idx: NodeIdx) -> Option<&mut N>;

    fn edge(&self, from: NodeIdx, to: NodeIdx) -> Option<&E>;
    fn edge_mut(&mut self, from: NodeIdx, to: NodeIdx) -> Option<&mut E>;
}

pub trait UndirectedGraph<N, E> {
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx; // TODO: does the end user really need the idx?
}

pub trait DirectedGraph<N, E> {
    fn new_edge(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> EdgeIdx; // TODO: does the end user really need the idx?
}
