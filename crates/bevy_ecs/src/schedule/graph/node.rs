use core::{fmt::Debug, hash::Hash};

use crate::schedule::graph::Direction;

/// Types that can be used as node identifiers in a [`DiGraph`]/[`UnGraph`].
///
/// [`DiGraph`]: crate::schedule::graph::DiGraph
/// [`UnGraph`]: crate::schedule::graph::UnGraph
pub trait GraphNodeId: Copy + Eq + Hash + Ord + Debug {
    /// This [`GraphNodeId`] and a [`Direction`].
    type Directed: DirectedGraphNodeId<Id = Self>;
    /// Two of these [`GraphNodeId`]s.
    type Pair: GraphNodeIdPair<Id = Self>;
}

/// Types that are a [`GraphNodeId`] with a [`Direction`].
pub trait DirectedGraphNodeId: Copy + Debug {
    /// The type of [`GraphNodeId`] a [`Direction`] is paired with.
    type Id: GraphNodeId;

    /// Packs a [`GraphNodeId`] and a [`Direction`] into a single type.
    fn new(id: Self::Id, direction: Direction) -> Self;

    /// Unpacks a [`GraphNodeId`] and a [`Direction`] from this type.
    fn unwrap(self) -> (Self::Id, Direction);
}

/// Types that are a pair of [`GraphNodeId`]s.
pub trait GraphNodeIdPair: Copy + Eq + Hash + Debug {
    /// The type of [`GraphNodeId`] for each element of the pair.
    type Id: GraphNodeId;

    /// Packs two [`GraphNodeId`]s into a single type.
    fn new(a: Self::Id, b: Self::Id) -> Self;

    /// Unpacks two [`GraphNodeId`]s from this type.
    fn unwrap(self) -> (Self::Id, Self::Id);
}

impl<N: GraphNodeId> DirectedGraphNodeId for (N, Direction) {
    type Id = N;

    fn new(id: N, direction: Direction) -> Self {
        (id, direction)
    }

    fn unwrap(self) -> (N, Direction) {
        self
    }
}

impl<N: GraphNodeId> GraphNodeIdPair for (N, N) {
    type Id = N;

    fn new(a: N, b: N) -> Self {
        (a, b)
    }

    fn unwrap(self) -> (N, N) {
        self
    }
}
