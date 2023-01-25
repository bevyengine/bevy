use hashbrown::HashMap;
use slotmap::SecondaryMap;

use crate::graphs::{adjacency_storage::AdjacencyStorage, keys::NodeIdx};

/// An iterator which zips the `degree` of a `NodeIdx` with it
pub struct ZipDegree<'g, S, N, I: Iterator<Item = (NodeIdx, N)>, const DIRECTED: bool> {
    adjacencies: &'g SecondaryMap<NodeIdx, AdjacencyStorage<S>>,
    inner: I,
}

impl<'g, S, N, I: Iterator<Item = (NodeIdx, N)>, const DIRECTED: bool>
    ZipDegree<'g, S, N, I, DIRECTED>
{
    /// Creates a new `ZipDegree` iterator with the provided `inner` iterator
    pub fn new(inner: I, adjacencies: &'g SecondaryMap<NodeIdx, AdjacencyStorage<S>>) -> Self {
        Self { adjacencies, inner }
    }
}

impl<'g, T, N, I: Iterator<Item = (NodeIdx, N)>, const DIRECTED: bool> Iterator
    for ZipDegree<'g, Vec<T>, N, I, DIRECTED>
{
    type Item = ((NodeIdx, N), usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(index, node)| {
            let degree = if DIRECTED {
                self.adjacencies[index].incoming().len() + self.adjacencies[index].outgoing().len()
            } else {
                self.adjacencies[index].incoming().len()
            };
            ((index, node), degree)
        })
    }
}

impl<'g, K, V, N, I: Iterator<Item = (NodeIdx, N)>, const DIRECTED: bool> Iterator
    for ZipDegree<'g, HashMap<K, V>, N, I, DIRECTED>
{
    type Item = ((NodeIdx, N), usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(index, node)| {
            let degree = if DIRECTED {
                self.adjacencies[index].incoming().len() + self.adjacencies[index].outgoing().len()
            } else {
                self.adjacencies[index].incoming().len()
            };
            ((index, node), degree)
        })
    }
}
