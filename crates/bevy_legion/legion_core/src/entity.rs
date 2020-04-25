use crate::index::ArchetypeIndex;
use crate::index::ChunkIndex;
use crate::index::ComponentIndex;
use crate::index::SetIndex;
use parking_lot::{Mutex, RwLock, RwLockWriteGuard};
use std::fmt::Display;
use std::num::Wrapping;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

pub type EntityIndex = u32;
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

    pub fn index(self) -> EntityIndex { self.index }
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}#{}", self.index, self.version)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct EntityLocation {
    archetype_index: ArchetypeIndex,
    set_index: SetIndex,
    chunk_index: ChunkIndex,
    component_index: ComponentIndex,
}

impl EntityLocation {
    pub(crate) fn new(
        archetype_index: ArchetypeIndex,
        set_index: SetIndex,
        chunk_index: ChunkIndex,
        component_index: ComponentIndex,
    ) -> Self {
        EntityLocation {
            archetype_index,
            set_index,
            chunk_index,
            component_index,
        }
    }

    pub fn archetype(&self) -> ArchetypeIndex { self.archetype_index }

    pub fn set(&self) -> SetIndex { self.set_index }

    pub fn chunk(&self) -> ChunkIndex { self.chunk_index }

    pub fn component(&self) -> ComponentIndex { self.component_index }
}

pub(crate) struct Locations {
    blocks: Vec<Option<Vec<EntityLocation>>>,
}

impl Locations {
    pub fn new() -> Self { Locations { blocks: Vec::new() } }

    fn index(entity: EntityIndex) -> (usize, usize) {
        let block = entity as usize / BlockAllocator::BLOCK_SIZE;
        let index = entity as usize - block * BlockAllocator::BLOCK_SIZE;
        (block, index)
    }

    pub fn get(&self, entity: Entity) -> Option<EntityLocation> {
        let (block, index) = Locations::index(entity.index());
        self.blocks
            .get(block)
            .map(|b| b.as_ref())
            .flatten()
            .map(|b| b[index])
    }

