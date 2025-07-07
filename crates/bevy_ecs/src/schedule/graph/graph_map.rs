//! `Graph<DIRECTED>` is a graph datastructure where node values are mapping
//! keys.
//! Based on the `GraphMap` datastructure from [`petgraph`].
//!
//! [`petgraph`]: https://docs.rs/petgraph/0.6.5/petgraph/

use alloc::vec::Vec;
use bevy_platform::{collections::HashSet, hash::FixedHasher};
use core::{
    fmt,
    hash::{BuildHasher, Hash},
};
use indexmap::IndexMap;
use slotmap::{Key, KeyData};
use smallvec::SmallVec;

use super::NodeId;

use Direction::{Incoming, Outgoing};

/// A `Graph` with undirected edges.
///
/// For example, an edge between *1* and *2* is equivalent to an edge between
/// *2* and *1*.
pub type UnGraph<S = FixedHasher> = Graph<false, S>;

/// A `Graph` with directed edges.
///
/// For example, an edge from *1* to *2* is distinct from an edge from *2* to
/// *1*.
pub type DiGraph<S = FixedHasher> = Graph<true, S>;

/// `Graph<DIRECTED>` is a graph datastructure using an associative array
/// of its node weights `NodeId`.
///
/// It uses a combined adjacency list and sparse adjacency matrix
/// representation, using **O(|N| + |E|)** space, and allows testing for edge
/// existence in constant time.
///
/// `Graph` is parameterized over:
///
/// - Constant generic bool `DIRECTED` determines whether the graph edges are directed or
///   undirected.
/// - The `BuildHasher` `S`.
///
/// You can use the type aliases `UnGraph` and `DiGraph` for convenience.
///
/// `Graph` does not allow parallel edges, but self loops are allowed.
#[derive(Clone)]
pub struct Graph<const DIRECTED: bool, S = FixedHasher>
where
    S: BuildHasher,
{
    nodes: IndexMap<NodeId, Vec<CompactNodeIdAndDirection>, S>,
    edges: HashSet<CompactNodeIdPair, S>,
}

impl<const DIRECTED: bool, S: BuildHasher> fmt::Debug for Graph<DIRECTED, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.nodes.fmt(f)
    }
}

