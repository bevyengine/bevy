use std::borrow::Borrow;

use slotmap::HopSlotMap;

use crate::{
    graphs::{
        edge::{Edge, EdgeMut},
        keys::EdgeIdx,
        Graph,
    },
    utils::wrapped_iterator::WrappedIterator,
};

/// An iterator which converts `(&)EdgeIdx` to a `EdgeMut<E>` of the graph
pub struct EdgesByIdxMut<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> {
    edges: &'g mut HopSlotMap<EdgeIdx, Edge<E>>,
    inner: I,
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> EdgesByIdxMut<'g, E, B, I> {
    /// Creates a new `EdgesByIdxMut` iterator over a graph with the provided `inner` iterator
    pub fn from_graph<N>(inner: I, graph: &'g mut impl Graph<N, E>) -> Self {
        Self {
            edges: unsafe { graph.edges_mut_raw() },
            inner,
        }
    }

    /// Creates a new `EdgesByIdxMut` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, edges: &'g mut HopSlotMap<EdgeIdx, Edge<E>>) -> Self {
        Self { edges, inner }
    }
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> WrappedIterator<B>
    for EdgesByIdxMut<'g, E, B, I>
{
    type Inner = I;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        self.inner
    }
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> Iterator for EdgesByIdxMut<'g, E, B, I> {
    type Item = EdgeMut<'g, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            // Unsafe necessary because Rust can't deduce that we won't
            // return multiple references to the same value.
            unsafe {
                self.edges.get_mut(*index.borrow()).map(|edge| {
                    let ptr: *mut E = &mut edge.2;
                    EdgeMut(edge.0, edge.1, &mut *ptr)
                })
            }
        } else {
            None
        }
    }
}
