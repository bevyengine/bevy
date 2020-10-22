use alloc::{boxed::Box, vec::Vec};
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
    // Unused entity IDs below `meta.len()`
    free: Vec<u32>,
    free_cursor: AtomicU32,
    // Reserved IDs within `meta.len()` with implicit archetype 0 and undefined index. Should be
    // consumed and used to initialize locations to produce real entities after calling `flush`.
    reserved: Box<[AtomicU32]>,
    reserved_cursor: AtomicU32,
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
                // The freelist has entities in it, so move the last entry to the reserved list, to
                // be consumed by the caller as part of a higher-level flush.
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
                    let id = self.free[next as usize];
                    let reservation = self.reserved_cursor.fetch_add(1, Ordering::Relaxed);
                    self.reserved[reservation as usize].store(id, Ordering::Relaxed);
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
        let index = self.free_cursor.load(Ordering::Relaxed);
        match index.checked_sub(1) {
            None => {
                self.grow(0);
                let cursor = self.free_cursor.fetch_sub(1, Ordering::Relaxed);
                let id = self.free[(cursor - 1) as usize];
                Entity {
                    generation: self.meta[id as usize].generation,
                    id,
                }
            }
            Some(next) => {
                // Not racey due to &mut self
                self.free_cursor.store(next, Ordering::Relaxed);
                let id = self.free[next as usize];
                Entity {
                    generation: self.meta[id as usize].generation,
                    id,
                }
            }
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
        self.free[index as usize] = entity.id;
        debug_assert!(
            loc.index != usize::max_value(),
            "free called on reserved entity without flush"
        );
        Ok(loc)
    }

    /// Ensure `n` at least allocations can succeed without reallocating
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
        self.free_cursor
            .store(self.meta.len() as u32, Ordering::Relaxed);
        for (i, x) in self.free.iter_mut().enumerate() {
            *x = i as u32;
        }
        self.pending.store(0, Ordering::Relaxed);
        self.reserved_cursor.store(0, Ordering::Relaxed);
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
    pub fn reserved_len(&self) -> u32 {
        self.reserved_cursor.load(Ordering::Relaxed)
    }

    pub fn reserved(&self, i: u32) -> u32 {
        debug_assert!(i < self.reserved_len());
        self.reserved[i as usize].load(Ordering::Relaxed)
    }

    pub fn clear_reserved(&mut self) {
        self.reserved_cursor.store(0, Ordering::Relaxed);
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
        let mut new_free = Vec::with_capacity(new_len);
        new_free.extend_from_slice(&self.free[0..free_cursor as usize]);
        // Add freshly allocated trailing free slots
        new_free.extend(((self.meta.len() as u32 + pending)..new_len as u32).rev());
        debug_assert!(new_free.len() <= new_len);
        self.free_cursor
            .store(new_free.len() as u32, Ordering::Relaxed); // Not racey due to &mut self

        // Zero-fill
        new_free.resize(new_len, 0);

        self.meta = new_meta;
        self.free = new_free;
        let mut new_reserved = Vec::with_capacity(new_len);
        // Not racey due to &mut self
        let reserved_cursor = self.reserved_cursor.load(Ordering::Relaxed);
        for x in &self.reserved[..reserved_cursor as usize] {
            new_reserved.push(AtomicU32::new(x.load(Ordering::Relaxed)));
        }
        new_reserved.resize_with(new_len, || AtomicU32::new(0));
        self.reserved = new_reserved.into();
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

    #[test]
    fn entity_bits_roundtrip() {
        let e = Entity {
            generation: 0xDEADBEEF,
            id: 0xBAADF00D,
        };
        assert_eq!(Entity::from_bits(e.to_bits()), e);
    }
}
