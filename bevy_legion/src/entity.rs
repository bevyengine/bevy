use parking_lot::{Mutex, RwLock};
use std::fmt::Display;
use std::num::Wrapping;
use std::sync::Arc;

pub(crate) type EntityIndex = u32;
pub(crate) type EntityVersion = Wrapping<u32>;

/// A handle to an entity.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Entity {
    index: EntityIndex,
    version: EntityVersion,
}

impl Entity {
    pub(crate) fn new(index: EntityIndex, version: EntityVersion) -> Entity {
        Entity { index, version }
    }

    pub(crate) fn index(self) -> EntityIndex { self.index }
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}#{}", self.index, self.version)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct EntityLocation {
    archetype_index: usize,
    set_index: usize,
    chunk_index: usize,
    component_index: usize,
}

impl EntityLocation {
    pub(crate) fn new(
        archetype_index: usize,
        set_index: usize,
        chunk_index: usize,
        component_index: usize,
    ) -> Self {
        EntityLocation {
            archetype_index,
            set_index,
            chunk_index,
            component_index,
        }
    }

    pub(crate) fn archetype(&self) -> usize { self.archetype_index }

    pub(crate) fn set(&self) -> usize { self.set_index }

    pub(crate) fn chunk(&self) -> usize { self.chunk_index }

    pub(crate) fn component(&self) -> usize { self.component_index }
}

#[derive(Debug)]
pub(crate) struct BlockAllocator {
    allocated: usize,
    free: Vec<EntityBlock>,
}

impl BlockAllocator {
    const BLOCK_SIZE: usize = 1024;

    pub(crate) fn new() -> Self {
        BlockAllocator {
            allocated: 0,
            free: Vec::new(),
        }
    }

    pub fn allocate(&mut self) -> EntityBlock {
        if let Some(block) = self.free.pop() {
            block
        } else {
            let block = EntityBlock::new(self.allocated as EntityIndex, BlockAllocator::BLOCK_SIZE);
            self.allocated += BlockAllocator::BLOCK_SIZE;
            block
        }
    }

    pub fn free(&mut self, block: EntityBlock) { self.free.push(block); }
}

#[derive(Debug)]
pub(crate) struct EntityBlock {
    start: EntityIndex,
    len: usize,
    versions: Vec<EntityVersion>,
    free: Vec<EntityIndex>,
    locations: Vec<EntityLocation>,
}

impl EntityBlock {
    pub fn new(start: EntityIndex, len: usize) -> EntityBlock {
        EntityBlock {
            start,
            len,
            versions: Vec::with_capacity(len),
            free: Vec::new(),
            locations: std::iter::repeat(EntityLocation::new(0, 0, 0, 0))
                .take(len)
                .collect(),
        }
    }

    fn index(&self, index: EntityIndex) -> usize { (index - self.start) as usize }

    pub fn in_range(&self, index: EntityIndex) -> bool {
        index >= self.start && index < (self.start + self.len as u32)
    }

    pub fn is_alive(&self, entity: Entity) -> Option<bool> {
        if entity.index >= self.start {
            let i = self.index(entity.index);
            self.versions.get(i).map(|v| *v == entity.version)
        } else {
            None
        }
    }

    pub fn allocate(&mut self) -> Option<Entity> {
        if let Some(index) = self.free.pop() {
            let i = self.index(index);
            Some(Entity::new(index, self.versions[i]))
        } else if self.versions.len() < self.len {
            let index = self.start + self.versions.len() as EntityIndex;
            self.versions.push(Wrapping(1));
            Some(Entity::new(index, Wrapping(1)))
        } else {
            None
        }
    }

    pub fn free(&mut self, entity: Entity) -> Option<EntityLocation> {
        if let Some(true) = self.is_alive(entity) {
            let i = self.index(entity.index);
            self.versions[i] += Wrapping(1);
            self.free.push(entity.index);
            self.get_location(entity.index)
        } else {
            None
        }
    }

    pub fn set_location(&mut self, entity: EntityIndex, location: EntityLocation) {
        assert!(entity >= self.start);
        let index = (entity - self.start) as usize;
        *self.locations.get_mut(index).unwrap() = location;
    }

    pub fn get_location(&self, entity: EntityIndex) -> Option<EntityLocation> {
        if entity < self.start {
            return None;
        }

        let index = (entity - self.start) as usize;
        self.locations.get(index).copied()
    }
}

/// Manages the allocation and deletion of `Entity` IDs within a world.
#[derive(Debug, Clone)]
pub struct EntityAllocator {
    allocator: Arc<Mutex<BlockAllocator>>,
    blocks: Arc<RwLock<Vec<EntityBlock>>>,
}

