use std::marker::PhantomData;

use crate::graphs::keys::NodeIdx;

/// An iterator which iterates every source `NodeIdx` of a graph
pub struct Sources<N, I: Iterator<Item = ((NodeIdx, N), usize)>> {
    inner: I,
    phantom: PhantomData<N>,
}

impl<N, I: Iterator<Item = ((NodeIdx, N), usize)>> Sources<N, I> {
    /// An iterator which iterates every source node of a graph
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<N, I: Iterator<Item = ((NodeIdx, N), usize)>> Iterator for Sources<N, I> {
    type Item = N;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .find(|(_, in_degree)| *in_degree == 0)
            .map(|((_, node), _)| node)
    }
}
