use alloc::vec::Vec;
use core::{
    convert::TryFrom,
    fmt, mem,
    sync::atomic::{AtomicU32, Ordering},
};
#[cfg(feature = "std")]
use std::error::Error;

/// Lightweight unique ID of an entity
///
/// Obtained from `World::spawn`. Can be stored to refer to an entity in the future.
#[derive(Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity {
    pub(crate) generation: u32,
    pub(crate) id: u32,
}

impl Entity {
    /// Creates a new entity reference with a generation of 0
    pub fn new(id: u32) -> Entity {
        Entity { id, generation: 0 }
    }

    /// Convert to a form convenient for passing outside of rust
    ///
    /// Only useful for identifying entities within the same instance of an application. Do not use
    /// for serialization between runs.
    ///
    /// No particular structure is guaranteed for the returned bits.
    pub fn to_bits(self) -> u64 {
        u64::from(self.generation) << 32 | u64::from(self.id)
    }

    /// Reconstruct an `Entity` previously destructured with `to_bits`
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    pub fn from_bits(bits: u64) -> Self {
        Self {
            generation: (bits >> 32) as u32,
            id: bits as u32,
        }
    }

    /// Extract a transiently unique identifier
    ///
    /// No two simultaneously-live entities share the same ID, but dead entities' IDs may collide
    /// with both live and dead entities. Useful for compactly representing entities within a
    /// specific snapshot of the world, such as when serializing.
    pub fn id(self) -> u32 {
        self.id
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.id, self.generation)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Entities {
    pub meta: Vec<EntityMeta>,

    // Reserved entities outside the range of `meta`, having implicit generation 0, archetype 0, and
    // undefined index. Calling `flush` converts these to real entities, which can have a fully
    // defined location.
    pending: AtomicU32,

    // Unused entity IDs below `meta.len()`, containing a freelist followed by the reserved IDs.
    // By decrementing `free_cursor`, we move an ID from the freelist to reserved without
    // actually modifying the array.
    //
    // This is the same size as meta, so we can freelist every Entity in existence.
    unused: Vec<u32>,

    // `unused[0..free_cursor]` are freelisted IDs, all below `meta.len()`.
    free_cursor: AtomicU32,

    // `unused[free_cursor..reserved_cursor]` are reserved IDs, all below `meta.len()`.
    // They need to be consumed and used to initialize locations to produce real entities after
    // calling `flush`.
    //
    // The invariant is `reserved_cursor >= free_cursor`.
    reserved_cursor: u32,
}

impl Entities {
    /// Reserve an entity ID concurrently
    ///
    /// Storage for entity generation and location is lazily allocated by calling `flush`. Locations
    /// can be determined by the return value of `flush` and by iterating through the `reserved`
    /// accessors, and should all be written immediately after flushing.
    pub fn reserve_entity(&self) -> Entity {
        loop {
            let index = self.free_cursor.load(Ordering::Relaxed);
            match index.checked_sub(1) {
                // The freelist is empty, so increment `pending` to arrange for a new entity with a
                // predictable ID to be allocated on the next `flush` call
                None => {
                    let n = self.pending.fetch_add(1, Ordering::Relaxed);
                    return Entity {
                        generation: 0,
                        id: u32::try_from(self.meta.len())
                            .ok()
                            .and_then(|x| x.checked_add(n))
                            .expect("too many entities"),
                    };
                }
                // The freelist has entities in it, so shift over the boundary between the
                // freelisted values and the reserved values by one. Reserved values will be
                // consumed by the caller as part of a higher-level flush.
                Some(next) => {
                    // We don't care about memory ordering here so long as we get our slot.
                    if self
                        .free_cursor
                        .compare_exchange_weak(index, next, Ordering::Relaxed, Ordering::Relaxed)
                        .is_err()
                    {
                        // Another thread already consumed this slot, start over.
                        continue;
                    }
                    let id = self.unused[next as usize];
                    return Entity {
                        generation: self.meta[id as usize].generation,
                        id,
                    };
                }
            }
        }
    }

