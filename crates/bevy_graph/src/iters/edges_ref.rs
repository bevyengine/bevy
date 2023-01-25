use std::marker::PhantomData;

use crate::{
    graphs::edge::{Edge, EdgeRef},
    utils::wrapped_iterator::WrappedIterator,
};

/// An iterator which converts `&'g Edge<E>` to a `EdgeRef<'g, E>`
pub struct EdgesRef<'g, E: 'g, I: Iterator<Item = &'g Edge<E>>> {
    inner: I,
    phantom: PhantomData<E>,
}

impl<'g, E: 'g, I: Iterator<Item = &'g Edge<E>>> EdgesRef<'g, E, I> {
    /// Creates a new `EdgesRef` iterator with the provided `inner` iterator
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<'g, E: 'g, I: Iterator<Item = &'g Edge<E>>> WrappedIterator<&'g Edge<E>>
    for EdgesRef<'g, E, I>
{
    type Inner = I;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        self.inner
    }
}

impl<'g, E: 'g, I: Iterator<Item = &'g Edge<E>>> Iterator for EdgesRef<'g, E, I> {
    type Item = EdgeRef<'g, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|edge| edge.as_ref_edge())
    }
}
