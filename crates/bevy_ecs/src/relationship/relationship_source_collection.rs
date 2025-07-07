use alloc::collections::{btree_set, BTreeSet};
use core::{
    hash::BuildHasher,
    ops::{Deref, DerefMut},
};

use crate::entity::{Entity, EntityHashSet, EntityIndexSet};
use alloc::vec::Vec;
use indexmap::IndexSet;
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

    /// Creates a new empty instance.
    fn new() -> Self;

    /// Returns an instance with the given pre-allocated entity `capacity`.
    ///
    /// Some collections will ignore the provided `capacity` and return a default instance.
    fn with_capacity(capacity: usize) -> Self;

    /// Reserves capacity for at least `additional` more entities to be inserted.
    ///
    /// Not all collections support this operation, in which case it is a no-op.
    fn reserve(&mut self, additional: usize);

    /// Adds the given `entity` to the collection.
    ///
    /// Returns whether the entity was added to the collection.
    /// Mainly useful when dealing with collections that don't allow
    /// multiple instances of the same entity ([`EntityHashSet`]).
    fn add(&mut self, entity: Entity) -> bool;

    /// Removes the given `entity` from the collection.
    ///
    /// Returns whether the collection actually contained
    /// the entity.
    fn remove(&mut self, entity: Entity) -> bool;

    /// Iterates all entities in the collection.
    fn iter(&self) -> Self::SourceIter<'_>;

    /// Returns the current length of the collection.
    fn len(&self) -> usize;

    /// Clears the collection.
    fn clear(&mut self);

    /// Attempts to save memory by shrinking the capacity to fit the current length.
    ///
    /// This operation is a no-op for collections that do not support it.
    fn shrink_to_fit(&mut self);

    /// Returns true if the collection contains no entities.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Add multiple entities to collection at once.
    ///
    /// May be faster than repeatedly calling [`Self::add`].
    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        // The method name shouldn't conflict with `Extend::extend` as it's in the rust prelude and
        // would always conflict with it.
        for entity in entities {
            self.add(entity);
        }
    }
}

/// This trait signals that a [`RelationshipSourceCollection`] is ordered.
pub trait OrderedRelationshipSourceCollection: RelationshipSourceCollection {
    /// Inserts the entity at a specific index.
    /// If the index is too large, the entity will be added to the end of the collection.
    fn insert(&mut self, index: usize, entity: Entity);
    /// Removes the entity at the specified index if it exists.
    fn remove_at(&mut self, index: usize) -> Option<Entity>;
    /// Inserts the entity at a specific index.
    /// This will never reorder other entities.
    /// If the index is too large, the entity will be added to the end of the collection.
    fn insert_stable(&mut self, index: usize, entity: Entity);
    /// Removes the entity at the specified index if it exists.
    /// This will never reorder other entities.
    fn remove_at_stable(&mut self, index: usize) -> Option<Entity>;
    /// Sorts the source collection.
    fn sort(&mut self);
    /// Inserts the entity at the proper place to maintain sorting.
    fn insert_sorted(&mut self, entity: Entity);

    /// This places the most recently added entity at the particular index.
    fn place_most_recent(&mut self, index: usize);

    /// This places the given entity at the particular index.
    /// This will do nothing if the entity is not in the collection.
    /// If the index is out of bounds, this will put the entity at the end.
    fn place(&mut self, entity: Entity, index: usize);

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

    fn new() -> Self {
        Vec::new()
    }

    fn reserve(&mut self, additional: usize) {
        Vec::reserve(self, additional);
    }

    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) -> bool {
        Vec::push(self, entity);

        true
    }

    fn remove(&mut self, entity: Entity) -> bool {
        if let Some(index) = <[Entity]>::iter(self).position(|e| *e == entity) {
            Vec::remove(self, index);
            return true;
        }

        false
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Entity]>::iter(self).copied()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn shrink_to_fit(&mut self) {
        Vec::shrink_to_fit(self);
    }

    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        self.extend(entities);
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

    fn sort(&mut self) {
        self.sort_unstable();
    }

    fn insert_sorted(&mut self, entity: Entity) {
        let index = self.partition_point(|e| e <= &entity);
        self.insert_stable(index, entity);
    }

    fn place_most_recent(&mut self, index: usize) {
        if let Some(entity) = self.pop() {
            let index = index.min(self.len());
            self.insert(index, entity);
        }
    }

    fn place(&mut self, entity: Entity, index: usize) {
        if let Some(current) = <[Entity]>::iter(self).position(|e| *e == entity) {
            let index = index.min(self.len());
            Vec::remove(self, current);
            self.insert(index, entity);
        };
    }
}

