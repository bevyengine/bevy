use slotmap::HopSlotMap;

use crate::graphs::{
    edge::{Edge, EdgeMut},
    keys::EdgeIdx,
};

/// An iterator which converts `&EdgeIdx` to a `EdgeMut<E>` of the graph
pub struct EdgesByIdxMut<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> {
    edges: &'g mut HopSlotMap<EdgeIdx, Edge<E>>,
    inner: I,
}

impl<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> EdgesByIdxMut<'g, E, I> {
    /// Creates a new `EdgesByIdxMut` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, edges: &'g mut HopSlotMap<EdgeIdx, Edge<E>>) -> Self {
        Self { edges, inner }
    }
}

impl<'g, E: 'g, I: Iterator<Item = &'g EdgeIdx>> Iterator for EdgesByIdxMut<'g, E, I> {
    type Item = EdgeMut<'g, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            // Unsafe necessary because Rust can't deduce that we won't
            // return multiple references to the same value.
            unsafe {
                self.edges.get_mut(*index).map(|edge| {
                    let ptr: *mut E = &mut edge.2;
                    EdgeMut(edge.0, edge.1, &mut *ptr)
                })
            }
        } else {
            None
        }
    }
}
