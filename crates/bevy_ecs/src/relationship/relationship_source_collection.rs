use crate::entity::ContainsEntity;
use crate::entity::{hash_set::EntityHashSet, Entity};
use alloc::vec::Vec;
use smallvec::SmallVec;

/// Represents a single item within a [`RelationshipSourceCollection`].
pub trait RelationshipSourceItem: 'static + ContainsEntity + Copy {
    /// Create a new item from an [`Entity`].
    fn from_entity(entity: Entity) -> Self;
}

/// This trait signals that a [`RelationshipSourceItem`] is ordered.
pub trait OrderedRelationshipSourceItem: RelationshipSourceItem + Ord {}

impl<T: RelationshipSourceItem + Ord> OrderedRelationshipSourceItem for T {}

/// The internal [`Entity`] collection used by a [`RelationshipTarget`](crate::relationship::RelationshipTarget) component.
/// This is not intended to be modified directly by users, as it could invalidate the correctness of relationships.
pub trait RelationshipSourceCollection {
    /// The type of the [`Entity`]-like item stored in the collection.
    type Item: RelationshipSourceItem;

    /// The type of iterator returned by the `iter` method.
    ///
    /// This is an associated type (rather than using a method that returns an opaque return-position impl trait)
    /// to ensure that all methods and traits (like [`DoubleEndedIterator`]) of the underlying collection's iterator
    /// are available to the user when implemented without unduly restricting the possible collections.
    ///
    /// The [`SourceIter`](super::SourceIter) type alias can be helpful to reduce confusion when working with this associated type.
    type SourceIter<'a>: Iterator<Item = Self::Item>
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
    fn add(&mut self, item: Self::Item) -> bool;

    /// Removes the given `entity` from the collection.
    ///
    /// Returns whether the collection actually contained
    /// the entity.
    fn remove(&mut self, item: Self::Item) -> bool;

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
    fn extend_from_iter(&mut self, items: impl IntoIterator<Item = Self::Item>) {
        // The method name shouldn't conflict with `Extend::extend` as it's in the rust prelude and
        // would always conflict with it.
        for item in items {
            self.add(item);
        }
    }
}

/// This trait signals that a [`RelationshipSourceCollection`] is ordered.
pub trait OrderedRelationshipSourceCollection: RelationshipSourceCollection {
    /// Inserts the entity at a specific index.
    /// If the index is too large, the entity will be added to the end of the collection.
    fn insert(&mut self, index: usize, item: Self::Item);
    /// Removes the entity at the specified idnex if it exists.
    fn remove_at(&mut self, index: usize) -> Option<Self::Item>;
    /// Inserts the entity at a specific index.
    /// This will never reorder other entities.
    /// If the index is too large, the entity will be added to the end of the collection.
    fn insert_stable(&mut self, index: usize, item: Self::Item);
    /// Removes the entity at the specified idnex if it exists.
    /// This will never reorder other entities.
    fn remove_at_stable(&mut self, index: usize) -> Option<Self::Item>;
    /// Sorts the source collection.
    fn sort(&mut self);
    /// Inserts the entity at the proper place to maintain sorting.
    fn insert_sorted(&mut self, item: Self::Item);

    /// This places the most recently added entity at the particular index.
    fn place_most_recent(&mut self, index: usize);

    /// This places the given entity at the particular index.
    /// This will do nothing if the entity is not in the collection.
    /// If the index is out of bounds, this will put the entity at the end.
    fn place(&mut self, item: Self::Item, index: usize);

    /// Adds the entity at index 0.
    fn push_front(&mut self, item: Self::Item) {
        self.insert(0, item);
    }

    /// Adds the entity to the back of the collection.
    fn push_back(&mut self, item: Self::Item) {
        self.insert(usize::MAX, item);
    }

    /// Removes the first entity.
    fn pop_front(&mut self) -> Option<Self::Item> {
        self.remove_at(0)
    }

    /// Removes the last entity.
    fn pop_back(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            None
        } else {
            self.remove_at(self.len() - 1)
        }
    }
}

impl<T: RelationshipSourceItem> RelationshipSourceCollection for Vec<T> {
    type Item = T;

    type SourceIter<'a> = core::iter::Copied<core::slice::Iter<'a, T>>;

    fn new() -> Self {
        Vec::new()
    }

    fn reserve(&mut self, additional: usize) {
        Vec::reserve(self, additional);
    }

    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn add(&mut self, item: Self::Item) -> bool {
        Vec::push(self, item);

        true
    }

    fn remove(&mut self, item: Self::Item) -> bool {
        if let Some(index) = <[Self::Item]>::iter(self).position(|i| i.entity() == item.entity()) {
            Vec::remove(self, index);
            return true;
        }

        false
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Self::Item]>::iter(self).copied()
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

