use crate::{Asset, AssetId};
use bevy_ecs::event::Event;
use std::fmt::Debug;

/// Events that occur for a specific [`Asset`], such as "value changed" events and "dependency" events.
#[derive(Event)]
pub enum AssetEvent<A: Asset> {
    /// Emitted whenever an [`Asset`] is added.
    Added { id: AssetId<A> },
    /// Emitted whenever an [`Asset`] value is modified.
    Modified { id: AssetId<A> },
    /// Emitted whenever an [`Asset`] is removed.
    Removed { id: AssetId<A> },
    /// Emitted whenever an [`Asset`] has been fully loaded (including its dependencies and all "recursive dependencies").
    LoadedWithDependencies { id: AssetId<A> },
}

impl<A: Asset> AssetEvent<A> {
    /// Returns `true` if this event is [`AssetEvent::LoadedWithDependencies`] and matches the given `id`.
    pub fn is_loaded_with_dependencies(&self, id: impl Into<AssetId<A>>) -> bool {
        let input_id: AssetId<A> = id.into();
        if let AssetEvent::LoadedWithDependencies { id } = self {
            *id == input_id
        } else {
            false
        }
    }

    /// Returns `true` if this event is [`AssetEvent::Added`] and matches the given `id`.
    pub fn is_added(&self, id: impl Into<AssetId<A>>) -> bool {
        let input_id: AssetId<A> = id.into();
        if let AssetEvent::Added { id } = self {
            *id == input_id
        } else {
            false
        }
    }

    /// Returns `true` if this event is [`AssetEvent::Modified`] and matches the given `id`.
    pub fn is_modified(&self, id: impl Into<AssetId<A>>) -> bool {
        let input_id: AssetId<A> = id.into();
        if let AssetEvent::Modified { id } = self {
            *id == input_id
        } else {
            false
        }
    }

    /// Returns `true` if this event is [`AssetEvent::Removed`] and matches the given `id`.
    pub fn is_removed(&self, id: impl Into<AssetId<A>>) -> bool {
        let input_id: AssetId<A> = id.into();
        if let AssetEvent::Removed { id } = self {
            *id == input_id
        } else {
            false
        }
    }
}

impl<A: Asset> Clone for AssetEvent<A> {
    fn clone(&self) -> Self {
        match self {
            Self::Added { id } => Self::Added { id: *id },
            Self::Modified { id } => Self::Modified { id: *id },
            Self::Removed { id } => Self::Removed { id: *id },
            Self::LoadedWithDependencies { id } => Self::LoadedWithDependencies { id: *id },
        }
    }
}

impl<A: Asset> Copy for AssetEvent<A> {}

impl<A: Asset> Debug for AssetEvent<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Added { id } => f.debug_struct("Added").field("id", id).finish(),
            Self::Modified { id } => f.debug_struct("Modified").field("id", id).finish(),
            Self::Removed { id } => f.debug_struct("Removed").field("id", id).finish(),
            Self::LoadedWithDependencies { id } => f
                .debug_struct("LoadedWithDependencies")
                .field("id", id)
                .finish(),
        }
    }
}

impl<A: Asset> PartialEq for AssetEvent<A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Added { id: l_id }, Self::Added { id: r_id })
            | (Self::Modified { id: l_id }, Self::Modified { id: r_id })
            | (Self::Removed { id: l_id }, Self::Removed { id: r_id })
            | (
                Self::LoadedWithDependencies { id: l_id },
                Self::LoadedWithDependencies { id: r_id },
            ) => l_id == r_id,
            _ => false,
        }
    }
}

impl<A: Asset> Eq for AssetEvent<A> {}
