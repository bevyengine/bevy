use std::marker::PhantomData;

use crate::graphs::edge::{Edge, EdgeMut};

/// An iterator which converts `&'g mut Edge<E>` to a `EdgeMut<'g, E>`
pub struct EdgesMut<'g, E: 'g, I: Iterator<Item = &'g mut Edge<E>>> {
    inner: I,
    phantom: PhantomData<E>,
}

impl<'g, E: 'g, I: Iterator<Item = &'g mut Edge<E>>> EdgesMut<'g, E, I> {
    /// Creates a new `EdgesMut` iterator with the provided `inner` iterator
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<'g, E: 'g, I: Iterator<Item = &'g mut Edge<E>>> Iterator for EdgesMut<'g, E, I> {
    type Item = EdgeMut<'g, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|edge| edge.as_mut_edge())
    }
}
