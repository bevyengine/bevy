use slotmap::HopSlotMap;

use crate::{
    graphs::{
        edge::{Edge, EdgeRef},
        keys::EdgeIdx,
    },
    utils::wrapped_iterator::WrappedIterator,
};

/// An iterator which converts `&EdgeIdx` to a `EdgeRef<E>` of the graph
pub struct EdgesByIdx<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> {
    edges: &'g HopSlotMap<EdgeIdx, Edge<E>>,
    inner: I,
}

impl<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> EdgesByIdx<'g, E, I> {
    /// Creates a new `EdgesByIdx` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, edges: &'g HopSlotMap<EdgeIdx, Edge<E>>) -> Self {
        Self { edges, inner }
    }
}

impl<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> WrappedIterator<Self, EdgeRef<'g, E>, I>
    for EdgesByIdx<'g, E, I>
{
    #[inline]
    fn into_inner(self) -> I {
        self.inner
    }
}

impl<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> Iterator for EdgesByIdx<'g, E, I> {
    type Item = EdgeRef<'g, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            self.edges.get(*index).map(|edge| edge.as_ref_edge())
        } else {
            None
        }
    }
}
