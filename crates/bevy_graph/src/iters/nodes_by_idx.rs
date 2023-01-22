use slotmap::HopSlotMap;

use crate::{graphs::keys::NodeIdx, utils::wrapped_iterator::WrappedIterator};

/// An iterator which converts `&NodeIdx` to a `&'g N` of the graph
pub struct NodesByIdx<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> {
    nodes: &'g HopSlotMap<NodeIdx, N>,
    inner: I,
}

impl<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> NodesByIdx<'g, N, I> {
    /// Creates a new `NodesByIdx` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, nodes: &'g HopSlotMap<NodeIdx, N>) -> Self {
        Self { nodes, inner }
    }
}

impl<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> WrappedIterator<Self, &'g N, I>
    for NodesByIdx<'g, N, I>
{
    #[inline]
    fn into_inner(self) -> I {
        self.inner
    }
}

impl<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> Iterator for NodesByIdx<'g, N, I> {
    type Item = &'g N;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            self.nodes.get(*index)
        } else {
            None
        }
    }
}
