//! Entity handling types.
//!
//! In Bevy ECS, there is no monolithic data structure for an entity. Instead, the [`Entity`]
//! `struct` is just a *generational index* (a combination of an ID and a generation). Then,
//! the `Entity` maps to the specific [`Component`s](crate::component::Component). This way,
//! entities can have meaningful data attached to it. This is a fundamental design choice
//! that has been taken to enhance performance and usability.
//!
//! # Usage
//!
//! Here are links to the methods used to perform common operations
//! involving entities:
//!
//! - **Spawning an empty entity:** use [`Commands::spawn`](crate::system::Commands::spawn).
//! - **Spawning an entity with components:** use
//!   [`Commands::spawn_bundle`](crate::system::Commands::spawn_bundle).
//! - **Despawning an entity:** use
//!   [`EntityCommands::despawn`](crate::system::EntityCommands::despawn).
//! - **Inserting a component to an entity:** use
//!   [`EntityCommands::insert`](crate::system::EntityCommands::insert).
//! - **Adding multiple components to an entity:** use
//!   [`EntityCommands::insert_bundle`](crate::system::EntityCommands::insert_bundle).
//! - **Removing a component to an entity:** use
//!   [`EntityCommands::remove`](crate::system::EntityCommands::remove).
mod map_entities;
mod serde;

pub use self::serde::*;
pub use map_entities::*;

use crate::{archetype::ArchetypeId, storage::SparseSetIndex};
use std::{
    convert::TryFrom,
    fmt, mem,
    sync::atomic::{AtomicI64, Ordering},
};

/// Lightweight unique ID of an entity.
///
/// Obtained from [`World::spawn`](crate::world::World::spawn), typically via
/// [`Commands::spawn`](crate::system::Commands::spawn). Can be stored to refer to an entity in the
/// future.
///
/// `Entity` can be a part of a query, e.g. `Query<(Entity, &MyComponent)>`.
/// Components of a specific entity can be accessed using
/// [`Query::get`](crate::system::Query::get) and related methods.
#[derive(Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity {
    pub(crate) generation: u32,
    pub(crate) id: u32,
}

pub enum AllocAtWithoutReplacement {
    Exists(EntityLocation),
    DidNotExist,
    ExistsWithWrongGeneration,
}

impl Entity {
    /// Creates a new entity reference with a generation of 0.
    ///
    /// # Note
    ///
    /// Spawning a specific `entity` value is rarely the right choice. Most apps should favor
    /// [`Commands::spawn`](crate::system::Commands::spawn). This method should generally
    /// only be used for sharing entities across apps, and only when they have a scheme
    /// worked out to share an ID space (which doesn't happen by default).
    pub fn new(id: u32) -> Entity {
        Entity { id, generation: 0 }
    }

    /// Convert to a form convenient for passing outside of rust.
    ///
    /// Only useful for identifying entities within the same instance of an application. Do not use
    /// for serialization between runs.
    ///
    /// No particular structure is guaranteed for the returned bits.
    pub fn to_bits(self) -> u64 {
        u64::from(self.generation) << 32 | u64::from(self.id)
    }

    /// Reconstruct an `Entity` previously destructured with [`Entity::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    pub fn from_bits(bits: u64) -> Self {
        Self {
            generation: (bits >> 32) as u32,
            id: bits as u32,
        }
    }

    /// Return a transiently unique identifier.
    ///
    /// No two simultaneously-live entities share the same ID, but dead entities' IDs may collide
    /// with both live and dead entities. Useful for compactly representing entities within a
    /// specific snapshot of the world, such as when serializing.
    #[inline]
    pub fn id(self) -> u32 {
        self.id
    }

    /// Returns the generation of this Entity's id. The generation is incremented each time an
    /// entity with a given id is despawned. This serves as a "count" of the number of times a
    /// given id has been reused (id, generation) pairs uniquely identify a given Entity.
    #[inline]
    pub fn generation(self) -> u32 {
        self.generation
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.id, self.generation)
    }
}

impl SparseSetIndex for Entity {
    fn sparse_set_index(&self) -> usize {
        self.id() as usize
    }

    fn get_sparse_set_index(value: usize) -> Self {
        Entity::new(value as u32)
    }
}

/// An [`Iterator`] returning a sequence of [`Entity`] values from
/// [`Entities::reserve_entities`](crate::entity::Entities::reserve_entities).
pub struct ReserveEntitiesIterator<'a> {
    // Metas, so we can recover the current generation for anything in the freelist.
    meta: &'a [EntityMeta],

    // Reserved IDs formerly in the freelist to hand out.
    id_iter: std::slice::Iter<'a, u32>,

    // New Entity IDs to hand out, outside the range of meta.len().
    id_range: std::ops::Range<u32>,
}