    /// Allocate an entity ID directly
    ///
    /// Location should be written immediately.
    pub fn alloc(&mut self) -> Entity {
        debug_assert_eq!(
            self.pending.load(Ordering::Relaxed),
            0,
            "allocator must be flushed before potentially growing"
        );

        let free_cursor = self.free_cursor.load(Ordering::Relaxed);

        debug_assert_eq!(
            self.reserved_cursor, free_cursor,
            "allocator must be flushed before potentially growing"
        );

        let new_free_cursor = match free_cursor.checked_sub(1) {
            None => {
                self.grow(0);
                self.free_cursor.load(Ordering::Relaxed) - 1 // Not racey due to &mut self
            }
            Some(next) => next,
        };

        let id = self.unused[new_free_cursor as usize];

        self.free_cursor.store(new_free_cursor, Ordering::Relaxed); // Not racey due to &mut self

        // Nothing is reserved, because we flushed, so reserved_cursor == free_cursor.
        self.reserved_cursor = new_free_cursor;

        Entity {
            generation: self.meta[id as usize].generation,
            id,
        }
    }

    /// Destroy an entity, allowing it to be reused
    ///
    /// Must not be called on reserved entities prior to `flush`.
    pub fn free(&mut self, entity: Entity) -> Result<Location, NoSuchEntity> {
        let meta = &mut self.meta[entity.id as usize];
        if meta.generation != entity.generation {
            return Err(NoSuchEntity);
        }
        meta.generation += 1;
        let loc = mem::replace(
            &mut meta.location,
            Location {
                archetype: 0,
                // Guard against bugs in reservation handling
                index: usize::max_value(),
            },
        );

        let index = self.free_cursor.fetch_add(1, Ordering::Relaxed); // Not racey due to &mut self

        let reserved_cursor = self.reserved_cursor;
        self.reserved_cursor = reserved_cursor + 1;
        if reserved_cursor > index {
            // We need to slide up the reserved range, moving the first reserved ID
            // to the end, to make room.
            //
            // QUESTION: I think this can only happen if we can free IDs while unflushed
            // stuff exists, is that legal? There's no assert preventing it. If it's
            // illegal we can delete this, because there can't be reserved things here.
            self.unused[reserved_cursor as usize] = self.unused[index as usize];
        }

        self.unused[index as usize] = entity.id;
        debug_assert!(
            loc.index != usize::max_value(),
            "free called on reserved entity without flush"
        );
        Ok(loc)
    }

    /// Ensure at least `n` allocations can succeed without reallocating
    pub fn reserve(&mut self, additional: u32) {
        debug_assert_eq!(
            self.pending.load(Ordering::Relaxed),
            0,
            "allocator must be flushed before potentially growing"
        );
        let free = self.free_cursor.load(Ordering::Relaxed);
        if additional > free {
            self.grow(additional - free);
        }
    }

    pub fn contains(&self, entity: Entity) -> bool {
        if entity.id >= self.meta.len() as u32 {
            return true;
        }
        self.meta[entity.id as usize].generation == entity.generation
    }

    pub fn clear(&mut self) {
        // Not racey due to &mut self
        let end = self.unused.len() as u32;
        self.free_cursor.store(end, Ordering::Relaxed);
        self.reserved_cursor = end;
        for (i, x) in self.unused.iter_mut().enumerate() {
            *x = i as u32;
        }
        self.pending.store(0, Ordering::Relaxed);
    }

    /// Access the location storage of an entity
    ///
    /// Must not be called on pending entities.
    pub fn get_mut(&mut self, entity: Entity) -> Result<&mut Location, NoSuchEntity> {
        let meta = &mut self.meta[entity.id as usize];
        if meta.generation == entity.generation {
            Ok(&mut meta.location)
        } else {
            Err(NoSuchEntity)
        }
    }

