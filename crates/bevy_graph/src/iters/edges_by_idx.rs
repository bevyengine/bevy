use std::borrow::Borrow;

use slotmap::HopSlotMap;

use crate::{
    graphs::{
        edge::{Edge, EdgeRef},
        keys::EdgeIdx,
        Graph,
    },
    utils::wrapped_indices_iterator::WrappedIndicesIterator,
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

impl<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> WrappedIndicesIterator<EdgeIdx>
    for EdgesByIdx<'g, E, &'g EdgeIdx, I>
{
    type IndicesIter = std::iter::Cloned<I>;

    #[inline]
    fn into_indices(self) -> Self::IndicesIter {
        self.inner.cloned()
    }
}

impl<'g, E: 'g, I: Iterator<Item = EdgeIdx>> WrappedIndicesIterator<EdgeIdx>
    for EdgesByIdx<'g, E, EdgeIdx, I>
{
    type IndicesIter = I;

    #[inline]
    fn into_indices(self) -> Self::IndicesIter {
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
