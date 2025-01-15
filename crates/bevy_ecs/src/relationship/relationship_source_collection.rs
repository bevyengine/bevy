use crate::entity::Entity;
use alloc::vec::Vec;

pub trait RelationshipSourceCollection {
    fn with_capacity(capacity: usize) -> Self;
    fn add(&mut self, entity: Entity);
    fn remove(&mut self, entity: Entity);
    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity>;
    fn take(&mut self) -> Vec<Entity>;
    fn len(&self) -> usize;
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

    fn take(&mut self) -> Vec<Entity> {
        core::mem::take(self)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }
}