impl<const DIRECTED: bool, S> Graph<DIRECTED, S>
where
    S: BuildHasher,
{
    /// Create a new `Graph` with estimated capacity.
    pub fn with_capacity(nodes: usize, edges: usize) -> Self
    where
        S: Default,
    {
        Self {
            nodes: IndexMap::with_capacity_and_hasher(nodes, S::default()),
            edges: HashSet::with_capacity_and_hasher(edges, S::default()),
        }
    }

    /// Use their natural order to map the node pair (a, b) to a canonical edge id.
    #[inline]
    fn edge_key(a: NodeId, b: NodeId) -> CompactNodeIdPair {
        let (a, b) = if DIRECTED || a <= b { (a, b) } else { (b, a) };

        CompactNodeIdPair::store(a, b)
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Add node `n` to the graph.
    pub fn add_node(&mut self, n: NodeId) {
        self.nodes.entry(n).or_default();
    }

    /// Remove a node `n` from the graph.
    ///
    /// Computes in **O(N)** time, due to the removal of edges with other nodes.
    pub fn remove_node(&mut self, n: NodeId) {
        let Some(links) = self.nodes.swap_remove(&n) else {
            return;
        };

        let links = links.into_iter().map(CompactNodeIdAndDirection::load);

        for (succ, dir) in links {
            let edge = if dir == Outgoing {
                Self::edge_key(n, succ)
            } else {
                Self::edge_key(succ, n)
            };
            // remove all successor links
            self.remove_single_edge(succ, n, dir.opposite());
            // Remove all edge values
            self.edges.remove(&edge);
        }
    }

    /// Return `true` if the node is contained in the graph.
    pub fn contains_node(&self, n: NodeId) -> bool {
        self.nodes.contains_key(&n)
    }

    /// Add an edge connecting `a` and `b` to the graph.
    /// For a directed graph, the edge is directed from `a` to `b`.
    ///
    /// Inserts nodes `a` and/or `b` if they aren't already part of the graph.
    pub fn add_edge(&mut self, a: NodeId, b: NodeId) {
        if self.edges.insert(Self::edge_key(a, b)) {
            // insert in the adjacency list if it's a new edge
            self.nodes
                .entry(a)
                .or_insert_with(|| Vec::with_capacity(1))
                .push(CompactNodeIdAndDirection::store(b, Outgoing));
            if a != b {
                // self loops don't have the Incoming entry
                self.nodes
                    .entry(b)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(CompactNodeIdAndDirection::store(a, Incoming));
            }
        }
    }

    /// Remove edge relation from a to b
    ///
    /// Return `true` if it did exist.
    fn remove_single_edge(&mut self, a: NodeId, b: NodeId, dir: Direction) -> bool {
        let Some(sus) = self.nodes.get_mut(&a) else {
            return false;
        };

        let Some(index) = sus
            .iter()
            .copied()
            .map(CompactNodeIdAndDirection::load)
            .position(|elt| (DIRECTED && elt == (b, dir)) || (!DIRECTED && elt.0 == b))
        else {
            return false;
        };

        sus.swap_remove(index);
        true
    }

    /// Remove edge from `a` to `b` from the graph.
    ///
    /// Return `false` if the edge didn't exist.
    pub fn remove_edge(&mut self, a: NodeId, b: NodeId) -> bool {
        let exist1 = self.remove_single_edge(a, b, Outgoing);
        let exist2 = if a != b {
            self.remove_single_edge(b, a, Incoming)
        } else {
            exist1
        };
        let weight = self.edges.remove(&Self::edge_key(a, b));
        debug_assert!(exist1 == exist2 && exist1 == weight);
        weight
    }

    /// Return `true` if the edge connecting `a` with `b` is contained in the graph.
    pub fn contains_edge(&self, a: NodeId, b: NodeId) -> bool {
        self.edges.contains(&Self::edge_key(a, b))
    }

    /// Return an iterator over the nodes of the graph.
    pub fn nodes(
        &self,
    ) -> impl DoubleEndedIterator<Item = NodeId> + ExactSizeIterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// Return an iterator of all nodes with an edge starting from `a`.
    pub fn neighbors(&self, a: NodeId) -> impl DoubleEndedIterator<Item = NodeId> + '_ {
        let iter = match self.nodes.get(&a) {
            Some(neigh) => neigh.iter(),
            None => [].iter(),
        };

        iter.copied()
            .map(CompactNodeIdAndDirection::load)
            .filter_map(|(n, dir)| (!DIRECTED || dir == Outgoing).then_some(n))
    }

    /// Return an iterator of all neighbors that have an edge between them and
    /// `a`, in the specified direction.
    /// If the graph's edges are undirected, this is equivalent to *.neighbors(a)*.
    pub fn neighbors_directed(
        &self,
        a: NodeId,
        dir: Direction,
    ) -> impl DoubleEndedIterator<Item = NodeId> + '_ {
        let iter = match self.nodes.get(&a) {
            Some(neigh) => neigh.iter(),
            None => [].iter(),
        };

        iter.copied()
            .map(CompactNodeIdAndDirection::load)
            .filter_map(move |(n, d)| (!DIRECTED || d == dir || n == a).then_some(n))
    }

    /// Return an iterator of target nodes with an edge starting from `a`,
    /// paired with their respective edge weights.
    pub fn edges(&self, a: NodeId) -> impl DoubleEndedIterator<Item = (NodeId, NodeId)> + '_ {
        self.neighbors(a)
            .map(move |b| match self.edges.get(&Self::edge_key(a, b)) {
                None => unreachable!(),
                Some(_) => (a, b),
            })
    }

    /// Return an iterator of target nodes with an edge starting from `a`,
    /// paired with their respective edge weights.
    pub fn edges_directed(
        &self,
        a: NodeId,
        dir: Direction,
    ) -> impl DoubleEndedIterator<Item = (NodeId, NodeId)> + '_ {
        self.neighbors_directed(a, dir).map(move |b| {
            let (a, b) = if dir == Incoming { (b, a) } else { (a, b) };

            match self.edges.get(&Self::edge_key(a, b)) {
                None => unreachable!(),
                Some(_) => (a, b),
            }
        })
    }

    /// Return an iterator over all edges of the graph with their weight in arbitrary order.
    pub fn all_edges(&self) -> impl ExactSizeIterator<Item = (NodeId, NodeId)> + '_ {
        self.edges.iter().copied().map(CompactNodeIdPair::load)
    }

    pub(crate) fn to_index(&self, ix: NodeId) -> usize {
        self.nodes.get_index_of(&ix).unwrap()
    }
}

/// Create a new empty `Graph`.
impl<const DIRECTED: bool, S> Default for Graph<DIRECTED, S>
where
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self::with_capacity(0, 0)
    }
}

impl<S: BuildHasher> DiGraph<S> {
    /// Iterate over all *Strongly Connected Components* in this graph.
    pub(crate) fn iter_sccs(&self) -> impl Iterator<Item = SmallVec<[NodeId; 4]>> + '_ {
        super::tarjan_scc::new_tarjan_scc(self)
    }
}

/// Edge direction.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    /// An `Outgoing` edge is an outward edge *from* the current node.
    Outgoing = 0,
    /// An `Incoming` edge is an inbound edge *to* the current node.
    Incoming = 1,
}

impl Direction {
    /// Return the opposite `Direction`.
    #[inline]
    pub fn opposite(self) -> Self {
        match self {
            Self::Outgoing => Self::Incoming,
            Self::Incoming => Self::Outgoing,
        }
    }
}

/// Compact storage of a [`NodeId`] and a [`Direction`].
#[derive(Clone, Copy)]
struct CompactNodeIdAndDirection {
    key: KeyData,
    is_system: bool,
    direction: Direction,
}

