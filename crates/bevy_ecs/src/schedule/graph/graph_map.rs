//! `Graph<DIRECTED>` is a graph datastructure where node values are mapping
//! keys.
//! Based on the `GraphMap` datastructure from [`petgraph`].
//!
//! [`petgraph`]: https://docs.rs/petgraph/0.6.5/petgraph/

use alloc::vec::Vec;
use core::{
    fmt::{self, Debug},
    hash::{BuildHasher, Hash},
};

use bevy_platform::{collections::HashSet, hash::FixedHasher};
use indexmap::IndexMap;
use smallvec::SmallVec;

use Direction::{Incoming, Outgoing};

/// Types that can be used as node identifiers in a [`DiGraph`]/[`UnGraph`].
///
/// [`DiGraph`]: crate::schedule::graph::DiGraph
/// [`UnGraph`]: crate::schedule::graph::UnGraph
pub trait GraphNodeId: Copy + Eq + Hash + Ord + Debug {
    /// The type that packs and unpacks this [`GraphNodeId`] with a [`Direction`].
    /// This is used to save space in the graph's adjacency list.
    type Adjacent: Copy + Debug + From<(Self, Direction)> + Into<(Self, Direction)>;
    /// The type that packs and unpacks this [`GraphNodeId`] with another
    /// [`GraphNodeId`]. This is used to save space in the graph's edge list.
    type Edge: Copy + Eq + Hash + Debug + From<(Self, Self)> + Into<(Self, Self)>;

    /// Name of the kind of this node id.
    ///
    /// For structs, this should return a human-readable name of the struct.
    /// For enums, this should return a human-readable name of the enum variant.
    fn kind(&self) -> &'static str;
}

/// A `Graph` with undirected edges of some [`GraphNodeId`] `N`.
///
/// For example, an edge between *1* and *2* is equivalent to an edge between
/// *2* and *1*.
pub type UnGraph<N, S = FixedHasher> = Graph<false, N, S>;

/// A `Graph` with directed edges of some [`GraphNodeId`] `N`.
///
/// For example, an edge from *1* to *2* is distinct from an edge from *2* to
/// *1*.
pub type DiGraph<N, S = FixedHasher> = Graph<true, N, S>;

/// `Graph<DIRECTED>` is a graph datastructure using an associative array
/// of its node weights of some [`GraphNodeId`].
///
/// It uses a combined adjacency list and sparse adjacency matrix
/// representation, using **O(|N| + |E|)** space, and allows testing for edge
/// existence in constant time.
///
/// `Graph` is parameterized over:
///
/// - Constant generic bool `DIRECTED` determines whether the graph edges are directed or
///   undirected.
/// - The `GraphNodeId` type `N`, which is used as the node weight.
/// - The `BuildHasher` `S`.
///
/// You can use the type aliases `UnGraph` and `DiGraph` for convenience.
///
/// `Graph` does not allow parallel edges, but self loops are allowed.
#[derive(Clone)]
pub struct Graph<const DIRECTED: bool, N: GraphNodeId, S = FixedHasher>
where
    S: BuildHasher,
{
    nodes: IndexMap<N, Vec<N::Adjacent>, S>,
    edges: HashSet<N::Edge, S>,
}

impl<const DIRECTED: bool, N: GraphNodeId, S: BuildHasher> Debug for Graph<DIRECTED, N, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.nodes.fmt(f)
    }
}

