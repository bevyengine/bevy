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

    fn edge_id_between(&self, from: NodeIdx, to: NodeIdx) -> Option<EdgeIdx>;

    fn edge_by_id(&self, edge: EdgeIdx) -> Option<&E>;
    fn edge_by_id_mut(&mut self, edge: EdgeIdx) -> Option<&mut E>;

    #[inline]
    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> Option<&E> {
        self.edge_by_id(self.edge_id_between(from, to)?)
    }
    #[inline]
    fn edge_between_mut(&mut self, from: NodeIdx, to: NodeIdx) -> Option<&mut E> {
        self.edge_by_id_mut(self.edge_id_between(from, to)?)
    }
}

pub trait UndirectedGraph<N, E> {
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx;

    fn remove_edge_between(&mut self, node: NodeIdx, other: NodeIdx) -> Option<E>;
}

pub trait DirectedGraph<N, E> {
    fn new_edge(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> EdgeIdx;

    fn remove_edge_between(&mut self, from: NodeIdx, to: NodeIdx) -> Option<E>;
}
