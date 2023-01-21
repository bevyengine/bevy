use hashbrown::HashMap;
use slotmap::SecondaryMap;

use crate::graphs::{adjacency_storage::AdjacencyStorage, keys::NodeIdx};

/// An iterator which zips the `in_degree` of a `NodeIdx` with it
pub struct ZipInDegree<'g, S, N, I: Iterator<Item = (NodeIdx, N)>> {
    adjacencies: &'g SecondaryMap<NodeIdx, AdjacencyStorage<S>>,
    inner: I,
}

impl<'g, S, N, I: Iterator<Item = (NodeIdx, N)>> ZipInDegree<'g, S, N, I> {
    /// Creates a new `ZipInDegree` iterator with the provided `inner` iterator
    pub fn new(inner: I, adjacencies: &'g SecondaryMap<NodeIdx, AdjacencyStorage<S>>) -> Self {
        Self { adjacencies, inner }
    }
}

impl<'g, T, N, I: Iterator<Item = (NodeIdx, N)>> Iterator for ZipInDegree<'g, Vec<T>, N, I> {
    type Item = ((NodeIdx, N), usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(index, node)| {
            let in_degree = self.adjacencies[index].incoming().len();
            ((index, node), in_degree)
        })
    }
}

impl<'g, K, V, N, I: Iterator<Item = (NodeIdx, N)>> Iterator
    for ZipInDegree<'g, HashMap<K, V>, N, I>
{
    type Item = ((NodeIdx, N), usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(index, node)| {
            let in_degree = self.adjacencies[index].incoming().len();
            ((index, node), in_degree)
        })
    }
}
