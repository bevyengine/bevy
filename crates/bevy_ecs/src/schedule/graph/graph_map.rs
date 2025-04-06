//! `Graph<DIRECTED>` is a graph datastructure where node values are mapping
//! keys.
//! Based on the `GraphMap` datastructure from [`petgraph`].
//!
//! [`petgraph`]: https://docs.rs/petgraph/0.6.5/petgraph/

use bevy_platform_support::hash::FixedHasher;
use petgraph::prelude::{DiGraphMap, UnGraphMap};

use super::NodeId;

pub use petgraph::Direction;

/// A `Graph` with undirected edges.
///
/// For example, an edge between *1* and *2* is equivalent to an edge between
/// *2* and *1*.
pub type UnGraph<S = FixedHasher> = UnGraphMap<NodeId, (), S>;

/// A `Graph` with directed edges.
///
/// For example, an edge from *1* to *2* is distinct from an edge from *2* to
/// *1*.
pub type DiGraph<S = FixedHasher> = DiGraphMap<NodeId, (), S>;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};
    use petgraph::algo::TarjanScc;

    /// The `Graph` type _must_ preserve the order that nodes are inserted in if
    /// no removals occur. Removals are permitted to swap the latest node into the
    /// location of the removed node.
    #[test]
    fn node_order_preservation() {
        use NodeId::System;

        let mut graph = <DiGraph>::default();

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

        let mut graph = <DiGraph>::default();

        graph.add_edge(System(1), System(2), ());
        graph.add_edge(System(2), System(1), ());

        graph.add_edge(System(2), System(3), ());
        graph.add_edge(System(3), System(2), ());

        graph.add_edge(System(4), System(5), ());
        graph.add_edge(System(5), System(4), ());

        graph.add_edge(System(6), System(2), ());

        let mut sccs = Vec::new();
        let mut tarjan = TarjanScc::<NodeId>::new();
        tarjan.run(&graph, |scc| {
            sccs.push(scc.to_vec());
        });

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
