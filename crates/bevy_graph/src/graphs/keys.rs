use slotmap::new_key_type;

use crate::error::GraphResult;

use super::{DirectedGraph, Graph, UndirectedGraph};

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

    #[inline]
    // TODO: make them 1 function when reorganizing traits
    pub fn remove_directed<N, E>(self, graph: &mut impl DirectedGraph<N, E>) -> GraphResult<E> {
        graph.remove_edge(self)
    }

    #[inline]
    // TODO: make them 1 function when reorganizing traits
    pub fn remove_undirected<N, E>(self, graph: &mut impl UndirectedGraph<N, E>) -> GraphResult<E> {
        graph.remove_edge(self)
    }
}
