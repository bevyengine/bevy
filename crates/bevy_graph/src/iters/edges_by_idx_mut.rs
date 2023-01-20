use std::marker::PhantomData;

use crate::graphs::{edge::EdgeMut, keys::EdgeIdx, Graph};

/// An iterator which converts `&mut EdgeIdx` to a `EdgeMut<E>` of the graph
pub struct EdgesByIdxMut<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = &'g mut EdgeIdx>> {
    graph: &'g G,
    inner: I,
    phantom: PhantomData<(N, E)>,
}

impl<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = &'g mut EdgeIdx>>
    EdgesByIdxMut<'g, N, E, G, I>
{
    /// Creates a new `EdgesByIdxMut` iterator over a graph with the provided `inner` iterator
    pub fn new(inner: I, graph: &'g G) -> Self {
        Self {
            graph,
            inner,
            phantom: PhantomData,
        }
    }
}

impl<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = &'g mut EdgeIdx>> Iterator
    for EdgesByIdxMut<'g, N, E, G, I>
{
    type Item = EdgeMut<'g, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.inner.next() {
            self.graph.get_edge_mut(*index)
        } else {
            None
        }
    }
}
