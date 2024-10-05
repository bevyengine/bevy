use core::marker::PhantomData;

use bevy_ecs::{entity::Entity, event::Event};

/// A One-to-One [relationship](crate::OneToOne) [`Event`].
#[derive(Event)]
pub enum OneToOneEvent<R> {
    /// A [relationship](crate::OneToOne) was added between two [entities](Entity).
    Added(OneToOneEventDetails<R>),
    /// A [relationship](crate::OneToOne) was removed from two [entities](Entity).
    Removed(OneToOneEventDetails<R>),
}

impl<R> OneToOneEvent<R> {
    /// Create a new [`Added`](OneToOneEvent::Added) [`Event`]
    pub const fn added(primary: Entity, secondary: Entity) -> Self {
        Self::Added(OneToOneEventDetails::new(primary, secondary))
    }

    /// Create a new [`Removed`](OneToOneEvent::Removed) [`Event`]
    pub const fn removed(primary: Entity, secondary: Entity) -> Self {
        Self::Removed(OneToOneEventDetails::new(primary, secondary))
    }

    /// Get the primary [`Entity`] in this [`Event`].
    /// The primary is the _cause_ of the event, while the secondary is the relation.
    pub const fn primary(&self) -> Entity {
        match self {
            Self::Added(details) | Self::Removed(details) => details.primary(),
        }
    }

    /// Get the secondary [`Entity`] in this [`Event`].
    /// The primary is the _cause_ of the event, while the secondary is the relation.
    pub const fn secondary(&self) -> Entity {
        match self {
            Self::Added(details) | Self::Removed(details) => details.secondary(),
        }
    }
}

impl<R> core::fmt::Debug for OneToOneEvent<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Added(arg0) => f.debug_tuple("Added").field(arg0).finish(),
            Self::Removed(arg0) => f.debug_tuple("Removed").field(arg0).finish(),
        }
    }
}

impl<R> PartialEq for OneToOneEvent<R> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Added(l0), Self::Added(r0)) => l0 == r0,
            (Self::Removed(l0), Self::Removed(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<R> Eq for OneToOneEvent<R> {}

impl<R> Clone for OneToOneEvent<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for OneToOneEvent<R> {}

/// The details of a [`OneToOneEvent`].
pub struct OneToOneEventDetails<R> {
    primary: Entity,
    secondary: Entity,
    phantom_data: PhantomData<fn(R)>,
}

impl<R> OneToOneEventDetails<R> {
    /// Create a new [`OneToOneEventDetails`] for a `primary` and a `secondary` [`Entity`].
    /// The `primary` [`Entity`] is the cause of the [`Event`], while the `secondary`
    /// is the other member of the relationship.
    pub const fn new(primary: Entity, secondary: Entity) -> Self {
        Self {
            primary,
            secondary,
            phantom_data: PhantomData,
        }
    }

    /// Get the [`Entity`] that caused this [`Event`] to be triggered.
    pub const fn primary(&self) -> Entity {
        self.primary
    }

    /// Get the [`Entity`] related to the `primary` [`Entity`].
    pub const fn secondary(&self) -> Entity {
        self.secondary
    }
}

impl<R> core::fmt::Debug for OneToOneEventDetails<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OneToOneEventDetails")
            .field("primary", &self.primary)
            .field("secondary", &self.secondary)
            .finish()
    }
}

impl<R> PartialEq for OneToOneEventDetails<R> {
    fn eq(&self, other: &Self) -> bool {
        self.primary == other.primary && self.secondary == other.secondary
    }
}

impl<R> Eq for OneToOneEventDetails<R> {}

impl<R> Clone for OneToOneEventDetails<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for OneToOneEventDetails<R> {}
