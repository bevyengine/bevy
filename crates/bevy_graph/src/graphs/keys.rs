use slotmap::new_key_type;

use crate::error::GraphResult;

use super::{GetEdge, NewEdge};

new_key_type! {
    pub struct NodeIdx;
    pub struct EdgeIdx;
}

impl EdgeIdx {
    #[inline]
    pub fn get<N, E>(self, graph: &impl GetEdge<E>) -> Option<&E> {
        graph.get_edge(self)
    }

    #[inline]
    pub fn get_mut<N, E>(self, graph: &mut impl GetEdge<E>) -> Option<&mut E> {
        graph.get_edge_mut(self)
    }

    #[inline]
    pub fn remove<N, E>(self, graph: &mut impl NewEdge<E>) -> GraphResult<E> {
        graph.remove_edge(self)
    }
}
