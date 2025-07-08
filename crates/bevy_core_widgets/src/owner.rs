//! Defines relationships for ownership of an entity, with no other inherited semantics.
use core::slice;

use bevy_ecs::{component::Component, entity::Entity};

/// A component that represents the owner of an entity. Ownership only determines lifetime,
/// such that the owned entity will be despawned when its owner is despawned. It does not imply
/// any other kind of semantic connection between the two entities.
// TODO: Consider renaming and/or moving this.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
#[relationship(relationship_target = Owned)]
pub struct OwnedBy(pub Entity);

impl OwnedBy {
    /// Return the owned entity.
    pub fn get(&self) -> Entity {
        self.0
    }
}

impl Default for OwnedBy {
    fn default() -> Self {
        OwnedBy(Entity::PLACEHOLDER)
    }
}

/// A component that represents a collection of entities that are owned by another entity.
// #[derive(Component, Default, Reflect)]
// #[reflect(Component)]
#[derive(Component, Default)]
#[relationship_target(relationship = OwnedBy, linked_spawn)]
pub struct Owned(Vec<Entity>);

impl<'a> IntoIterator for &'a Owned {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl core::ops::Deref for Owned {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
