use crate::graphs::keys::NodeIdx;

/// An iterator which iterates every source `NodeIdx` of a graph
pub struct Sources<I: Iterator<Item = (NodeIdx, usize)>> {
    inner: I,
}

impl<I: Iterator<Item = (NodeIdx, usize)>> Sources<I> {
    /// An iterator which iterates every sink node of a graph
    pub fn new(inner: I) -> Self {
        Self { inner }
    }
}

impl<I: Iterator<Item = (NodeIdx, usize)>> Iterator for Sources<I> {
    type Item = NodeIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .find(|(_, in_degree)| *in_degree == 0)
            .map(|(index, _)| index)
    }
}