    /// Returns `Ok(Location { archetype: 0, index: undefined })` for pending entities
    pub fn get(&self, entity: Entity) -> Result<Location, NoSuchEntity> {
        if self.meta.len() <= entity.id as usize {
            return Ok(Location {
                archetype: 0,
                index: usize::max_value(),
            });
        }
        let meta = &self.meta[entity.id as usize];
        if meta.generation != entity.generation {
            return Err(NoSuchEntity);
        }
        if meta.location.archetype == 0 {
            return Ok(Location {
                archetype: 0,
                index: usize::max_value(),
            });
        }
        Ok(meta.location)
    }

    /// Allocate space for and enumerate pending entities
    #[allow(clippy::reversed_empty_ranges)]
    pub fn flush(&mut self) -> impl Iterator<Item = u32> {
        let pending = self.pending.load(Ordering::Relaxed); // Not racey due to &mut self
        if pending != 0 {
            let first = self.meta.len() as u32;
            self.grow(0);
            first..(first + pending)
        } else {
            0..0
        }
    }

    // The following three methods allow iteration over `reserved` simultaneous to location
    // writes. This is a lazy hack, but we only use it in `World::flush` so the complexity and unsafety
    // involved in producing an `impl Iterator<Item=(u32, &mut Location)>` isn't a clear win.
    pub fn reserved_len(&mut self) -> u32 {
        self.reserved_cursor - self.free_cursor.load(Ordering::Relaxed)
    }

    pub fn reserved(&mut self, i: u32) -> u32 {
        debug_assert!(i < self.reserved_len());
        let free_cursor = self.free_cursor.load(Ordering::Relaxed) as usize;
        self.unused[free_cursor + i as usize]
    }

    pub fn clear_reserved(&mut self) {
        let free_cursor = self.free_cursor.load(Ordering::Relaxed); // Not racey due to &mut self
        self.reserved_cursor = free_cursor;
    }

    /// Expand storage and mark all but the first `pending` of the new slots as free
    fn grow(&mut self, increment: u32) {
        let pending = self.pending.swap(0, Ordering::Relaxed);
        let new_len = (self.meta.len() + pending as usize + increment as usize)
            .max(self.meta.len() * 2)
            .max(1024);
        let mut new_meta = Vec::with_capacity(new_len);
        new_meta.extend_from_slice(&self.meta);
        new_meta.resize(
            new_len,
            EntityMeta {
                generation: 0,
                location: Location {
                    archetype: 0,
                    index: usize::max_value(), // dummy value, to be filled in
                },
            },
        );

        let free_cursor = self.free_cursor.load(Ordering::Relaxed); // Not racey due to &mut self
        let reserved_cursor = self.reserved_cursor; // Not racey due to &mut self

        let mut new_unused = Vec::with_capacity(new_len);

        // Add freshly allocated trailing free slots. List them first since they are
        // higher-numbered than any existing freelist entries, meaning we'll pop them last.
        new_unused.extend(((self.meta.len() as u32 + pending)..new_len as u32).rev());

        // Insert the original freelist, if any.
        new_unused.extend_from_slice(&self.unused[0..free_cursor as usize]);

        let new_free_cursor = new_unused.len() as u32;
        self.free_cursor.store(new_free_cursor, Ordering::Relaxed); // Not racey due to &mut self

        // Preserve any reserved values.
        new_unused.extend_from_slice(&self.unused[free_cursor as usize..reserved_cursor as usize]);
        self.reserved_cursor = new_unused.len() as u32;

        debug_assert!(new_unused.len() <= new_len);

        // Zero-fill. This gives us enough room to freelist every ID in existence.
        new_unused.resize(new_len, 0);

        self.meta = new_meta;
        self.unused = new_unused;
    }

