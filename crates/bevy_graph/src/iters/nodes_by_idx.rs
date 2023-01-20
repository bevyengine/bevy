use std::marker::PhantomData;

use crate::graphs::{keys::NodeIdx, Graph};

/// An iterator which converts `NodeIdx` to a `&'g N` of the graph
pub struct NodesByIdx<'g, N: 'g, E, G: Graph<N, E>, I: Iterator<Item = NodeIdx>> {
    graph: &'g G,
    inner: I,
    phantom: PhantomData<(N, E)>,
}

impl<'g, N: 'g, E, G: Graph<N, E>, I: Iterator<Item = NodeIdx>> NodesByIdx<'g, N, E, G, I> {
    /// Creates a new `NodesByIdx` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, graph: &'g G) -> Self {
        Self {
            graph,
            inner,
            phantom: PhantomData,
        }
    }

    /// Returns the inner iterator which yields `NodeIdx`
    #[inline]
    pub fn into_indices_iter(self) -> I {
        self.inner
    }
}

impl<'g, N: 'g, E, G: Graph<N, E>, I: Iterator<Item = NodeIdx>> Iterator
    for NodesByIdx<'g, N, E, G, I>
{
    type Item = &'g N;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            self.graph.get_node(index)
        } else {
            None
        }
    }
}