impl RelationshipSourceCollection for EntityHashSet {
    type SourceIter<'a> = core::iter::Copied<crate::entity::hash_set::Iter<'a>>;

    fn new() -> Self {
        EntityHashSet::new()
    }

    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    fn with_capacity(capacity: usize) -> Self {
        EntityHashSet::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) -> bool {
        self.insert(entity)
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

    fn clear(&mut self) {
        self.0.clear();
    }

    fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        self.extend(entities);
    }
}

impl<const N: usize> RelationshipSourceCollection for SmallVec<[Entity; N]> {
    type SourceIter<'a> = core::iter::Copied<core::slice::Iter<'a, Entity>>;

    fn new() -> Self {
        SmallVec::new()
    }

    fn reserve(&mut self, additional: usize) {
        SmallVec::reserve(self, additional);
    }

    fn with_capacity(capacity: usize) -> Self {
        SmallVec::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) -> bool {
        SmallVec::push(self, entity);

        true
    }

    fn remove(&mut self, entity: Entity) -> bool {
        if let Some(index) = <[Entity]>::iter(self).position(|e| *e == entity) {
            SmallVec::remove(self, index);
            return true;
        }

        false
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Entity]>::iter(self).copied()
    }

    fn len(&self) -> usize {
        SmallVec::len(self)
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn shrink_to_fit(&mut self) {
        SmallVec::shrink_to_fit(self);
    }

    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        self.extend(entities);
    }
}

impl RelationshipSourceCollection for Entity {
    type SourceIter<'a> = core::option::IntoIter<Entity>;

    fn new() -> Self {
        Entity::PLACEHOLDER
    }

    fn reserve(&mut self, _: usize) {}

    fn with_capacity(_capacity: usize) -> Self {
        Self::new()
    }

    fn add(&mut self, entity: Entity) -> bool {
        assert_eq!(
            *self,
            Entity::PLACEHOLDER,
            "Entity {entity} attempted to target an entity with a one-to-one relationship, but it is already targeted by {}. You must remove the original relationship first.",
            *self
        );
        *self = entity;

        true
    }

    fn remove(&mut self, entity: Entity) -> bool {
        if *self == entity {
            *self = Entity::PLACEHOLDER;

            return true;
        }

        false
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        if *self == Entity::PLACEHOLDER {
            None.into_iter()
        } else {
            Some(*self).into_iter()
        }
    }

    fn len(&self) -> usize {
        if *self == Entity::PLACEHOLDER {
            return 0;
        }
        1
    }

    fn clear(&mut self) {
        *self = Entity::PLACEHOLDER;
    }

    fn shrink_to_fit(&mut self) {}

    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            assert_eq!(
                *self,
                Entity::PLACEHOLDER,
                "Entity {entity} attempted to target an entity with a one-to-one relationship, but it is already targeted by {}. You must remove the original relationship first.",
                *self
            );
            *self = entity;
        }
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

    fn sort(&mut self) {
        self.sort_unstable();
    }

    fn insert_sorted(&mut self, entity: Entity) {
        let index = self.partition_point(|e| e <= &entity);
        self.insert_stable(index, entity);
    }

    fn place_most_recent(&mut self, index: usize) {
        if let Some(entity) = self.pop() {
            let index = index.min(self.len() - 1);
            self.insert(index, entity);
        }
    }

    fn place(&mut self, entity: Entity, index: usize) {
        if let Some(current) = <[Entity]>::iter(self).position(|e| *e == entity) {
            // The len is at least 1, so the subtraction is safe.
            let index = index.min(self.len() - 1);
            SmallVec::<[Entity; N]>::remove(self, current);
            self.insert(index, entity);
        };
    }
}

impl<S: BuildHasher + Default> RelationshipSourceCollection for IndexSet<Entity, S> {
    type SourceIter<'a>
        = core::iter::Copied<indexmap::set::Iter<'a, Entity>>
    where
        S: 'a;

    fn new() -> Self {
        IndexSet::default()
    }

    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }

    fn with_capacity(capacity: usize) -> Self {
        IndexSet::with_capacity_and_hasher(capacity, S::default())
    }

    fn add(&mut self, entity: Entity) -> bool {
        self.insert(entity)
    }

    fn remove(&mut self, entity: Entity) -> bool {
        self.shift_remove(&entity)
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        self.iter().copied()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn shrink_to_fit(&mut self) {
        self.shrink_to_fit();
    }

    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        self.extend(entities);
    }
}

