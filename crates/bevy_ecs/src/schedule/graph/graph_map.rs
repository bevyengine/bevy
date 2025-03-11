//! `Graph<DIRECTED>` is a graph datastructure where node values are mapping
//! keys.
//! Based on the `GraphMap` datastructure from [`petgraph`].
//!
//! [`petgraph`]: https://docs.rs/petgraph/0.6.5/petgraph/

use alloc::vec::Vec;
use bevy_platform_support::{collections::HashSet, hash::FixedHasher};
use core::{
    fmt,
    hash::{BuildHasher, Hash},
};
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::schedule::graph::{DirectedGraphNodeId, GraphNodeId, GraphNodeIdPair};

use Direction::{Incoming, Outgoing};

/// A `Graph` with undirected edges.
///
/// For example, an edge between *1* and *2* is equivalent to an edge between
/// *2* and *1*.
pub type UnGraph<Id, S = FixedHasher> = Graph<false, Id, S>;

/// A `Graph` with directed edges.
///
/// For example, an edge from *1* to *2* is distinct from an edge from *2* to
/// *1*.
pub type DiGraph<Id, S = FixedHasher> = Graph<true, Id, S>;

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
pub struct Graph<const DIRECTED: bool, Id, S = FixedHasher>
where
    Id: GraphNodeId,
    S: BuildHasher,
{
    nodes: IndexMap<Id, Vec<Id::Directed>, S>,
    edges: HashSet<Id::Pair, S>,
}

impl<const DIRECTED: bool, Id, S> fmt::Debug for Graph<DIRECTED, Id, S>
where
    Id: GraphNodeId,
    S: BuildHasher,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.nodes.fmt(f)
    }
}

impl<const DIRECTED: bool, Id, S> Graph<DIRECTED, Id, S>
where
    Id: GraphNodeId,
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
    fn edge_key(a: Id, b: Id) -> Id::Pair {
        let (a, b) = if DIRECTED || a <= b { (a, b) } else { (b, a) };

        Id::Pair::pack(a, b)
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Add node `n` to the graph.
    pub fn add_node(&mut self, n: Id) {
        self.nodes.entry(n).or_default();
    }

    /// Remove a node `n` from the graph.
    ///
    /// Computes in **O(N)** time, due to the removal of edges with other nodes.
    pub fn remove_node(&mut self, n: Id) {
        let Some(links) = self.nodes.swap_remove(&n) else {
            return;
        };

        let links = links.into_iter().map(Id::Directed::unpack);

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
    pub fn contains_node(&self, n: Id) -> bool {
        self.nodes.contains_key(&n)
    }

    /// Add an edge connecting `a` and `b` to the graph.
    /// For a directed graph, the edge is directed from `a` to `b`.
    ///
    /// Inserts nodes `a` and/or `b` if they aren't already part of the graph.
    pub fn add_edge(&mut self, a: Id, b: Id) {
        if self.edges.insert(Self::edge_key(a, b)) {
            // insert in the adjacency list if it's a new edge
            self.nodes
                .entry(a)
                .or_insert_with(|| Vec::with_capacity(1))
                .push(Id::Directed::pack(b, Outgoing));
            if a != b {
                // self loops don't have the Incoming entry
                self.nodes
                    .entry(b)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(Id::Directed::pack(a, Incoming));
            }
        }
    }

    /// Remove edge relation from a to b
    ///
    /// Return `true` if it did exist.
    fn remove_single_edge(&mut self, a: Id, b: Id, dir: Direction) -> bool {
        let Some(sus) = self.nodes.get_mut(&a) else {
            return false;
        };

        let Some(index) = sus
            .iter()
            .copied()
            .map(Id::Directed::unpack)
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
    pub fn remove_edge(&mut self, a: Id, b: Id) -> bool {
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
    pub fn contains_edge(&self, a: Id, b: Id) -> bool {
        self.edges.contains(&Self::edge_key(a, b))
    }

    /// Return an iterator over the nodes of the graph.
    pub fn nodes(&self) -> impl DoubleEndedIterator<Item = Id> + ExactSizeIterator<Item = Id> + '_ {
        self.nodes.keys().copied()
    }

    /// Return an iterator of all nodes with an edge starting from `a`.
    pub fn neighbors(&self, a: Id) -> impl DoubleEndedIterator<Item = Id> + '_ {
        let iter = match self.nodes.get(&a) {
            Some(neigh) => neigh.iter(),
            None => [].iter(),
        };

        iter.copied()
            .map(Id::Directed::unpack)
            .filter_map(|(n, dir)| (!DIRECTED || dir == Outgoing).then_some(n))
    }

    /// Return an iterator of all neighbors that have an edge between them and
    /// `a`, in the specified direction.
    /// If the graph's edges are undirected, this is equivalent to *.neighbors(a)*.
    pub fn neighbors_directed(
        &self,
        a: Id,
        dir: Direction,
    ) -> impl DoubleEndedIterator<Item = Id> + '_ {
        let iter = match self.nodes.get(&a) {
            Some(neigh) => neigh.iter(),
            None => [].iter(),
        };

        iter.copied()
            .map(Id::Directed::unpack)
            .filter_map(move |(n, d)| (!DIRECTED || d == dir || n == a).then_some(n))
    }

    /// Return an iterator of target nodes with an edge starting from `a`,
    /// paired with their respective edge weights.
    pub fn edges(&self, a: Id) -> impl DoubleEndedIterator<Item = (Id, Id)> + '_ {
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
        a: Id,
        dir: Direction,
    ) -> impl DoubleEndedIterator<Item = (Id, Id)> + '_ {
        self.neighbors_directed(a, dir).map(move |b| {
            let (a, b) = if dir == Incoming { (b, a) } else { (a, b) };

            match self.edges.get(&Self::edge_key(a, b)) {
                None => unreachable!(),
                Some(_) => (a, b),
            }
        })
    }

    /// Return an iterator over all edges of the graph with their weight in arbitrary order.
    pub fn all_edges(&self) -> impl ExactSizeIterator<Item = (Id, Id)> + '_ {
        self.edges.iter().copied().map(Id::Pair::unpack)
    }

    pub(crate) fn to_index(&self, ix: Id) -> usize {
        self.nodes.get_index_of(&ix).unwrap()
    }
}

