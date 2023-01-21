use hashbrown::HashMap;
use slotmap::SecondaryMap;

use crate::graphs::{adjacency_storage::AdjacencyStorage, keys::NodeIdx};

/// An iterator which zips the `in_degree` of a `NodeIdx` with it
pub struct ZipInDegree<'g, S, I: Iterator<Item = NodeIdx>> {
    adjacencies: &'g SecondaryMap<NodeIdx, AdjacencyStorage<S>>,
    inner: I,
}

impl<'g, S, I: Iterator<Item = NodeIdx>> ZipInDegree<'g, S, I> {
    /// Creates a new `ZipInDegree` iterator with the provided `inner` iterator
    pub fn new(inner: I, adjacencies: &'g SecondaryMap<NodeIdx, AdjacencyStorage<S>>) -> Self {
        Self { adjacencies, inner }
    }
}

impl<'g, T, I: Iterator<Item = NodeIdx>> Iterator for ZipInDegree<'g, Vec<T>, I> {
    type Item = (NodeIdx, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|index| (index, self.adjacencies[index].incoming().len()))
    }
}

impl<'g, K, V, I: Iterator<Item = NodeIdx>> Iterator for ZipInDegree<'g, HashMap<K, V>, I> {
    type Item = (NodeIdx, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|index| (index, self.adjacencies[index].incoming().len()))
    }
}