impl EntityAllocator {
    pub(crate) fn new(allocator: Arc<Mutex<BlockAllocator>>) -> Self {
        EntityAllocator {
            allocator,
            blocks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Determines if the given `Entity` is considered alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.blocks
            .read()
            .iter()
            .filter_map(|b| b.is_alive(entity))
            .nth(0)
            .unwrap_or(false)
    }

    /// Allocates a new unused `Entity` ID.
    pub fn create_entity(&self) -> Entity {
        let mut blocks = self.blocks.write();

        if let Some(entity) = blocks.iter_mut().rev().filter_map(|b| b.allocate()).nth(0) {
            entity
        } else {
            let mut block = self.allocator.lock().allocate();
            let entity = block.allocate().unwrap();
            blocks.push(block);
            entity
        }
    }

    pub(crate) fn delete_entity(&self, entity: Entity) -> Option<EntityLocation> {
        self.blocks.write().iter_mut().find_map(|b| b.free(entity))
    }

    pub(crate) fn set_location(&self, entity: EntityIndex, location: EntityLocation) {
        self.blocks
            .write()
            .iter_mut()
            .rev()
            .find(|b| b.in_range(entity))
            .unwrap()
            .set_location(entity, location);
    }

    pub(crate) fn get_location(&self, entity: EntityIndex) -> Option<EntityLocation> {
        self.blocks
            .read()
            .iter()
            .find(|b| b.in_range(entity))
            .and_then(|b| b.get_location(entity))
    }

    pub(crate) fn merge(&self, other: EntityAllocator) {
        assert!(Arc::ptr_eq(&self.allocator, &other.allocator));
        self.blocks.write().append(&mut other.blocks.write());
    }
}

impl Drop for EntityAllocator {
    fn drop(&mut self) {
        for block in self.blocks.write().drain(..) {
            self.allocator.lock().free(block);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::*;
    use std::collections::HashSet;

    #[test]
    fn create_entity() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        allocator.create_entity();
    }

    #[test]
    fn create_entity_many() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));

        for _ in 0..512 {
            allocator.create_entity();
        }
    }

    #[test]
    fn create_entity_many_blocks() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));

        for _ in 0..3000 {
            allocator.create_entity();
        }
    }

    #[test]
    fn create_entity_recreate() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));

        for _ in 0..3 {
            let entities: Vec<Entity> = (0..512).map(|_| allocator.create_entity()).collect();
            for e in entities {
                allocator.delete_entity(e);
            }
        }
    }

    #[test]
    fn is_alive_allocated() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = allocator.create_entity();

        assert_eq!(true, allocator.is_alive(entity));
    }

    #[test]
    fn is_alive_unallocated() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = Entity::new(10 as EntityIndex, Wrapping(10));

        assert_eq!(false, allocator.is_alive(entity));
    }

    #[test]
    fn is_alive_killed() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = allocator.create_entity();
        allocator.delete_entity(entity);

        assert_eq!(false, allocator.is_alive(entity));
    }

    #[test]
    fn delete_entity_was_alive() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = allocator.create_entity();

        assert_eq!(true, allocator.delete_entity(entity).is_some());
    }

    #[test]
    fn delete_entity_was_dead() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = allocator.create_entity();
        allocator.delete_entity(entity);

        assert_eq!(None, allocator.delete_entity(entity));
    }

    #[test]
    fn delete_entity_was_unallocated() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = Entity::new(10 as EntityIndex, Wrapping(10));

        assert_eq!(None, allocator.delete_entity(entity));
    }

    #[test]
    fn multiple_allocators_unique_ids() {
        let blocks = Arc::from(Mutex::new(BlockAllocator::new()));
        let allocator_a = EntityAllocator::new(blocks.clone());
        let allocator_b = EntityAllocator::new(blocks.clone());

        let mut entities_a = HashSet::<Entity>::default();
        let mut entities_b = HashSet::<Entity>::default();

        for _ in 0..5 {
            entities_a.extend((0..1500).map(|_| allocator_a.create_entity()));
            entities_b.extend((0..1500).map(|_| allocator_b.create_entity()));
        }

        assert_eq!(true, entities_a.is_disjoint(&entities_b));

        for e in entities_a {
            assert_eq!(true, allocator_a.is_alive(e));
            assert_eq!(false, allocator_b.is_alive(e));
        }

        for e in entities_b {
            assert_eq!(false, allocator_a.is_alive(e));
            assert_eq!(true, allocator_b.is_alive(e));
        }
    }
}