    pub fn get_reserver(&self) -> EntityReserver {
        // SAFE: reservers use atomics for anything write-related
        let entities: &'static Entities = unsafe { mem::transmute(self) };
        EntityReserver { entities }
    }
}

/// Reserves entities in a way that is usable in multi-threaded contexts.
#[derive(Debug)]
pub struct EntityReserver {
    entities: &'static Entities,
}

impl EntityReserver {
    /// Reserves an entity
    pub fn reserve_entity(&self) -> Entity {
        self.entities.reserve_entity()
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct EntityMeta {
    pub generation: u32,
    pub location: Location,
}

/// A location of an entity in an archetype
#[derive(Copy, Clone, Debug)]
pub struct Location {
    /// The archetype index
    pub archetype: u32,

    /// The index of the entity in the archetype
    pub index: usize,
}

/// Error indicating that no entity with a particular ID exists
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoSuchEntity;

impl fmt::Display for NoSuchEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("no such entity")
    }
}

#[cfg(feature = "std")]
impl Error for NoSuchEntity {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn entity_bits_roundtrip() {
        let e = Entity {
            generation: 0xDEADBEEF,
            id: 0xBAADF00D,
        };
        assert_eq!(Entity::from_bits(e.to_bits()), e);
    }

    #[test]
    fn alloc_and_free() {
        let mut rng = rand::thread_rng();

        let mut e = Entities::default();
        let mut first_unused = 0u32;
        let mut id_to_gen: HashMap<u32, u32> = Default::default();
        let mut free_set: HashSet<u32> = Default::default();

        for _ in 0..100000 {
            let alloc = rng.gen_range(0, 3) != 0;
            if alloc || first_unused == 0 {
                let entity = e.alloc();

                let id = entity.id;
                if !free_set.is_empty() {
                    // This should have come from the freelist.
                    assert!(free_set.remove(&id));
                } else if id >= first_unused {
                    first_unused = id + 1;
                }

                e.get_mut(entity).unwrap().index = 37;

                assert!(id_to_gen.insert(id, entity.generation).is_none());
            } else {
                // Free a random ID, whether or not it's in use, and check for errors.
                let id = rng.gen_range(0, first_unused);

                let generation = id_to_gen.remove(&id);
                let entity = Entity {
                    id,
                    generation: generation.unwrap_or(0),
                };

                assert_eq!(e.free(entity).is_ok(), generation.is_some());

                free_set.insert(id);
            }
        }
    }

    #[test]
    fn reserve_entity() {
        let mut e = Entities::default();

        // Allocate and ignore a bunch of items to mostly drain the initial freelist.
        e.grow(0);
        let skip = e.free_cursor.load(Ordering::Relaxed) - 10;
        let _v0: Vec<Entity> = (0..skip).map(|_| e.alloc()).collect();

        // Allocate 10 items.
        let mut v1: Vec<Entity> = (0..10).map(|_| e.alloc()).collect();
        assert_eq!(v1.iter().map(|e| e.id).max(), Some(1023));
        for &entity in v1.iter() {
            e.get_mut(entity).unwrap().index = 37;
        }

        // Put the last 4 on the freelist.
        for entity in v1.drain(6..) {
            e.free(entity).unwrap();
        }
        assert_eq!(e.free_cursor.load(Ordering::Relaxed), 4);

        // Allocate 10 entities, so 4 will come from the freelist.
        // This means we will have allocated 10 + 10 - 4 total items, so max id is 15.
        let v2: Vec<Entity> = (0..10).map(|_| e.reserve_entity()).collect();
        assert_eq!(v2.iter().map(|e| e.id).max(), Some(skip + 15));

        // We should have exactly IDs skip..skip+16.
        let mut v3: Vec<Entity> = v1.iter().chain(v2.iter()).copied().collect();
        assert_eq!(v3.len(), 16);
        v3.sort_by_key(|entity| entity.id);
        for (i, entity) in v3.into_iter().enumerate() {
            assert_eq!(entity.id, skip + i as u32);
        }

        // 6 will come from pending.
        assert_eq!(e.pending.load(Ordering::Relaxed), 6);
        assert_eq!(e.flush().count(), 6);
    }
}
