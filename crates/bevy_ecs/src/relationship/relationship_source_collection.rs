use crate::entity::{hash_set::EntityHashSet, Entity};
use alloc::vec::Vec;
use smallvec::SmallVec;

/// The internal [`Entity`] collection used by a [`RelationshipTarget`](crate::relationship::RelationshipTarget) component.
/// This is not intended to be modified directly by users, as it could invalidate the correctness of relationships.
pub trait RelationshipSourceCollection {
    /// The type of iterator returned by the `iter` method.
    ///
    /// This is an associated type (rather than using a method that returns an opaque return-position impl trait)
    /// to ensure that all methods and traits (like [`DoubleEndedIterator`]) of the underlying collection's iterator
    /// are available to the user when implemented without unduly restricting the possible collections.
    ///
    /// The [`SourceIter`](super::SourceIter) type alias can be helpful to reduce confusion when working with this associated type.
    type SourceIter<'a>: Iterator<Item = Entity>
    where
        Self: 'a;

    /// Returns an instance with the given pre-allocated entity `capacity`.
    fn with_capacity(capacity: usize) -> Self;

    /// Adds the given `entity` to the collection.
    fn add(&mut self, entity: Entity);

    /// Removes the given `entity` from the collection.
    fn remove(&mut self, entity: Entity);

    /// Iterates all entities in the collection.
    fn iter(&self) -> Self::SourceIter<'_>;

    /// Returns the current length of the collection.
    fn len(&self) -> usize;

    /// Returns true if the collection contains no entities.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RelationshipSourceCollection for Vec<Entity> {
    type SourceIter<'a> = core::iter::Copied<core::slice::Iter<'a, Entity>>;

    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) {
        Vec::push(self, entity);
    }

    fn remove(&mut self, entity: Entity) {
        if let Some(index) = <[Entity]>::iter(self).position(|e| *e == entity) {
            Vec::remove(self, index);
        }
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Entity]>::iter(self).copied()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }
}

impl RelationshipSourceCollection for EntityHashSet {
    type SourceIter<'a> = core::iter::Copied<crate::entity::hash_set::Iter<'a>>;

    fn with_capacity(capacity: usize) -> Self {
        EntityHashSet::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) {
        self.insert(entity);
    }

    fn remove(&mut self, entity: Entity) {
        // We need to call the remove method on the underlying hash set,
        // which takes its argument by reference
        self.0.remove(&entity);
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        self.iter().copied()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

impl<const N: usize> RelationshipSourceCollection for SmallVec<[Entity; N]> {
    type SourceIter<'a> = core::iter::Copied<core::slice::Iter<'a, Entity>>;

    fn with_capacity(capacity: usize) -> Self {
        SmallVec::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) {
        SmallVec::push(self, entity);
    }

    fn remove(&mut self, entity: Entity) {
        if let Some(index) = <[Entity]>::iter(self).position(|e| *e == entity) {
            SmallVec::remove(self, index);
        }
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Entity]>::iter(self).copied()
    }

    fn len(&self) -> usize {
        SmallVec::len(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::{Component, World};
    use crate::relationship::RelationshipTarget;

    #[test]
    fn vec_relationship_source_collection() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel, linked_spawn)]
        struct RelTarget(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        world.entity_mut(a).insert(Rel(b));

        let rel_target = world.get::<RelTarget>(b).unwrap();
        let collection = rel_target.collection();
        assert_eq!(collection, &alloc::vec!(a));
    }

    #[test]
    fn entity_hash_set_relationship_source_collection() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel, linked_spawn)]
        struct RelTarget(EntityHashSet);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        world.entity_mut(a).insert(Rel(b));

        let rel_target = world.get::<RelTarget>(b).unwrap();
        let collection = rel_target.collection();
        assert_eq!(collection, &EntityHashSet::from([a]));
    }

    #[test]
    fn smallvec_relationship_source_collection() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel, linked_spawn)]
        struct RelTarget(SmallVec<[Entity; 4]>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        world.entity_mut(a).insert(Rel(b));

        let rel_target = world.get::<RelTarget>(b).unwrap();
        let collection = rel_target.collection();
        assert_eq!(collection, &SmallVec::from_buf([a]));
    }
}
