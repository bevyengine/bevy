use slotmap::new_key_type;

use crate::error::GraphError;

use super::Graph;

new_key_type! {
    /// a key that holds an index to a node in a graph
    pub struct NodeIdx;
    /// a key that holds an index to an edge in a graph
    pub struct EdgeIdx;
}

impl EdgeIdx {
    /// shorthand for getting an immutable reference to the edge data
    #[inline]
    pub fn get<N, E>(self, graph: &impl Graph<N, E>) -> Result<&E, GraphError> {
        graph.get_edge(self)
    }

    /// shorthand for getting a mutable reference to the edge data
    #[inline]
    pub fn get_mut<N, E>(self, graph: &mut impl Graph<N, E>) -> Result<&mut E, GraphError> {
        graph.get_edge_mut(self)
    }

    /// shorthand for removing this edge
    #[inline]
    pub fn remove<N, E>(self, graph: &mut impl Graph<N, E>) -> Result<E, GraphError> {
        graph.remove_edge(self)
    }
}