impl<'a> Iterator for ReserveEntitiesIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.id_iter
            .next()
            .map(|&id| Entity {
                generation: self.meta[id as usize].generation,
                id,
            })
            .or_else(|| self.id_range.next().map(|id| Entity { generation: 0, id }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.id_iter.len() + self.id_range.len();
        (len, Some(len))
    }
}

impl<'a> core::iter::ExactSizeIterator for ReserveEntitiesIterator<'a> {}

#[derive(Debug, Default)]
pub struct Entities {
    pub meta: Vec<EntityMeta>,

    /// The `pending` and `free_cursor` fields describe three sets of Entity IDs
    /// that have been freed or are in the process of being allocated:
    ///
    /// - The `freelist` IDs, previously freed by `free()`. These IDs are available to any of
    ///   `alloc()`, `reserve_entity()` or `reserve_entities()`. Allocation will always prefer
    ///   these over brand new IDs.
    ///
    /// - The `reserved` list of IDs that were once in the freelist, but got reserved by
    ///   `reserve_entities` or `reserve_entity()`. They are now waiting for `flush()` to make them
    ///   fully allocated.
    ///
    /// - The count of new IDs that do not yet exist in `self.meta()`, but which we have handed out
    ///   and reserved. `flush()` will allocate room for them in `self.meta()`.
    ///
    /// The contents of `pending` look like this:
    ///
    /// ```txt
    /// ----------------------------
    /// |  freelist  |  reserved   |
    /// ----------------------------
    ///              ^             ^
    ///          free_cursor   pending.len()
    /// ```
    ///
    /// As IDs are allocated, `free_cursor` is atomically decremented, moving
    /// items from the freelist into the reserved list by sliding over the boundary.
    ///
    /// Once the freelist runs out, `free_cursor` starts going negative.
    /// The more negative it is, the more IDs have been reserved starting exactly at
    /// the end of `meta.len()`.
    ///
    /// This formulation allows us to reserve any number of IDs first from the freelist
    /// and then from the new IDs, using only a single atomic subtract.
    ///
    /// Once `flush()` is done, `free_cursor` will equal `pending.len()`.
    pending: Vec<u32>,
    free_cursor: AtomicI64,
    /// Stores the number of free entities for [`len`](Entities::len)
    len: u32,
}

impl Entities {
    /// Reserve entity IDs concurrently.
    ///
    /// Storage for entity generation and location is lazily allocated by calling `flush`.
    pub fn reserve_entities(&self, count: u32) -> ReserveEntitiesIterator {
        // Use one atomic subtract to grab a range of new IDs. The range might be
        // entirely nonnegative, meaning all IDs come from the freelist, or entirely
        // negative, meaning they are all new IDs to allocate, or a mix of both.
        let range_end = self.free_cursor.fetch_sub(count as i64, Ordering::Relaxed);
        let range_start = range_end - count as i64;

        let freelist_range = range_start.max(0) as usize..range_end.max(0) as usize;

        let (new_id_start, new_id_end) = if range_start >= 0 {
            // We satisfied all requests from the freelist.
            (0, 0)
        } else {
            // We need to allocate some new Entity IDs outside of the range of self.meta.
            //
            // `range_start` covers some negative territory, e.g. `-3..6`.
            // Since the nonnegative values `0..6` are handled by the freelist, that
            // means we need to handle the negative range here.
            //
            // In this example, we truncate the end to 0, leaving us with `-3..0`.
            // Then we negate these values to indicate how far beyond the end of `meta.end()`
            // to go, yielding `meta.len()+0 .. meta.len()+3`.
            let base = self.meta.len() as i64;

            let new_id_end = u32::try_from(base - range_start).expect("too many entities");

            // `new_id_end` is in range, so no need to check `start`.
            let new_id_start = (base - range_end.min(0)) as u32;

            (new_id_start, new_id_end)
        };

        ReserveEntitiesIterator {
            meta: &self.meta[..],
            id_iter: self.pending[freelist_range].iter(),
            id_range: new_id_start..new_id_end,
        }
    }

    /// Reserve one entity ID concurrently.
    ///
    /// Equivalent to `self.reserve_entities(1).next().unwrap()`, but more efficient.
    pub fn reserve_entity(&self) -> Entity {
        let n = self.free_cursor.fetch_sub(1, Ordering::Relaxed);
        if n > 0 {
            // Allocate from the freelist.
            let id = self.pending[(n - 1) as usize];
            Entity {
                generation: self.meta[id as usize].generation,
                id,
            }
        } else {
            // Grab a new ID, outside the range of `meta.len()`. `flush()` must
            // eventually be called to make it valid.
            //
            // As `self.free_cursor` goes more and more negative, we return IDs farther
            // and farther beyond `meta.len()`.
            Entity {
                generation: 0,
                id: u32::try_from(self.meta.len() as i64 - n).expect("too many entities"),
            }
        }
    }

