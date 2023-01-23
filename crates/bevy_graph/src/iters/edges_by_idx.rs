use std::borrow::Borrow;

use slotmap::HopSlotMap;

use crate::{
    graphs::{
        edge::{Edge, EdgeRef},
        keys::EdgeIdx,
        Graph,
    },
    utils::wrapped_iterator::WrappedIterator,
};

/// An iterator which converts `(&)EdgeIdx` to a `EdgeRef<E>` of the graph
pub struct EdgesByIdx<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> {
    edges: &'g HopSlotMap<EdgeIdx, Edge<E>>,
    inner: I,
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> EdgesByIdx<'g, E, B, I> {
    /// Creates a new `EdgesByIdx` iterator over a graph with the provided `inner` iterator
    pub fn from_graph<N>(inner: I, graph: &'g mut impl Graph<N, E>) -> Self {
        Self {
            edges: unsafe { graph.edges_raw() },
            inner,
        }
    }

    /// Creates a new `EdgesByIdx` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, edges: &'g HopSlotMap<EdgeIdx, Edge<E>>) -> Self {
        Self { edges, inner }
    }
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> WrappedIterator<Self, EdgeRef<'g, E>, I>
    for EdgesByIdx<'g, E, B, I>
{
    #[inline]
    fn into_inner(self) -> I {
        self.inner
    }
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> Iterator for EdgesByIdx<'g, E, B, I> {
    type Item = EdgeRef<'g, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            self.edges
                .get(*index.borrow())
                .map(|edge| edge.as_ref_edge())
        } else {
            None
        }
    }
}
