use crate::graphs::keys::NodeIdx;

/// Iterator which guarantees that loops will only come once by skipping itself in adjacencies
pub struct LoopSafetyIter<'g, V: 'g, I: Iterator<Item = (&'g NodeIdx, V)>> {
    index_to_skip: NodeIdx,
    inner: I,
}

impl<'g, V: 'g, I: Iterator<Item = (&'g NodeIdx, V)>> LoopSafetyIter<'g, V, I> {
    /// Creates a new `LoopSafetyIter` iterator with the provided `inner` iterator and the `NodeIdx` it should skip
    pub fn new(inner: I, index_to_skip: NodeIdx) -> Self {
        Self {
            inner,
            index_to_skip,
        }
    }
}

impl<'g, V: 'g, I: Iterator<Item = (&'g NodeIdx, V)>> Iterator for LoopSafetyIter<'g, V, I> {
    type Item = (&'g NodeIdx, V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i) = self.inner.next() {
            if i.0 == &self.index_to_skip {
                self.next()
            } else {
                Some(i)
            }
        } else {
            None
        }
    }
}