    /// Check that we do not have pending work requiring `flush()` to be called.
    fn verify_flushed(&mut self) {
        debug_assert!(
            !self.needs_flush(),
            "flush() needs to be called before this operation is legal"
        );
    }

    /// Allocate an entity ID directly.
    pub fn alloc(&mut self) -> Entity {
        self.verify_flushed();
        self.len += 1;
        if let Some(id) = self.pending.pop() {
            let new_free_cursor = self.pending.len() as i64;
            *self.free_cursor.get_mut() = new_free_cursor;
            Entity {
                generation: self.meta[id as usize].generation,
                id,
            }
        } else {
            let id = u32::try_from(self.meta.len()).expect("too many entities");
            self.meta.push(EntityMeta::EMPTY);
            Entity { generation: 0, id }
        }
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any. Location should be
    /// written immediately.
    pub fn alloc_at(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.verify_flushed();

        let loc = if entity.id as usize >= self.meta.len() {
            self.pending.extend((self.meta.len() as u32)..entity.id);
            let new_free_cursor = self.pending.len() as i64;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.meta.resize(entity.id as usize + 1, EntityMeta::EMPTY);
            self.len += 1;
            None
        } else if let Some(index) = self.pending.iter().position(|item| *item == entity.id) {
            self.pending.swap_remove(index);
            let new_free_cursor = self.pending.len() as i64;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.len += 1;
            None
        } else {
            Some(mem::replace(
                &mut self.meta[entity.id as usize].location,
                EntityMeta::EMPTY.location,
            ))
        };

        self.meta[entity.id as usize].generation = entity.generation;

        loc
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any.
    pub fn alloc_at_without_replacement(&mut self, entity: Entity) -> AllocAtWithoutReplacement {
        self.verify_flushed();

        let result = if entity.id as usize >= self.meta.len() {
            self.pending.extend((self.meta.len() as u32)..entity.id);
            let new_free_cursor = self.pending.len() as i64;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.meta.resize(entity.id as usize + 1, EntityMeta::EMPTY);
            self.len += 1;
            AllocAtWithoutReplacement::DidNotExist
        } else if let Some(index) = self.pending.iter().position(|item| *item == entity.id) {
            self.pending.swap_remove(index);
            let new_free_cursor = self.pending.len() as i64;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.len += 1;
            AllocAtWithoutReplacement::DidNotExist
        } else {
            let current_meta = &mut self.meta[entity.id as usize];
            if current_meta.location.archetype_id == ArchetypeId::INVALID {
                AllocAtWithoutReplacement::DidNotExist
            } else if current_meta.generation == entity.generation {
                AllocAtWithoutReplacement::Exists(current_meta.location)
            } else {
                return AllocAtWithoutReplacement::ExistsWithWrongGeneration;
            }
        };

        self.meta[entity.id as usize].generation = entity.generation;
        result
    }

    /// Destroy an entity, allowing it to be reused.
    ///
    /// Must not be called while reserved entities are awaiting `flush()`.
    pub fn free(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.verify_flushed();

        let meta = &mut self.meta[entity.id as usize];
        if meta.generation != entity.generation {
            return None;
        }
        meta.generation += 1;

        let loc = mem::replace(&mut meta.location, EntityMeta::EMPTY.location);

        self.pending.push(entity.id);

        let new_free_cursor = self.pending.len() as i64;
        *self.free_cursor.get_mut() = new_free_cursor;
        self.len -= 1;
        Some(loc)
    }

    /// Ensure at least `n` allocations can succeed without reallocating.
    pub fn reserve(&mut self, additional: u32) {
        self.verify_flushed();

        let freelist_size = *self.free_cursor.get_mut();
        let shortfall = additional as i64 - freelist_size;
        if shortfall > 0 {
            self.meta.reserve(shortfall as usize);
        }
    }

    /// Returns true if the [`Entities`] contains [`entity`](Entity).
    // This will return false for entities which have been freed, even if
    // not reallocated since the generation is incremented in `free`
    pub fn contains(&self, entity: Entity) -> bool {
        self.resolve_from_id(entity.id())
            .map_or(false, |e| e.generation() == entity.generation)
    }

    pub fn clear(&mut self) {
        self.meta.clear();
        self.pending.clear();
        *self.free_cursor.get_mut() = 0;
        self.len = 0;
    }

    /// Returns `Ok(Location { archetype: Archetype::invalid(), index: undefined })` for pending entities.
    pub fn get(&self, entity: Entity) -> Option<EntityLocation> {
        if (entity.id as usize) < self.meta.len() {
            let meta = &self.meta[entity.id as usize];
            if meta.generation != entity.generation
                || meta.location.archetype_id == ArchetypeId::INVALID
            {
                return None;
            }
            Some(meta.location)
        } else {
            None
        }
    }

    /// Get the [`Entity`] with a given id, if it exists in this [`Entities`] collection
    /// Returns `None` if this [`Entity`] is outside of the range of currently reserved Entities
    ///
    /// Note: This method may return [`Entities`](Entity) which are currently free
    /// Note that [`contains`](Entities::contains) will correctly return false for freed
    /// entities, since it checks the generation
    pub fn resolve_from_id(&self, id: u32) -> Option<Entity> {
        let idu = id as usize;
        if let Some(&EntityMeta { generation, .. }) = self.meta.get(idu) {
            Some(Entity { generation, id })
        } else {
            // `id` is outside of the meta list - check whether it is reserved but not yet flushed.
            let free_cursor = self.free_cursor.load(Ordering::Relaxed);
            // If this entity was manually created, then free_cursor might be positive
            // Returning None handles that case correctly
            let num_pending = usize::try_from(-free_cursor).ok()?;
            (idu < self.meta.len() + num_pending).then(|| Entity { generation: 0, id })
        }
    }

    fn needs_flush(&mut self) -> bool {
        *self.free_cursor.get_mut() != self.pending.len() as i64
    }

    /// Allocates space for entities previously reserved with `reserve_entity` or
    /// `reserve_entities`, then initializes each one using the supplied function.
    ///
    /// # Safety
    /// Flush _must_ set the entity location to the correct ArchetypeId for the given Entity
    /// each time init is called. This _can_ be ArchetypeId::INVALID, provided the Entity has
    /// not been assigned to an Archetype.
    pub unsafe fn flush(&mut self, mut init: impl FnMut(Entity, &mut EntityLocation)) {
        let free_cursor = self.free_cursor.get_mut();
        let current_free_cursor = *free_cursor;

        let new_free_cursor = if current_free_cursor >= 0 {
            current_free_cursor as usize
        } else {
            let old_meta_len = self.meta.len();
            let new_meta_len = old_meta_len + -current_free_cursor as usize;
            self.meta.resize(new_meta_len, EntityMeta::EMPTY);
            self.len += -current_free_cursor as u32;
            for (id, meta) in self.meta.iter_mut().enumerate().skip(old_meta_len) {
                init(
                    Entity {
                        id: id as u32,
                        generation: meta.generation,
                    },
                    &mut meta.location,
                );
            }

            *free_cursor = 0;
            0
        };

        self.len += (self.pending.len() - new_free_cursor) as u32;
        for id in self.pending.drain(new_free_cursor..) {
            let meta = &mut self.meta[id as usize];
            init(
                Entity {
                    id,
                    generation: meta.generation,
                },
                &mut meta.location,
            );
        }
    }

    // Flushes all reserved entities to an "invalid" state. Attempting to retrieve them will return None
    // unless they are later populated with a valid archetype.
    pub fn flush_as_invalid(&mut self) {
        unsafe {
            self.flush(|_entity, location| {
                location.archetype_id = ArchetypeId::INVALID;
            })
        }
    }

    #[inline]
    pub fn len(&self) -> u32 {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EntityMeta {
    pub generation: u32,
    pub location: EntityLocation,
}

impl EntityMeta {
    const EMPTY: EntityMeta = EntityMeta {
        generation: 0,
        location: EntityLocation {
            archetype_id: ArchetypeId::INVALID,
            index: usize::MAX, // dummy value, to be filled in
        },
    };
}

/// A location of an entity in an archetype.
#[derive(Copy, Clone, Debug)]
pub struct EntityLocation {
    /// The archetype index
    pub archetype_id: ArchetypeId,

    /// The index of the entity in the archetype
    pub index: usize,
}

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

    #[test]
    fn reserve_entity_len() {
        let mut e = Entities::default();
        e.reserve_entity();
        unsafe { e.flush(|_, _| {}) };
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn get_reserved_and_invalid() {
        let mut entities = Entities::default();
        let e = entities.reserve_entity();
        assert!(entities.contains(e));
        assert!(entities.get(e).is_none());

        unsafe {
            entities.flush(|_entity, _location| {
                // do nothing ... leaving entity location invalid
            })
        };

        assert!(entities.contains(e));
        assert!(entities.get(e).is_none());
    }
}