    pub fn set(&mut self, entity: Entity, location: EntityLocation) {
        let (block_index, index) = Locations::index(entity.index());
        if self.blocks.len() <= block_index {
            let fill = block_index - self.blocks.len() + 1;
            self.blocks.extend((0..fill).map(|_| None));
        }

        let block_opt = &mut self.blocks[block_index];
        let block = block_opt.get_or_insert_with(|| {
            std::iter::repeat(EntityLocation::new(
                ArchetypeIndex(0),
                SetIndex(0),
                ChunkIndex(0),
                ComponentIndex(0),
            ))
            .take(BlockAllocator::BLOCK_SIZE)
            .collect()
        });

        block[index] = location;
    }
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
pub struct EntityBlock {
    start: EntityIndex,
    len: usize,
    versions: Vec<EntityVersion>,
    free: Vec<EntityIndex>,
}

impl EntityBlock {
    pub fn new(start: EntityIndex, len: usize) -> EntityBlock {
        EntityBlock {
            start,
            len,
            versions: Vec::with_capacity(len),
            free: Vec::new(),
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

    pub fn free(&mut self, entity: Entity) -> bool {
        if let Some(true) = self.is_alive(entity) {
            let i = self.index(entity.index);
            self.versions[i] += Wrapping(1);
            self.free.push(entity.index);
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
struct Blocks {
    blocks: Vec<Option<EntityBlock>>,
}

impl Blocks {
    fn new() -> Self { Self { blocks: Vec::new() } }

    pub fn index(entity: EntityIndex) -> usize { entity as usize / BlockAllocator::BLOCK_SIZE }

    fn find(&self, entity: EntityIndex) -> Option<&EntityBlock> {
        let i = Blocks::index(entity);
        self.blocks.get(i).map(|b| b.as_ref()).flatten()
    }

    fn find_mut(&mut self, entity: EntityIndex) -> Option<&mut EntityBlock> {
        let i = Blocks::index(entity);
        self.blocks.get_mut(i).map(|b| b.as_mut()).flatten()
    }

    fn push(&mut self, block: EntityBlock) -> usize {
        let i = Blocks::index(block.start);
        if self.blocks.len() > i {
            self.blocks[i] = Some(block);
        } else {
            let fill = i - self.blocks.len();
            self.blocks.extend((0..fill).map(|_| None));
            self.blocks.push(Some(block));
        }
        i
    }

    fn append(&mut self, other: &mut Blocks) {
        for block in other.blocks.drain(..) {
            if let Some(block) = block {
                self.push(block);
            }
        }
    }
}

impl Deref for Blocks {
    type Target = [Option<EntityBlock>];
    fn deref(&self) -> &Self::Target { self.blocks.deref() }
}

impl DerefMut for Blocks {
    fn deref_mut(&mut self) -> &mut Self::Target { self.blocks.deref_mut() }
}

/// Manages the allocation and deletion of `Entity` IDs within a world.
#[derive(Debug)]
pub struct EntityAllocator {
    allocator: Arc<Mutex<BlockAllocator>>,
    blocks: RwLock<Blocks>,
}

impl EntityAllocator {
    pub(crate) fn new(allocator: Arc<Mutex<BlockAllocator>>) -> Self {
        EntityAllocator {
            allocator,
            blocks: RwLock::new(Blocks::new()),
        }
    }

    /// Determines if the given `Entity` is considered alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.blocks
            .read()
            .find(entity.index())
            .map(|b| b.is_alive(entity))
            .flatten()
            .unwrap_or(false)
    }

    /// Allocates a new unused `Entity` ID.
    pub fn create_entity(&self) -> Entity { self.create_entities().next().unwrap() }

    /// Creates an iterator which allocates new `Entity` IDs.
    pub fn create_entities(&self) -> CreateEntityIter {
        CreateEntityIter {
            blocks: self.blocks.write(),
            allocator: &self.allocator,
            current_block: None,
        }
    }

    pub(crate) fn delete_entity(&self, entity: Entity) -> bool {
        self.blocks
            .write()
            .find_mut(entity.index())
            .map(|b| b.free(entity))
            .unwrap_or(false)
    }

    pub(crate) fn delete_all_entities(&self) {
        for block in self.blocks.write().blocks.drain(..) {
            if let Some(mut block) = block {
                // If any entity in the block is in an allocated state, clear
                // and repopulate the free list. This forces all entities into an
                // unallocated state. Bump versions of all entity indexes to
                // ensure that we don't reuse the same entity.
                if block.free.len() < block.versions.len() {
                    block.free.clear();
                    for (i, version) in block.versions.iter_mut().enumerate() {
                        *version += Wrapping(1);
                        block.free.push(i as u32 + block.start);
                    }
                }

                self.allocator.lock().free(block);
            }
        }
    }

    pub(crate) fn merge(&self, other: EntityAllocator) {
        assert!(Arc::ptr_eq(&self.allocator, &other.allocator));
        self.blocks.write().append(&mut *other.blocks.write());
    }
}

impl Drop for EntityAllocator {
    fn drop(&mut self) { self.delete_all_entities(); }
}

pub struct CreateEntityIter<'a> {
    current_block: Option<usize>,
    blocks: RwLockWriteGuard<'a, Blocks>,
    allocator: &'a Mutex<BlockAllocator>,
}

impl<'a> Iterator for CreateEntityIter<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        // try and allocate from the block we last used
        if let Some(block) = self.current_block {
            if let Some(entity) = self.blocks[block].as_mut().unwrap().allocate() {
                return Some(entity);
            }
        }

        // search for a block with spare entities
        for (i, allocated) in self
            .blocks
            .iter_mut()
            .enumerate()
            .rev()
            .filter(|(_, b)| b.is_some())
            .map(|(i, b)| (i, b.as_mut().unwrap().allocate()))
        {
            if let Some(entity) = allocated {
                self.current_block = Some(i);
                return Some(entity);
            }
        }

        // allocate a new block
        let mut block = self.allocator.lock().allocate();
        let entity = block.allocate().unwrap();
        self.current_block = Some(self.blocks.push(block));
        Some(entity)
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

        assert_eq!(true, allocator.delete_entity(entity));
    }

    #[test]
    fn delete_entity_was_dead() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = allocator.create_entity();
        allocator.delete_entity(entity);

        assert_eq!(false, allocator.delete_entity(entity));
    }

    #[test]
    fn delete_entity_was_unallocated() {
        let allocator = EntityAllocator::new(Arc::from(Mutex::new(BlockAllocator::new())));
        let entity = Entity::new(10 as EntityIndex, Wrapping(10));

        assert_eq!(false, allocator.delete_entity(entity));
    }

    #[test]
    fn multiple_allocators_unique_ids() {
        let blocks = Arc::from(Mutex::new(BlockAllocator::new()));
        let allocator_a = EntityAllocator::new(blocks.clone());
        let allocator_b = EntityAllocator::new(blocks);

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
