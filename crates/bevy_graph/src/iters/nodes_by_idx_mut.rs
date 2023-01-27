use std::borrow::Borrow;

use slotmap::HopSlotMap;

use crate::{
    graphs::{keys::NodeIdx, Graph},
    utils::wrapped_indices_iterator::WrappedIndicesIterator,
};

/// An iterator which converts `(&)NodeIdx` to a `&'g mut N` of the graph
pub struct NodesByIdxMut<'g, N: 'g, B: Borrow<NodeIdx>, I: Iterator<Item = B>> {
    nodes: &'g mut HopSlotMap<NodeIdx, N>,
    inner: I,
}

impl<'g, N: 'g, B: Borrow<NodeIdx>, I: Iterator<Item = B>> NodesByIdxMut<'g, N, B, I> {
    /// Creates a new `NodesByIdxMut` iterator over a graph with the provided `inner` iterator
    pub fn from_graph<E>(inner: I, graph: &'g mut impl Graph<N, E>) -> Self {
        Self {
            nodes: unsafe { graph.nodes_mut_raw() },
            inner,
        }
    }

    /// Creates a new `NodesByIdxMut` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, nodes: &'g mut HopSlotMap<NodeIdx, N>) -> Self {
        Self { nodes, inner }
    }
}

impl<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> WrappedIndicesIterator<NodeIdx>
    for NodesByIdxMut<'g, N, &'g NodeIdx, I>
{
    type IndicesIter = std::iter::Cloned<I>;

    #[inline]
    fn into_indices(self) -> Self::IndicesIter {
        self.inner.cloned()
    }
}

impl<'g, N: 'g, I: Iterator<Item = NodeIdx>> WrappedIndicesIterator<NodeIdx>
    for NodesByIdxMut<'g, N, NodeIdx, I>
{
    type IndicesIter = I;

    #[inline]
    fn into_indices(self) -> Self::IndicesIter {
        self.inner
    }
}

impl<'g, N: 'g, B: Borrow<NodeIdx>, I: Iterator<Item = B>> Iterator for NodesByIdxMut<'g, N, B, I> {
    type Item = &'g mut N;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            // Unsafe necessary because Rust can't deduce that we won't
            // return multiple references to the same value.
            unsafe {
                self.nodes.get_mut(*index.borrow()).map(|node| {
                    let ptr: *mut N = &mut *node;
                    &mut *ptr
                })
            }
        } else {
            None
        }
    }
}
