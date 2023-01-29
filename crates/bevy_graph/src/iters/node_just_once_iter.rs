use std::borrow::Borrow;

use hashbrown::HashSet;

use crate::graphs::keys::NodeIdx;

/// Iterator over `(&)NodeIdx` which guarantees that a value will only come once
pub struct NodeJustOnceIter<B: Borrow<NodeIdx>, I: Iterator<Item = B>> {
    duplicates: HashSet<NodeIdx>,
    inner: I,
}

impl<B: Borrow<NodeIdx>, I: Iterator<Item = B>> NodeJustOnceIter<B, I> {
    /// Creates a new `NodeJustOnceIter` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I) -> Self {
        Self {
            duplicates: HashSet::new(),
            inner,
        }
    }
}

impl<B: Borrow<NodeIdx>, I: Iterator<Item = B>> Iterator for NodeJustOnceIter<B, I> {
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            let node_idx = *index.borrow();
            if self.duplicates.contains(&node_idx) {
                return self.next();
            } else {
                self.duplicates.insert(node_idx);
            }
            Some(index)
        } else {
            None
        }
    }
}
