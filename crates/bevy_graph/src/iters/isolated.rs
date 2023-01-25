use std::marker::PhantomData;

use crate::{graphs::keys::NodeIdx, utils::wrapped_iterator::WrappedIterator};

/// An iterator which filters out every non-isolated node of a sub-iterator
pub struct Isolated<T, I: Iterator<Item = ((NodeIdx, T), usize)>> {
    inner: I,
    phantom: PhantomData<T>,
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> Isolated<T, I> {
    /// Creates a new `Isolated` iterator
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> WrappedIterator<NodeIdx> for Isolated<T, I> {
    type Inner = std::iter::Map<I, fn(((NodeIdx, T), usize)) -> NodeIdx>;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        self.inner.map(|((index, _), _)| index)
    }
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> Iterator for Isolated<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .find(|(_, in_out_degree)| *in_out_degree == 0)
            .map(|((_, node), _)| node)
    }
}
