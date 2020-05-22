use crate::entity::Entity;
use parking_lot::RwLock;
use std::{collections::HashSet, num::Wrapping, sync::Arc};

#[derive(Default, Debug, Clone)]
pub struct GuidEntityAllocator {
    entities: Arc<RwLock<HashSet<Entity>>>,
    next_ids: Arc<RwLock<Vec<Entity>>>,
}

impl GuidEntityAllocator {
    pub fn is_alive(&self, entity: Entity) -> bool { self.entities.read().contains(&entity) }

    pub fn push_next_ids(&self, ids: impl Iterator<Item = Entity>) {
        self.next_ids.write().extend(ids);
    }

    /// Allocates a new unused `Entity` ID.
    pub fn create_entity(&self) -> Entity {
        let entity = if !self.next_ids.read().is_empty() {
            self.next_ids.write().pop().unwrap()
        } else {
            Entity::new(rand::random::<u32>(), Wrapping(1))
        };

        self.entities.write().insert(entity);
        entity
    }

    /// Creates an iterator which allocates new `Entity` IDs.
    pub fn create_entities(&self) -> GuidCreateEntityIter {
        GuidCreateEntityIter { allocator: self }
    }

    pub(crate) fn delete_entity(&self, entity: Entity) -> bool {
        self.entities.write().remove(&entity)
    }

    pub(crate) fn delete_all_entities(&self) { self.entities.write().clear(); }

    pub(crate) fn merge(&self, other: GuidEntityAllocator) {
        self.entities.write().extend(other.entities.write().drain())
    }
}

pub struct GuidCreateEntityIter<'a> {
    allocator: &'a GuidEntityAllocator,
}

impl<'a> Iterator for GuidCreateEntityIter<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> { Some(self.allocator.create_entity()) }
}
