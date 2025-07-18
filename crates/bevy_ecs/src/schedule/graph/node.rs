use core::{fmt::Debug, hash::Hash};

use crate::schedule::graph::Direction;

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
}
