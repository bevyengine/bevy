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
    /// Returns true if and only if it was present.
    fn remove(&mut self, entity: Entity) -> bool;

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

/// This trait signals that a [`RelationshipSourceCollection`] is ordered.
pub trait OrderedRelationshipSourceCollection: RelationshipSourceCollection {
    /// Inserts the entity at a specific index.
    /// If the index is too large, the entity will be added to the end of the collection.
    fn insert(&mut self, index: usize, entity: Entity);
    /// Removes the entity at the specified idnex if it exists.
    fn remove_at(&mut self, index: usize) -> Option<Entity>;
    /// Inserts the entity at a specific index.
    /// This will never reorder other entities.
    /// If the index is too large, the entity will be added to the end of the collection.
    fn insert_stable(&mut self, index: usize, entity: Entity);
    /// Removes the entity at the specified idnex if it exists.
    /// This will never reorder other entities.
    fn remove_at_stable(&mut self, index: usize) -> Option<Entity>;
    /// Sorts the source collection.
    fn sort(&mut self);
    /// Inserts the entity at the proper place to maintain sorting.
    fn insert_sorted(&mut self, entity: Entity);

    /// Places the `contents` at the given `start` index.
    /// This does not add these entities if they do not exist.
    ///
    /// If the entities contain duplicates, the indices will be respected,
    /// with each entity landing at the index corresponding to its last entry in `contents`.
    /// In the event that any indices extend beyond the length of the collection,
    /// they will be "squezzed" into the proper size.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::relationship::OrderedRelationshipSourceCollection;
    /// use bevy_ecs::prelude::Entity;
    ///
    /// let mut relationship_source = vec![Entity::from_raw(0), Entity::from_raw(1), Entity::from_raw(2), Entity::from_raw(3), Entity::from_raw(4)];
    /// relationship_source.place(1, &[Entity::from_raw(2), Entity::from_raw(3), Entity::from_raw(4), Entity::from_raw(4), Entity::from_raw(4)]);
    /// assert_eq!(&relationship_source, &[Entity::from_raw(0), Entity::from_raw(2), Entity::from_raw(3), Entity::from_raw(1), Entity::from_raw(4)]);
    /// ```
    fn place(&mut self, start: usize, contents: &[Entity]) {
        for (offset, entity) in contents.iter().enumerate() {
            let index = start + offset;
            if self.remove(*entity) {
                self.insert_stable(index, *entity);
            }
        }
    }

    /// Adds the entity at index 0.
    fn push_front(&mut self, entity: Entity) {
        self.insert(0, entity);
    }

    /// Adds the entity to the back of the collection.
    fn push_back(&mut self, entity: Entity) {
        self.insert(usize::MAX, entity);
    }

    /// Removes the first entity.
    fn pop_front(&mut self) -> Option<Entity> {
        self.remove_at(0)
    }

    /// Removes the last entity.
    fn pop_back(&mut self) -> Option<Entity> {
        if self.is_empty() {
            None
        } else {
            self.remove_at(self.len() - 1)
        }
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

    fn remove(&mut self, entity: Entity) -> bool {
        if let Some(index) = <[Entity]>::iter(self).position(|e| *e == entity) {
            Vec::remove(self, index);
            true
        } else {
            false
        }
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Entity]>::iter(self).copied()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }
}

impl OrderedRelationshipSourceCollection for Vec<Entity> {
    fn insert(&mut self, index: usize, entity: Entity) {
        self.push(entity);
        let len = self.len();
        if index < len {
            self.swap(index, len - 1);
        }
    }

    fn remove_at(&mut self, index: usize) -> Option<Entity> {
        (index < self.len()).then(|| self.swap_remove(index))
    }

    fn sort(&mut self) {
        self.sort_unstable();
    }

    fn insert_sorted(&mut self, entity: Entity) {
        let index = self.partition_point(|e| e <= &entity);
        self.insert_stable(index, entity);
    }

    fn insert_stable(&mut self, index: usize, entity: Entity) {
        if index < self.len() {
            Vec::insert(self, index, entity);
        } else {
            self.push(entity);
        }
    }

    fn remove_at_stable(&mut self, index: usize) -> Option<Entity> {
        (index < self.len()).then(|| self.remove(index))
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

    fn remove(&mut self, entity: Entity) -> bool {
        // We need to call the remove method on the underlying hash set,
        // which takes its argument by reference
        self.0.remove(&entity)
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

    fn remove(&mut self, entity: Entity) -> bool {
        if let Some(index) = <[Entity]>::iter(self).position(|e| *e == entity) {
            SmallVec::remove(self, index);
            true
        } else {
            false
        }
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Entity]>::iter(self).copied()
    }

    fn len(&self) -> usize {
        SmallVec::len(self)
    }
}

impl<const N: usize> OrderedRelationshipSourceCollection for SmallVec<[Entity; N]> {
    fn insert(&mut self, index: usize, entity: Entity) {
        self.push(entity);
        let len = self.len();
        if index < len {
            self.swap(index, len - 1);
        }
    }

    fn remove_at(&mut self, index: usize) -> Option<Entity> {
        (index < self.len()).then(|| self.swap_remove(index))
    }

    fn sort(&mut self) {
        self.sort_unstable();
    }

    fn insert_sorted(&mut self, entity: Entity) {
        let index = self.partition_point(|e| e <= &entity);
        self.insert_stable(index, entity);
    }

    fn insert_stable(&mut self, index: usize, entity: Entity) {
        if index < self.len() {
            SmallVec::<[Entity; N]>::insert(self, index, entity);
        } else {
            self.push(entity);
        }
    }

    fn remove_at_stable(&mut self, index: usize) -> Option<Entity> {
        (index < self.len()).then(|| self.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_ecs;
    use crate::prelude::{Component, World};
    use crate::relationship::RelationshipTarget;

    #[test]
    fn vec_relationship_source_collection() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel, despawn_descendants)]
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
        #[relationship_target(relationship = Rel, despawn_descendants)]
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
        #[relationship_target(relationship = Rel, despawn_descendants)]
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