impl<const DIRECTED: bool, N: GraphNodeId, S: BuildHasher> Graph<DIRECTED, N, S> {
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
    fn edge_key(a: N, b: N) -> N::Edge {
        let (a, b) = if DIRECTED || a <= b { (a, b) } else { (b, a) };

        N::Edge::from((a, b))
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Add node `n` to the graph.
    pub fn add_node(&mut self, n: N) {
        self.nodes.entry(n).or_default();
    }

    /// Remove a node `n` from the graph.
    ///
    /// Computes in **O(N)** time, due to the removal of edges with other nodes.
    pub fn remove_node(&mut self, n: N) {
        let Some(links) = self.nodes.swap_remove(&n) else {
            return;
        };

        let links = links.into_iter().map(N::Adjacent::into);

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
    pub fn contains_node(&self, n: N) -> bool {
        self.nodes.contains_key(&n)
    }

    /// Add an edge connecting `a` and `b` to the graph.
    /// For a directed graph, the edge is directed from `a` to `b`.
    ///
    /// Inserts nodes `a` and/or `b` if they aren't already part of the graph.
    pub fn add_edge(&mut self, a: N, b: N) {
        if self.edges.insert(Self::edge_key(a, b)) {
            // insert in the adjacency list if it's a new edge
            self.nodes
                .entry(a)
                .or_insert_with(|| Vec::with_capacity(1))
                .push(N::Adjacent::from((b, Outgoing)));
            if a != b {
                // self loops don't have the Incoming entry
                self.nodes
                    .entry(b)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(N::Adjacent::from((a, Incoming)));
            }
        }
    }

    /// Remove edge relation from a to b.
    ///
    /// Return `true` if it did exist.
    fn remove_single_edge(&mut self, a: N, b: N, dir: Direction) -> bool {
        let Some(sus) = self.nodes.get_mut(&a) else {
            return false;
        };

        let Some(index) = sus
            .iter()
            .copied()
            .map(N::Adjacent::into)
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
    pub fn remove_edge(&mut self, a: N, b: N) -> bool {
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
    pub fn contains_edge(&self, a: N, b: N) -> bool {
        self.edges.contains(&Self::edge_key(a, b))
    }

    /// Return an iterator over the nodes of the graph.
    pub fn nodes(&self) -> impl DoubleEndedIterator<Item = N> + ExactSizeIterator<Item = N> + '_ {
        self.nodes.keys().copied()
    }

    /// Return an iterator of all nodes with an edge starting from `a`.
    pub fn neighbors(&self, a: N) -> impl DoubleEndedIterator<Item = N> + '_ {
        let iter = match self.nodes.get(&a) {
            Some(neigh) => neigh.iter(),
            None => [].iter(),
        };

        iter.copied()
            .map(N::Adjacent::into)
            .filter_map(|(n, dir)| (!DIRECTED || dir == Outgoing).then_some(n))
    }

    /// Return an iterator of all neighbors that have an edge between them and
    /// `a`, in the specified direction.
    /// If the graph's edges are undirected, this is equivalent to *.neighbors(a)*.
    pub fn neighbors_directed(
        &self,
        a: N,
        dir: Direction,
    ) -> impl DoubleEndedIterator<Item = N> + '_ {
        let iter = match self.nodes.get(&a) {
            Some(neigh) => neigh.iter(),
            None => [].iter(),
        };

        iter.copied()
            .map(N::Adjacent::into)
            .filter_map(move |(n, d)| (!DIRECTED || d == dir || n == a).then_some(n))
    }

    /// Return an iterator of target nodes with an edge starting from `a`,
    /// paired with their respective edge weights.
    pub fn edges(&self, a: N) -> impl DoubleEndedIterator<Item = (N, N)> + '_ {
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
        a: N,
        dir: Direction,
    ) -> impl DoubleEndedIterator<Item = (N, N)> + '_ {
        self.neighbors_directed(a, dir).map(move |b| {
            let (a, b) = if dir == Incoming { (b, a) } else { (a, b) };

            match self.edges.get(&Self::edge_key(a, b)) {
                None => unreachable!(),
                Some(_) => (a, b),
            }
        })
    }

    /// Return an iterator over all edges of the graph with their weight in arbitrary order.
    pub fn all_edges(&self) -> impl ExactSizeIterator<Item = (N, N)> + '_ {
        self.edges.iter().copied().map(N::Edge::into)
    }

    pub(crate) fn to_index(&self, ix: N) -> usize {
        self.nodes.get_index_of(&ix).unwrap()
    }

    /// Converts from one [`GraphNodeId`] type to another. If the conversion fails,
    /// it returns the error from the target type's [`TryFrom`] implementation.
    ///
    /// Nodes must uniquely convert from `N` to `T` (i.e. no two `N` can convert
    /// to the same `T`).
    ///
    /// # Errors
    ///
    /// If the conversion fails, it returns an error of type `T::Error`.
    pub fn try_into<T: GraphNodeId + TryFrom<N>>(self) -> Result<Graph<DIRECTED, T, S>, T::Error>
    where
        S: Default,
    {
        // Converts the node key and every adjacency list entry from `N` to `T`.
        fn try_convert_node<N: GraphNodeId, T: GraphNodeId + TryFrom<N>>(
            (key, adj): (N, Vec<N::Adjacent>),
        ) -> Result<(T, Vec<T::Adjacent>), T::Error> {
            let key = key.try_into()?;
            let adj = adj
                .into_iter()
                .map(|node| {
                    let (id, dir) = node.into();
                    Ok(T::Adjacent::from((id.try_into()?, dir)))
                })
                .collect::<Result<_, T::Error>>()?;
            Ok((key, adj))
        }
        // Unpacks the edge pair, converts the nodes from `N` to `T`, and repacks them.
        fn try_convert_edge<N: GraphNodeId, T: GraphNodeId + TryFrom<N>>(
            edge: N::Edge,
        ) -> Result<T::Edge, T::Error> {
            let (a, b) = edge.into();
            Ok(T::Edge::from((a.try_into()?, b.try_into()?)))
        }

        let nodes = self
            .nodes
            .into_iter()
            .map(try_convert_node::<N, T>)
            .collect::<Result<_, T::Error>>()?;
        let edges = self
            .edges
            .into_iter()
            .map(try_convert_edge::<N, T>)
            .collect::<Result<_, T::Error>>()?;
        Ok(Graph { nodes, edges })
    }
}

/// Create a new empty `Graph`.
impl<const DIRECTED: bool, N, S> Default for Graph<DIRECTED, N, S>
where
    N: GraphNodeId,
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self::with_capacity(0, 0)
    }
}

impl<N: GraphNodeId, S: BuildHasher> DiGraph<N, S> {
    /// Iterate over all *Strongly Connected Components* in this graph.
    pub(crate) fn iter_sccs(&self) -> impl Iterator<Item = SmallVec<[N; 4]>> + '_ {
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
    use crate::schedule::{NodeId, SystemKey};

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
        let mut graph = DiGraph::<NodeId>::default();

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
        let mut graph = DiGraph::<NodeId>::default();

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
