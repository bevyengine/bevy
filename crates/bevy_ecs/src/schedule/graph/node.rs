use core::fmt::Debug;

use crate::schedule::{SystemKey, SystemSetKey};

/// Unique identifier for a system or system set stored in a [`ScheduleGraph`].
///
/// [`ScheduleGraph`]: crate::schedule::ScheduleGraph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeId {
    /// Identifier for a system.
    System(SystemKey),
    /// Identifier for a system set.
    Set(SystemSetKey),
}

impl NodeId {
    /// Returns `true` if the identified node is a system.
    pub const fn is_system(&self) -> bool {
        matches!(self, NodeId::System(_))
    }

    /// Returns `true` if the identified node is a system set.
    pub const fn is_set(&self) -> bool {
        matches!(self, NodeId::Set(_))
    }

    /// Returns the system key if the node is a system, otherwise `None`.
    pub const fn as_system(&self) -> Option<SystemKey> {
        match self {
            NodeId::System(system) => Some(*system),
            NodeId::Set(_) => None,
        }
    }

    /// Returns the system set key if the node is a system set, otherwise `None`.
    pub const fn as_set(&self) -> Option<SystemSetKey> {
        match self {
            NodeId::System(_) => None,
            NodeId::Set(set) => Some(*set),
        }
    }
}