    fn extend_from_iter(&mut self, items: impl IntoIterator<Item = Self::Item>) {
        self.extend(items);
    }
}

impl<T: OrderedRelationshipSourceItem> OrderedRelationshipSourceCollection for Vec<T> {
    fn insert(&mut self, index: usize, item: Self::Item) {
        self.push(item);
        let len = self.len();
        if index < len {
            self.swap(index, len - 1);
        }
    }

    fn remove_at(&mut self, index: usize) -> Option<Self::Item> {
        (index < self.len()).then(|| self.swap_remove(index))
    }

    fn insert_stable(&mut self, index: usize, item: Self::Item) {
        if index < self.len() {
            Vec::insert(self, index, item);
        } else {
            self.push(item);
        }
    }

    fn remove_at_stable(&mut self, index: usize) -> Option<Self::Item> {
        (index < self.len()).then(|| self.remove(index))
    }

    fn sort(&mut self) {
        self.sort_unstable();
    }

    fn insert_sorted(&mut self, item: Self::Item) {
        let index = self.partition_point(|e| e <= &item);
        self.insert_stable(index, item);
    }

    fn place_most_recent(&mut self, index: usize) {
        if let Some(item) = self.pop() {
            let index = index.min(self.len().saturating_sub(1));
            self.insert(index, item);
        }
    }

    fn place(&mut self, item: Self::Item, index: usize) {
        if let Some(current) = <[Self::Item]>::iter(self).position(|e| *e == item) {
            // The len is at least 1, so the subtraction is safe.
            let index = index.min(self.len().saturating_sub(1));
            Vec::remove(self, current);
            self.insert(index, item);
        };
    }
}

impl RelationshipSourceCollection for EntityHashSet {
    type Item = Entity;

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

impl<const N: usize, T: RelationshipSourceItem> RelationshipSourceCollection for SmallVec<[T; N]> {
    type Item = T;

    type SourceIter<'a> = core::iter::Copied<core::slice::Iter<'a, T>>;

    fn new() -> Self {
        SmallVec::new()
    }

    fn reserve(&mut self, additional: usize) {
        SmallVec::reserve(self, additional);
    }

    fn with_capacity(capacity: usize) -> Self {
        SmallVec::with_capacity(capacity)
    }

    fn add(&mut self, item: Self::Item) -> bool {
        SmallVec::push(self, item);

        true
    }

    fn remove(&mut self, item: Self::Item) -> bool {
        if let Some(index) = <[Self::Item]>::iter(self).position(|e| e.entity() == item.entity()) {
            SmallVec::remove(self, index);
            return true;
        }

        false
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        <[Self::Item]>::iter(self).copied()
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

    fn extend_from_iter(&mut self, items: impl IntoIterator<Item = Self::Item>) {
        self.extend(items);
    }
}

impl RelationshipSourceItem for Entity {
    fn from_entity(entity: Entity) -> Self {
        entity
    }
}

impl RelationshipSourceCollection for Entity {
    type Item = Self;

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

impl<const N: usize, T: OrderedRelationshipSourceItem> OrderedRelationshipSourceCollection
    for SmallVec<[T; N]>
{
    fn insert(&mut self, index: usize, item: Self::Item) {
        self.push(item);
        let len = self.len();
        if index < len {
            self.swap(index, len - 1);
        }
    }

    fn remove_at(&mut self, index: usize) -> Option<Self::Item> {
        (index < self.len()).then(|| self.swap_remove(index))
    }

    fn insert_stable(&mut self, index: usize, item: Self::Item) {
        if index < self.len() {
            SmallVec::<[Self::Item; N]>::insert(self, index, item);
        } else {
            self.push(item);
        }
    }

    fn remove_at_stable(&mut self, index: usize) -> Option<Self::Item> {
        (index < self.len()).then(|| self.remove(index))
    }

    fn sort(&mut self) {
        self.sort_unstable();
    }

    fn insert_sorted(&mut self, item: Self::Item) {
        let index = self.partition_point(|e| e <= &item);
        self.insert_stable(index, item);
    }

    fn place_most_recent(&mut self, index: usize) {
        if let Some(entity) = self.pop() {
            let index = index.min(self.len() - 1);
            self.insert(index, entity);
        }
    }

    fn place(&mut self, item: Self::Item, index: usize) {
        if let Some(current) = <[Self::Item]>::iter(self).position(|e| *e == item) {
            // The len is at least 1, so the subtraction is safe.
            let index = index.min(self.len() - 1);
            SmallVec::<[Self::Item; N]>::remove(self, current);
            self.insert(index, item);
        };
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
