use slotmap::new_key_type;

use crate::error::GraphResult;

use super::Graph;

new_key_type! {
    /// a key that holds an index to a node in a graph
    pub struct NodeIdx;
    /// a key that holds an index to an edge in a graph
    pub struct EdgeIdx;
}

impl EdgeIdx {
    #[inline]
    pub fn get<N, E>(self, graph: &impl Graph<N, E>) -> GraphResult<&E> {
        graph.get_edge(self)
    }

    #[inline]
    pub fn get_mut<N, E>(self, graph: &mut impl Graph<N, E>) -> GraphResult<&mut E> {
        graph.get_edge_mut(self)
    }

    #[inline]
    pub fn remove<N, E>(self, graph: &mut impl Graph<N, E>) -> GraphResult<E> {
        graph.remove_edge(self)
    }
}
