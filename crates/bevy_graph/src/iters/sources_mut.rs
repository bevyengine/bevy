use std::marker::PhantomData;

use crate::graphs::{keys::NodeIdx, Graph};

/// An iterator which iterates every source node of a graph
pub struct SourcesMut<'g, N: 'g, E, G: Graph<N, E>, I: Iterator<Item = (NodeIdx, &'g mut N)>> {
    graph: &'g mut G,
    inner: I,
    phantom: PhantomData<(N, E)>,
}

impl<'g, N: 'g, E, G: Graph<N, E>, I: Iterator<Item = (NodeIdx, &'g mut N)>>
    SourcesMut<'g, N, E, G, I>
{
    /// An iterator which iterates every source node of a graph
    pub fn new(inner: I, graph: &'g mut G) -> Self {
        Self {
            inner,
            graph,
            phantom: PhantomData,
        }
    }
}

impl<'g, N: 'g, E, G: Graph<N, E>, I: Iterator<Item = (NodeIdx, &'g mut N)>> Iterator
    for SourcesMut<'g, N, E, G, I>
{
    type Item = &'g mut N;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .find(|(index, _)| self.graph.in_degree(*index) == 0)
            .map(|(_, n)| n)
    }
}