impl fmt::Debug for CompactNodeIdAndDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.load().fmt(f)
    }
}

impl CompactNodeIdAndDirection {
    fn store(node: NodeId, direction: Direction) -> Self {
        let key = match node {
            NodeId::System(key) => key.data(),
            NodeId::Set(key) => key.data(),
        };
        let is_system = node.is_system();

        Self {
            key,
            is_system,
            direction,
        }
    }

    fn load(self) -> (NodeId, Direction) {
        let Self {
            key,
            is_system,
            direction,
        } = self;

        let node = match is_system {
            true => NodeId::System(key.into()),
            false => NodeId::Set(key.into()),
        };

        (node, direction)
    }
}

/// Compact storage of a [`NodeId`] pair.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct CompactNodeIdPair {
    key_a: KeyData,
    key_b: KeyData,
    is_system_a: bool,
    is_system_b: bool,
}

impl fmt::Debug for CompactNodeIdPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.load().fmt(f)
    }
}

impl CompactNodeIdPair {
    fn store(a: NodeId, b: NodeId) -> Self {
        let key_a = match a {
            NodeId::System(index) => index.data(),
            NodeId::Set(index) => index.data(),
        };
        let is_system_a = a.is_system();

        let key_b = match b {
            NodeId::System(index) => index.data(),
            NodeId::Set(index) => index.data(),
        };
        let is_system_b = b.is_system();

        Self {
            key_a,
            key_b,
            is_system_a,
            is_system_b,
        }
    }

    fn load(self) -> (NodeId, NodeId) {
        let Self {
            key_a,
            key_b,
            is_system_a,
            is_system_b,
        } = self;

        let a = match is_system_a {
            true => NodeId::System(key_a.into()),
            false => NodeId::Set(key_a.into()),
        };

        let b = match is_system_b {
            true => NodeId::System(key_b.into()),
            false => NodeId::Set(key_b.into()),
        };

        (a, b)
    }
}

#[cfg(test)]
mod tests {
    use crate::schedule::SystemKey;

    use super::*;
    use alloc::vec;
    use slotmap::SlotMap;

    /// The `Graph` type _must_ preserve the order that nodes are inserted in if
    /// no removals occur. Removals are permitted to swap the latest node into the
    /// location of the removed node.
    #[test]
    fn node_order_preservation() {
        use NodeId::System;

        let mut slotmap = SlotMap::<SystemKey, ()>::with_key();
        let mut graph = <DiGraph>::default();

        let sys1 = slotmap.insert(());
        let sys2 = slotmap.insert(());
        let sys3 = slotmap.insert(());
        let sys4 = slotmap.insert(());

        graph.add_node(System(sys1));
        graph.add_node(System(sys2));
        graph.add_node(System(sys3));
        graph.add_node(System(sys4));

        assert_eq!(
            graph.nodes().collect::<Vec<_>>(),
            vec![System(sys1), System(sys2), System(sys3), System(sys4)]
        );

        graph.remove_node(System(sys1));

        assert_eq!(
            graph.nodes().collect::<Vec<_>>(),
            vec![System(sys4), System(sys2), System(sys3)]
        );

        graph.remove_node(System(sys4));

        assert_eq!(
            graph.nodes().collect::<Vec<_>>(),
            vec![System(sys3), System(sys2)]
        );

        graph.remove_node(System(sys2));

        assert_eq!(graph.nodes().collect::<Vec<_>>(), vec![System(sys3)]);

        graph.remove_node(System(sys3));

        assert_eq!(graph.nodes().collect::<Vec<_>>(), vec![]);
    }

    /// Nodes that have bidirectional edges (or any edge in the case of undirected graphs) are
    /// considered strongly connected. A strongly connected component is a collection of
    /// nodes where there exists a path from any node to any other node in the collection.
    #[test]
    fn strongly_connected_components() {
        use NodeId::System;

        let mut slotmap = SlotMap::<SystemKey, ()>::with_key();
        let mut graph = <DiGraph>::default();

        let sys1 = slotmap.insert(());
        let sys2 = slotmap.insert(());
        let sys3 = slotmap.insert(());
        let sys4 = slotmap.insert(());
        let sys5 = slotmap.insert(());
        let sys6 = slotmap.insert(());

        graph.add_edge(System(sys1), System(sys2));
        graph.add_edge(System(sys2), System(sys1));

        graph.add_edge(System(sys2), System(sys3));
        graph.add_edge(System(sys3), System(sys2));

        graph.add_edge(System(sys4), System(sys5));
        graph.add_edge(System(sys5), System(sys4));

        graph.add_edge(System(sys6), System(sys2));

        let sccs = graph
            .iter_sccs()
            .map(|scc| scc.to_vec())
            .collect::<Vec<_>>();

        assert_eq!(
            sccs,
            vec![
                vec![System(sys3), System(sys2), System(sys1)],
                vec![System(sys5), System(sys4)],
                vec![System(sys6)]
            ]
        );
    }
}