/// Create a new empty `Graph`.
impl<const DIRECTED: bool, Id, S> Default for Graph<DIRECTED, Id, S>
where
    Id: GraphNodeId,
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self::with_capacity(0, 0)
    }
}

impl<Id, S> DiGraph<Id, S>
where
    Id: GraphNodeId,
    S: BuildHasher,
{
    /// Iterate over all *Strongly Connected Components* in this graph.
    pub(crate) fn iter_sccs(&self) -> impl Iterator<Item = SmallVec<[Id; 4]>> + '_ {
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

#[cfg(test)]
mod tests {
    use crate::schedule::NodeId;

    use super::*;
    use alloc::vec;

    /// The `Graph` type _must_ preserve the order that nodes are inserted in if
    /// no removals occur. Removals are permitted to swap the latest node into the
    /// location of the removed node.
    #[test]
    fn node_order_preservation() {
        use NodeId::System;

        let mut graph = DiGraph::<NodeId>::default();

        graph.add_node(System(1));
        graph.add_node(System(2));
        graph.add_node(System(3));
        graph.add_node(System(4));

        assert_eq!(
            graph.nodes().collect::<Vec<_>>(),
            vec![System(1), System(2), System(3), System(4)]
        );

        graph.remove_node(System(1));

        assert_eq!(
            graph.nodes().collect::<Vec<_>>(),
            vec![System(4), System(2), System(3)]
        );

        graph.remove_node(System(4));

        assert_eq!(
            graph.nodes().collect::<Vec<_>>(),
            vec![System(3), System(2)]
        );

        graph.remove_node(System(2));

        assert_eq!(graph.nodes().collect::<Vec<_>>(), vec![System(3)]);

        graph.remove_node(System(3));

        assert_eq!(graph.nodes().collect::<Vec<_>>(), vec![]);
    }

    /// Nodes that have bidirectional edges (or any edge in the case of undirected graphs) are
    /// considered strongly connected. A strongly connected component is a collection of
    /// nodes where there exists a path from any node to any other node in the collection.
    #[test]
    fn strongly_connected_components() {
        use NodeId::System;

        let mut graph = DiGraph::<NodeId>::default();

        graph.add_edge(System(1), System(2));
        graph.add_edge(System(2), System(1));

        graph.add_edge(System(2), System(3));
        graph.add_edge(System(3), System(2));

        graph.add_edge(System(4), System(5));
        graph.add_edge(System(5), System(4));

        graph.add_edge(System(6), System(2));

        let sccs = graph
            .iter_sccs()
            .map(|scc| scc.to_vec())
            .collect::<Vec<_>>();

        assert_eq!(
            sccs,
            vec![
                vec![System(3), System(2), System(1)],
                vec![System(5), System(4)],
                vec![System(6)]
            ]
        );
    }
}
