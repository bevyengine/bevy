use slotmap::HopSlotMap;

use crate::graphs::keys::NodeIdx;

/// An iterator which converts `&NodeIdx` to a `&'g mut N` of the graph
pub struct NodesByIdxMut<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> {
    nodes: &'g mut HopSlotMap<NodeIdx, N>,
    inner: I,
}

impl<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> NodesByIdxMut<'g, N, I> {
    /// Creates a new `NodesByIdxMut` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, nodes: &'g mut HopSlotMap<NodeIdx, N>) -> Self {
        Self { nodes, inner }
    }

    /// Returns the inner iterator which yields `NodeIdx`
    #[inline]
    pub fn into_indices_iter(self) -> I {
        self.inner
    }
}

impl<'g, N: 'g, I: Iterator<Item = &'g NodeIdx>> Iterator for NodesByIdxMut<'g, N, I> {
    type Item = &'g mut N;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            // Unsafe necessary because Rust can't deduce that we won't
            // return multiple references to the same value.
            unsafe {
                self.nodes.get_mut(*index).map(|node| {
                    let ptr: *mut N = &mut *node;
                    &mut *ptr
                })
            }
        } else {
            None
        }
    }
}
