use std::marker::PhantomData;

use crate::{graphs::keys::NodeIdx, utils::wrapped_iterator::WrappedIterator};

/// An iterator which iterates every source / sink node of a graph
pub struct SourcesSinks<T, I: Iterator<Item = ((NodeIdx, T), usize)>> {
    inner: I,
    phantom: PhantomData<T>,
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> SourcesSinks<T, I> {
    /// An iterator which iterates every source / sink node of a graph
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> WrappedIterator<Self, T, I>
    for SourcesSinks<T, I>
{
    #[inline]
    fn into_inner(self) -> I {
        self.inner
    }
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> Iterator for SourcesSinks<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .find(|(_, in_out_degree)| *in_out_degree == 0)
            .map(|((_, node), _)| node)
    }
}
