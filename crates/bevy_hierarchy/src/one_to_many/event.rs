use core::marker::PhantomData;

use bevy_ecs::{entity::Entity, event::Event};

/// A One-to-Many [relationship](crate::OneToManyOne) [`Event`].
#[derive(Event)]
pub enum OneToManyEvent<R> {
    /// A [relationship](crate::OneToManyOne) was added between two [entities](Entity).
    Added(OneToManyEventDetails<R>),
    /// A [relationship](crate::OneToManyOne) was removed from two [entities](Entity).
    Removed(OneToManyEventDetails<R>),
}

impl<R> OneToManyEvent<R> {
    /// Create a new [`Added`](OneToOneEvent::Added) [`Event`]
    pub const fn added(many: Entity, one: Entity) -> Self {
        Self::Added(OneToManyEventDetails::new(many, one))
    }

    /// Create a new [`Removed`](OneToOneEvent::Removed) [`Event`]
    pub const fn removed(many: Entity, one: Entity) -> Self {
        Self::Removed(OneToManyEventDetails::new(many, one))
    }

    /// Get the [`Entity`] that has the [`OneToManyMany`](crate::OneToManyMany) component.
    pub const fn many(&self) -> Entity {
        match self {
            Self::Added(details) | Self::Removed(details) => details.many(),
        }
    }

    /// Get the [`Entity`] that has the [`OneToManyOne`](crate::OneToManyOne) component.
    pub const fn one(&self) -> Entity {
        match self {
            Self::Added(details) | Self::Removed(details) => details.one(),
        }
    }
}

impl<R> core::fmt::Debug for OneToManyEvent<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Added(arg0) => f.debug_tuple("Added").field(arg0).finish(),
            Self::Removed(arg0) => f.debug_tuple("Removed").field(arg0).finish(),
        }
    }
}

impl<R> PartialEq for OneToManyEvent<R> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Added(l0), Self::Added(r0)) => l0 == r0,
            (Self::Removed(l0), Self::Removed(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<R> Eq for OneToManyEvent<R> {}

impl<R> Clone for OneToManyEvent<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for OneToManyEvent<R> {}

/// The details of a [`OneToManyEvent`].
pub struct OneToManyEventDetails<R> {
    many: Entity,
    one: Entity,
    phantom_data: PhantomData<fn(R)>,
}

impl<R> OneToManyEventDetails<R> {
    /// Create a new [`OneToManyEventDetails`] for a `many` and a `one` [`Entity`].
    /// The `many` [`Entity`] has the [`OneToManyMany`](crate::OneToManyMany) component,
    /// while the `one` [`Entity`] has the [`OneToManyOne`](crate::OneToManyOne) component.
    pub const fn new(many: Entity, one: Entity) -> Self {
        Self {
            many,
            one,
            phantom_data: PhantomData,
        }
    }

    /// Get the [`Entity`] that has the [`OneToManyMany`](crate::OneToManyMany) component.
    pub const fn many(&self) -> Entity {
        self.many
    }

    /// Get the [`Entity`] that has the [`OneToManyOne`](crate::OneToManyOne) component.
    pub const fn one(&self) -> Entity {
        self.many
    }
}

impl<R> core::fmt::Debug for OneToManyEventDetails<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OneToManyEventDetails")
            .field("many", &self.many)
            .field("one", &self.one)
            .finish()
    }
}

impl<R> PartialEq for OneToManyEventDetails<R> {
    fn eq(&self, other: &Self) -> bool {
        self.many == other.many && self.one == other.one
    }
}

impl<R> Eq for OneToManyEventDetails<R> {}

impl<R> Clone for OneToManyEventDetails<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for OneToManyEventDetails<R> {}
