use std::borrow::Borrow;

use hashbrown::HashSet;
use slotmap::HopSlotMap;

use crate::graphs::{edge::Edge, keys::EdgeIdx, Graph};

/// Iterator over `(&)EdgeIdx` which guarantees that loops will only come once
pub struct LoopSafetyIter<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> {
    edges: &'g HopSlotMap<EdgeIdx, Edge<E>>,
    loops: HashSet<EdgeIdx>,
    inner: I,
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> LoopSafetyIter<'g, E, B, I> {
    /// Creates a new `LoopSafetyIter` iterator over a graph with the provided `inner` iterator
    pub fn from_graph<N>(inner: I, graph: &'g mut impl Graph<N, E>) -> Self {
        Self {
            edges: unsafe { graph.edges_raw() },
            loops: HashSet::new(),
            inner,
        }
    }

    /// Creates a new `LoopSafetyIter` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, edges: &'g HopSlotMap<EdgeIdx, Edge<E>>) -> Self {
        Self {
            edges,
            loops: HashSet::new(),
            inner,
        }
    }
}

impl<'g, E: 'g, B: Borrow<EdgeIdx>, I: Iterator<Item = B>> Iterator
    for LoopSafetyIter<'g, E, B, I>
{
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            let edge_idx = *index.borrow();
            let Edge(src, dst, _) = unsafe { self.edges.get_unchecked(edge_idx) };
            if src == dst {
                if self.loops.contains(&edge_idx) {
                    return self.next();
                }
                self.loops.insert(edge_idx);
            }
            Some(index)
        } else {
            None
        }
    }
}