impl RelationshipSourceCollection for EntityIndexSet {
    type SourceIter<'a> = core::iter::Copied<crate::entity::index_set::Iter<'a>>;

    fn new() -> Self {
        EntityIndexSet::new()
    }

    fn reserve(&mut self, additional: usize) {
        self.deref_mut().reserve(additional);
    }

    fn with_capacity(capacity: usize) -> Self {
        EntityIndexSet::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) -> bool {
        self.insert(entity)
    }

    fn remove(&mut self, entity: Entity) -> bool {
        self.deref_mut().shift_remove(&entity)
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        self.iter().copied()
    }

    fn len(&self) -> usize {
        self.deref().len()
    }

    fn clear(&mut self) {
        self.deref_mut().clear();
    }

    fn shrink_to_fit(&mut self) {
        self.deref_mut().shrink_to_fit();
    }

    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        self.extend(entities);
    }
}

impl RelationshipSourceCollection for BTreeSet<Entity> {
    type SourceIter<'a> = core::iter::Copied<btree_set::Iter<'a, Entity>>;

    fn new() -> Self {
        BTreeSet::new()
    }

    fn with_capacity(_: usize) -> Self {
        // BTreeSet doesn't have a capacity
        Self::new()
    }

    fn reserve(&mut self, _: usize) {
        // BTreeSet doesn't have a capacity
    }

    fn add(&mut self, entity: Entity) -> bool {
        self.insert(entity)
    }

    fn remove(&mut self, entity: Entity) -> bool {
        self.remove(&entity)
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        self.iter().copied()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn shrink_to_fit(&mut self) {
        // BTreeSet doesn't have a capacity
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

    #[test]
    fn entity_relationship_source_collection() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel)]
        struct RelTarget(Entity);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        world.entity_mut(a).insert(Rel(b));

        let rel_target = world.get::<RelTarget>(b).unwrap();
        let collection = rel_target.collection();
        assert_eq!(collection, &a);
    }

    #[test]
    fn one_to_one_relationships() {
        #[derive(Component)]
        #[relationship(relationship_target = Below)]
        struct Above(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Above)]
        struct Below(Entity);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        world.entity_mut(a).insert(Above(b));
        assert_eq!(a, world.get::<Below>(b).unwrap().0);

        // Verify removing target removes relationship
        world.entity_mut(b).remove::<Below>();
        assert!(world.get::<Above>(a).is_none());

        // Verify removing relationship removes target
        world.entity_mut(a).insert(Above(b));
        world.entity_mut(a).remove::<Above>();
        assert!(world.get::<Below>(b).is_none());

        // Actually - a is above c now! Verify relationship was updated correctly
        let c = world.spawn_empty().id();
        world.entity_mut(a).insert(Above(c));
        assert!(world.get::<Below>(b).is_none());
        assert_eq!(a, world.get::<Below>(c).unwrap().0);
    }

    #[test]
    fn entity_index_map() {
        for add_before in [false, true] {
            #[derive(Component)]
            #[relationship(relationship_target = RelTarget)]
            struct Rel(Entity);

            #[derive(Component)]
            #[relationship_target(relationship = Rel, linked_spawn)]
            struct RelTarget(Vec<Entity>);

            let mut world = World::new();
            if add_before {
                let _ = world.spawn_empty().id();
            }
            let a = world.spawn_empty().id();
            let b = world.spawn_empty().id();
            let c = world.spawn_empty().id();
            let d = world.spawn_empty().id();

            world.entity_mut(a).add_related::<Rel>(&[b, c, d]);

            let rel_target = world.get::<RelTarget>(a).unwrap();
            let collection = rel_target.collection();

            // Insertions should maintain ordering
            assert!(collection.iter().eq([b, c, d]));

            world.entity_mut(c).despawn();

            let rel_target = world.get::<RelTarget>(a).unwrap();
            let collection = rel_target.collection();

            // Removals should maintain ordering
            assert!(collection.iter().eq([b, d]));
        }
    }

    #[test]
    #[should_panic]
    fn one_to_one_relationship_shared_target() {
        #[derive(Component)]
        #[relationship(relationship_target = Below)]
        struct Above(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Above)]
        struct Below(Entity);
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        world.entity_mut(a).insert(Above(c));
        world.entity_mut(b).insert(Above(c));
    }

    #[test]
    fn one_to_one_relationship_reinsert() {
        #[derive(Component)]
        #[relationship(relationship_target = Below)]
        struct Above(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Above)]
        struct Below(Entity);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        world.entity_mut(a).insert(Above(b));
        world.entity_mut(a).insert(Above(b));
    }
}
