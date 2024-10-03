use core::marker::PhantomData;

use bevy_ecs::{entity::Entity, event::Event};

/// An [`Event`] that is fired whenever there is a change in One-to-Many relationship `R`.
///
/// [`Event`]: bevy_ecs::event::Event
#[derive(Event)]
pub enum OneToManyEvent<R> {
    /// Fired whenever a One-to-Many relationship of type `R` is added between two [entities](Entity)
    Added(Entity, Entity, PhantomData<fn(R)>),
    /// Fired whenever a One-to-Many relationship of type `R` is remove from two [entities](Entity)
    Removed(Entity, Entity, PhantomData<fn(R)>),
}

impl<R> core::fmt::Debug for OneToManyEvent<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Added(arg0, arg1, _) => f.debug_tuple("Added").field(arg0).field(arg1).finish(),
            Self::Removed(arg0, arg1, _) => {
                f.debug_tuple("Removed").field(arg0).field(arg1).finish()
            }
        }
    }
}

impl<R> PartialEq for OneToManyEvent<R> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Added(l0, l1, _), Self::Added(r0, r1, _))
            | (Self::Removed(l0, l1, _), Self::Removed(r0, r1, _)) => l0 == r0 && l1 == r1,
            _ => false,
        }
    }
}

impl<R> Eq for OneToManyEvent<R> {}

impl<R> Clone for OneToManyEvent<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Added(arg0, arg1, _) => Self::Added(*arg0, *arg1, PhantomData),
            Self::Removed(arg0, arg1, _) => Self::Removed(*arg0, *arg1, PhantomData),
        }
    }
}
