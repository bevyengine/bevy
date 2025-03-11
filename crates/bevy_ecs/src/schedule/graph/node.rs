use core::{
    fmt::{self, Debug},
    hash::Hash,
};

use crate::schedule::graph::Direction;

/// Unique identifier for a system or system set stored in a [`ScheduleGraph`].
///
/// [`ScheduleGraph`]: crate::schedule::ScheduleGraph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeId {
    /// Identifier for a system.
    System(usize),
    /// Identifier for a system set.
    Set(usize),
}

impl NodeId {
    /// Returns the internal integer value.
    pub const fn index(&self) -> usize {
        match self {
            NodeId::System(index) | NodeId::Set(index) => *index,
        }
    }

    /// Returns `true` if the identified node is a system.
    pub const fn is_system(&self) -> bool {
        matches!(self, NodeId::System(_))
    }

    /// Returns `true` if the identified node is a system set.
    pub const fn is_set(&self) -> bool {
        matches!(self, NodeId::Set(_))
    }

    /// Compare this [`NodeId`] with another.
    pub const fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering::{Equal, Greater, Less};
        use NodeId::{Set, System};

        match (self, other) {
            (System(a), System(b)) | (Set(a), Set(b)) => match a.checked_sub(*b) {
                None => Less,
                Some(0) => Equal,
                Some(_) => Greater,
            },
            (System(_), Set(_)) => Less,
            (Set(_), System(_)) => Greater,
        }
    }
}

impl GraphNodeId for NodeId {
    type Pair = CompactNodeIdPair;
    type Directed = CompactNodeIdAndDirection;
}

/// Compact storage of a [`NodeId`] pair.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct CompactNodeIdPair {
    index_a: usize,
    index_b: usize,
    is_system_a: bool,
    is_system_b: bool,
}

impl Debug for CompactNodeIdPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.unpack().fmt(f)
    }
}

impl GraphNodeIdPair<NodeId> for CompactNodeIdPair {
    fn pack(a: NodeId, b: NodeId) -> Self {
        Self {
            index_a: a.index(),
            index_b: b.index(),
            is_system_a: a.is_system(),
            is_system_b: b.is_system(),
        }
    }

    fn unpack(self) -> (NodeId, NodeId) {
        let a = match self.is_system_a {
            true => NodeId::System(self.index_a),
            false => NodeId::Set(self.index_a),
        };
        let b = match self.is_system_b {
            true => NodeId::System(self.index_b),
            false => NodeId::Set(self.index_b),
        };
        (a, b)
    }
}

/// Compact storage of a [`NodeId`] and a [`Direction`].
#[derive(Clone, Copy)]
pub struct CompactNodeIdAndDirection {
    index: usize,
    is_system: bool,
    direction: Direction,
}

impl Debug for CompactNodeIdAndDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.unpack().fmt(f)
    }
}

impl DirectedGraphNodeId<NodeId> for CompactNodeIdAndDirection {
    fn pack(node: NodeId, dir: Direction) -> Self {
        Self {
            index: node.index(),
            is_system: node.is_system(),
            direction: dir,
        }
    }

    fn unpack(self) -> (NodeId, Direction) {
        let node = match self.is_system {
            true => NodeId::System(self.index),
            false => NodeId::Set(self.index),
        };
        (node, self.direction)
    }
}

/// A node in a [`DiGraph`] or [`UnGraph`].
///
/// [`DiGraph`]: crate::schedule::graph::DiGraph
/// [`UnGraph`]: crate::schedule::graph::UnGraph
pub trait GraphNodeId: Copy + Eq + Ord + Debug + Hash {
    /// A pair of [`GraphNodeId`]s for storing edge information. Typically
    /// stored in a memory-efficient format.
    type Pair: GraphNodeIdPair<Self>;
    /// A pair of [`GraphNodeId`] and [`Direction`] for storing neighbor
    /// information. Typically stored in a memory-efficient format.
    type Directed: DirectedGraphNodeId<Self>;
}

/// A pair of [`GraphNodeId`]s for storing edge information. Typically stored in
/// a memory-efficient format.
pub trait GraphNodeIdPair<Id: GraphNodeId>: Copy + Eq + Hash {
    /// Packs the given identifiers into a pair.
    fn pack(a: Id, b: Id) -> Self;

    /// Unpacks this pair into two identifiers.
    fn unpack(self) -> (Id, Id);
}

impl<Id: GraphNodeId> GraphNodeIdPair<Id> for (Id, Id) {
    fn pack(a: Id, b: Id) -> Self {
        (a, b)
    }

    fn unpack(self) -> (Id, Id) {
        (self.0, self.1)
    }
}

/// A pair of [`GraphNodeId`] and [`Direction`] for storing neighbor
/// information. Typically stored in a memory-efficient format.
pub trait DirectedGraphNodeId<Id: GraphNodeId>: Copy + Debug {
    /// Packs the given identifier and direction into a pair.
    fn pack(node: Id, dir: Direction) -> Self;

    /// Unpacks this pair into the identifier and direction.
    fn unpack(self) -> (Id, Direction);
}

impl<Id: GraphNodeId> DirectedGraphNodeId<Id> for (Id, Direction) {
    fn pack(node: Id, dir: Direction) -> Self {
        (node, dir)
    }

    fn unpack(self) -> (Id, Direction) {
        (self.0, self.1)
    }
}
