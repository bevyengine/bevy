use std::marker::PhantomData;

use crate::{graphs::keys::NodeIdx, utils::wrapped_indices_iterator::WrappedIndicesIterator};

/// An iterator which filters out every non-isolated node of a sub-iterator
pub struct Isolated<T, I: Iterator<Item = ((NodeIdx, T), usize)>> {
    inner: InnerIsolated<T, I>,
    phantom: PhantomData<T>,
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> Isolated<T, I> {
    /// Creates a new `Isolated` iterator
    pub fn new(inner: I) -> Self {
        Self {
            inner: InnerIsolated {
                inner,
                phantom: PhantomData,
            },
            phantom: PhantomData,
        }
    }
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> WrappedIndicesIterator<NodeIdx>
    for Isolated<T, I>
{
    type IndicesIter = std::iter::Map<InnerIsolated<T, I>, fn(((NodeIdx, T), usize)) -> NodeIdx>;

    #[inline]
    fn into_indices(self) -> Self::IndicesIter {
        self.inner.map(|((index, _), _)| index)
    }
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> Iterator for Isolated<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|((_, node), _)| node)
    }
}

/// An iterator which filters out every non-isolated node of a sub-iterator
pub struct InnerIsolated<T, I: Iterator<Item = ((NodeIdx, T), usize)>> {
    inner: I,
    phantom: PhantomData<T>,
}

impl<T, I: Iterator<Item = ((NodeIdx, T), usize)>> Iterator for InnerIsolated<T, I> {
    type Item = ((NodeIdx, T), usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find(|(_, in_out_degree)| *in_out_degree == 0)
    }
}
