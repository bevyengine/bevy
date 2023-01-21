use std::marker::PhantomData;

use crate::graphs::keys::NodeIdx;

/// An iterator which iterates every source / sink node of a graph
pub struct SourcesSinks<N, I: Iterator<Item = ((NodeIdx, N), usize)>> {
    inner: I,
    phantom: PhantomData<N>,
}

impl<N, I: Iterator<Item = ((NodeIdx, N), usize)>> SourcesSinks<N, I> {
    /// An iterator which iterates every source / sink node of a graph
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<N, I: Iterator<Item = ((NodeIdx, N), usize)>> Iterator for SourcesSinks<N, I> {
    type Item = N;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .find(|(_, in_out_degree)| *in_out_degree == 0)
            .map(|((_, node), _)| node)
    }
}
