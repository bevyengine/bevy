use core::fmt::Debug;

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
