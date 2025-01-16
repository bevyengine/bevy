use crate::entity::Entity;
use alloc::vec::Vec;

/// The internal [`Entity`] collection used by a [`RelationshipSources`](crate::relationship::RelationshipSources) component.
/// This is not intended to be modified directly by users, as it could invalidate the correctness of relationships.
pub trait RelationshipSourceCollection {
    /// Returns an instance with the given pre-allocated entity `capacity`.
    fn with_capacity(capacity: usize) -> Self;

    /// Adds the given `entity` to the collection.
    fn add(&mut self, entity: Entity);

    /// Removes the given `entity` from the collection.
    fn remove(&mut self, entity: Entity);

    /// Iterates all entities in the collection.
    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity>;

    /// Returns the current length of the collection.
    fn len(&self) -> usize;

    /// Returns true if the collection contains no entities.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RelationshipSourceCollection for Vec<Entity> {
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

    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity> {
        <[Entity]>::iter(self).copied()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }
}
