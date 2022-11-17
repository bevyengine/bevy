//! Entity handling types.
//!
//! An **entity** exclusively owns zero or more [component] instances, all of different types, and can dynamically acquire or lose them over its lifetime.
//!
//! See [`Entity`] to learn more.
//!
//! [component]: crate::component::Component
//!
//! # Usage
//!
//! Operations involving entities and their components are performed either from a system by submitting commands,
//! or from the outside (or from an exclusive system) by directly using [`World`] methods:
//!
//! |Operation|Command|Method|
//! |:---:|:---:|:---:|
//! |Spawn an entity with components|[`Commands::spawn`]|---|
//! |Spawn an entity without components|[`Commands::spawn_empty`]|[`World::spawn_empty`]|
//! |Despawn an entity|[`EntityCommands::despawn`]|[`World::despawn`]|
//! |Insert a component, bundle, or tuple of components and bundles to an entity|[`EntityCommands::insert`]|[`EntityMut::insert`]|
//! |Remove a component, bundle, or tuple of components and bundles from an entity|[`EntityCommands::remove`]|[`EntityMut::remove`]|
//!
//! [`World`]: crate::world::World
//! [`Commands::spawn`]: crate::system::Commands::spawn
//! [`Commands::spawn_empty`]: crate::system::Commands::spawn_empty
//! [`EntityCommands::despawn`]: crate::system::EntityCommands::despawn
//! [`EntityCommands::insert`]: crate::system::EntityCommands::insert
//! [`EntityCommands::remove`]: crate::system::EntityCommands::remove
//! [`World::spawn`]: crate::world::World::spawn
//! [`World::spawn_empty`]: crate::world::World::spawn_empty
//! [`World::despawn`]: crate::world::World::despawn
//! [`EntityMut::insert`]: crate::world::EntityMut::insert
//! [`EntityMut::remove`]: crate::world::EntityMut::remove
mod map_entities;

pub use map_entities::*;

use crate::{archetype::ArchetypeId, storage::SparseSetIndex};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt, mem, sync::atomic::Ordering};

#[cfg(target_has_atomic = "64")]
use std::sync::atomic::AtomicI64 as AtomicIdCursor;
#[cfg(target_has_atomic = "64")]
type IdCursor = i64;

/// Most modern platforms support 64-bit atomics, but some less-common platforms
/// do not. This fallback allows compilation using a 32-bit cursor instead, with
/// the caveat that some conversions may fail (and panic) at runtime.
#[cfg(not(target_has_atomic = "64"))]
use std::sync::atomic::AtomicIsize as AtomicIdCursor;
#[cfg(not(target_has_atomic = "64"))]
type IdCursor = isize;

/// Lightweight identifier of an [entity](crate::entity).
///
/// The identifier is implemented using a [generational index]: a combination of an index and a generation.
/// This allows fast insertion after data removal in an array while minimizing loss of spatial locality.
///
/// [generational index]: https://lucassardois.medium.com/generational-indices-guide-8e3c5f7fd594
///
/// # Usage
///
/// This data type is returned by iterating a `Query` that has `Entity` as part of its query fetch type parameter ([learn more]).
/// It can also be obtained by calling [`EntityCommands::id`] or [`EntityMut::id`].
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct SomeComponent;
/// fn setup(mut commands: Commands) {
///     // Calling `spawn` returns `EntityCommands`.
///     let entity = commands.spawn(SomeComponent).id();
/// }
///
/// fn exclusive_system(world: &mut World) {
///     // Calling `spawn` returns `EntityMut`.
///     let entity = world.spawn(SomeComponent).id();
/// }
/// #
/// # bevy_ecs::system::assert_is_system(setup);
/// # bevy_ecs::system::assert_is_system(exclusive_system);
/// ```
///
/// It can be used to refer to a specific entity to apply [`EntityCommands`], or to call [`Query::get`] (or similar methods) to access its components.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct Expired;
/// #
/// fn dispose_expired_food(mut commands: Commands, query: Query<Entity, With<Expired>>) {
///     for food_entity in &query {
///         commands.entity(food_entity).despawn();
///     }
/// }
/// #
/// # bevy_ecs::system::assert_is_system(dispose_expired_food);
/// ```
///
/// [learn more]: crate::system::Query#entity-id-access
/// [`EntityCommands::id`]: crate::system::EntityCommands::id
/// [`EntityMut::id`]: crate::world::EntityMut::id
/// [`EntityCommands`]: crate::system::EntityCommands
/// [`Query::get`]: crate::system::Query::get
#[derive(Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Entity {
    pub(crate) generation: u32,
    pub(crate) index: u32,
}

pub enum AllocAtWithoutReplacement {
    Exists(EntityLocation),
    DidNotExist,
    ExistsWithWrongGeneration,
}

impl Entity {
    /// Creates a new entity reference with the specified `index` and a generation of 0.
    ///
    /// # Note
    ///
    /// Spawning a specific `entity` value is __rarely the right choice__. Most apps should favor
    /// [`Commands::spawn`](crate::system::Commands::spawn). This method should generally
    /// only be used for sharing entities across apps, and only when they have a scheme
    /// worked out to share an index space (which doesn't happen by default).
    ///
    /// In general, one should not try to synchronize the ECS by attempting to ensure that
    /// `Entity` lines up between instances, but instead insert a secondary identifier as
    /// a component.
    ///
    /// There are still some use cases where it might be appropriate to use this function
    /// externally.
    ///
    /// ## Examples
    ///
    /// Initializing a collection (e.g. `array` or `Vec`) with a known size:
    ///
    /// ```no_run
    /// # use bevy_ecs::prelude::*;
    /// // Create a new array of size 10 and initialize it with (invalid) entities.
    /// let mut entities: [Entity; 10] = [Entity::from_raw(0); 10];
    ///
    /// // ... replace the entities with valid ones.
    /// ```
    ///
    /// Deriving `Reflect` for a component that has an `Entity` field:
    ///
    /// ```no_run
    /// # use bevy_ecs::{prelude::*, component::*};
    /// # use bevy_reflect::Reflect;
    /// #[derive(Reflect, Component)]
    /// #[reflect(Component)]
    /// pub struct MyStruct {
    ///     pub entity: Entity,
    /// }
    ///
    /// impl FromWorld for MyStruct {
    ///     fn from_world(_world: &mut World) -> Self {
    ///         Self {
    ///             entity: Entity::from_raw(u32::MAX),
    ///         }
    ///     }
    /// }
    /// ```
    pub const fn from_raw(index: u32) -> Entity {
        Entity {
            index,
            generation: 0,
        }
    }

    /// Convert to a form convenient for passing outside of rust.
    ///
    /// Only useful for identifying entities within the same instance of an application. Do not use
    /// for serialization between runs.
    ///
    /// No particular structure is guaranteed for the returned bits.
    pub const fn to_bits(self) -> u64 {
        (self.generation as u64) << 32 | self.index as u64
    }

    /// Reconstruct an `Entity` previously destructured with [`Entity::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    pub const fn from_bits(bits: u64) -> Self {
        Self {
            generation: (bits >> 32) as u32,
            index: bits as u32,
        }
    }

    /// Return a transiently unique identifier.
    ///
    /// No two simultaneously-live entities share the same index, but dead entities' indices may collide
    /// with both live and dead entities. Useful for compactly representing entities within a
    /// specific snapshot of the world, such as when serializing.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Returns the generation of this Entity's index. The generation is incremented each time an
    /// entity with a given index is despawned. This serves as a "count" of the number of times a
    /// given index has been reused (index, generation) pairs uniquely identify a given Entity.
    #[inline]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.index, self.generation)
    }
}

impl SparseSetIndex for Entity {
    fn sparse_set_index(&self) -> usize {
        self.index() as usize
    }

    fn get_sparse_set_index(value: usize) -> Self {
        Entity::from_raw(value as u32)
    }
}

/// An [`Iterator`] returning a sequence of [`Entity`] values from
/// [`Entities::reserve_entities`](crate::entity::Entities::reserve_entities).
pub struct ReserveEntitiesIterator<'a> {
    // Metas, so we can recover the current generation for anything in the freelist.
    meta: &'a [EntityMeta],

    // Reserved indices formerly in the freelist to hand out.
    index_iter: std::slice::Iter<'a, u32>,

    // New Entity indices to hand out, outside the range of meta.len().
    index_range: std::ops::Range<u32>,
}

impl<'a> Iterator for ReserveEntitiesIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.index_iter
            .next()
            .map(|&index| Entity {
                generation: self.meta[index as usize].generation,
                index,
            })
            .or_else(|| {
                self.index_range.next().map(|index| Entity {
                    generation: 0,
                    index,
                })
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.index_iter.len() + self.index_range.len();
        (len, Some(len))
    }
}

impl<'a> core::iter::ExactSizeIterator for ReserveEntitiesIterator<'a> {}
impl<'a> core::iter::FusedIterator for ReserveEntitiesIterator<'a> {}

#[derive(Debug, Default)]
pub struct Entities {
    pub(crate) meta: Vec<EntityMeta>,

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
    free_cursor: AtomicIdCursor,
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
        let range_end = self
            .free_cursor
            // Unwrap: these conversions can only fail on platforms that don't support 64-bit atomics
            // and use AtomicIsize instead (see note on `IdCursor`).
            .fetch_sub(IdCursor::try_from(count).unwrap(), Ordering::Relaxed);
        let range_start = range_end - IdCursor::try_from(count).unwrap();

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
            let base = self.meta.len() as IdCursor;

            let new_id_end = u32::try_from(base - range_start).expect("too many entities");

            // `new_id_end` is in range, so no need to check `start`.
            let new_id_start = (base - range_end.min(0)) as u32;

            (new_id_start, new_id_end)
        };

        ReserveEntitiesIterator {
            meta: &self.meta[..],
            index_iter: self.pending[freelist_range].iter(),
            index_range: new_id_start..new_id_end,
        }
    }

    /// Reserve one entity ID concurrently.
    ///
    /// Equivalent to `self.reserve_entities(1).next().unwrap()`, but more efficient.
    pub fn reserve_entity(&self) -> Entity {
        let n = self.free_cursor.fetch_sub(1, Ordering::Relaxed);
        if n > 0 {
            // Allocate from the freelist.
            let index = self.pending[(n - 1) as usize];
            Entity {
                generation: self.meta[index as usize].generation,
                index,
            }
        } else {
            // Grab a new ID, outside the range of `meta.len()`. `flush()` must
            // eventually be called to make it valid.
            //
            // As `self.free_cursor` goes more and more negative, we return IDs farther
            // and farther beyond `meta.len()`.
            Entity {
                generation: 0,
                index: u32::try_from(self.meta.len() as IdCursor - n).expect("too many entities"),
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
        if let Some(index) = self.pending.pop() {
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            Entity {
                generation: self.meta[index as usize].generation,
                index,
            }
        } else {
            let index = u32::try_from(self.meta.len()).expect("too many entities");
            self.meta.push(EntityMeta::EMPTY);
            Entity {
                generation: 0,
                index,
            }
        }
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any. Location should be
    /// written immediately.
    pub fn alloc_at(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.verify_flushed();

        let loc = if entity.index as usize >= self.meta.len() {
            self.pending.extend((self.meta.len() as u32)..entity.index);
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.meta
                .resize(entity.index as usize + 1, EntityMeta::EMPTY);
            self.len += 1;
            None
        } else if let Some(index) = self.pending.iter().position(|item| *item == entity.index) {
            self.pending.swap_remove(index);
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.len += 1;
            None
        } else {
            Some(mem::replace(
                &mut self.meta[entity.index as usize].location,
                EntityMeta::EMPTY.location,
            ))
        };

        self.meta[entity.index as usize].generation = entity.generation;

        loc
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any.
    pub fn alloc_at_without_replacement(&mut self, entity: Entity) -> AllocAtWithoutReplacement {
        self.verify_flushed();

        let result = if entity.index as usize >= self.meta.len() {
            self.pending.extend((self.meta.len() as u32)..entity.index);
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.meta
                .resize(entity.index as usize + 1, EntityMeta::EMPTY);
            self.len += 1;
            AllocAtWithoutReplacement::DidNotExist
        } else if let Some(index) = self.pending.iter().position(|item| *item == entity.index) {
            self.pending.swap_remove(index);
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.len += 1;
            AllocAtWithoutReplacement::DidNotExist
        } else {
            let current_meta = &mut self.meta[entity.index as usize];
            if current_meta.location.archetype_id == ArchetypeId::INVALID {
                AllocAtWithoutReplacement::DidNotExist
            } else if current_meta.generation == entity.generation {
                AllocAtWithoutReplacement::Exists(current_meta.location)
            } else {
                return AllocAtWithoutReplacement::ExistsWithWrongGeneration;
            }
        };

        self.meta[entity.index as usize].generation = entity.generation;
        result
    }

    /// Destroy an entity, allowing it to be reused.
    ///
    /// Must not be called while reserved entities are awaiting `flush()`.
    pub fn free(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.verify_flushed();

        let meta = &mut self.meta[entity.index as usize];
        if meta.generation != entity.generation {
            return None;
        }
        meta.generation += 1;

        let loc = mem::replace(&mut meta.location, EntityMeta::EMPTY.location);

        self.pending.push(entity.index);

        let new_free_cursor = self.pending.len() as IdCursor;
        *self.free_cursor.get_mut() = new_free_cursor;
        self.len -= 1;
        Some(loc)
    }

    /// Ensure at least `n` allocations can succeed without reallocating.
    pub fn reserve(&mut self, additional: u32) {
        self.verify_flushed();

        let freelist_size = *self.free_cursor.get_mut();
        // Unwrap: these conversions can only fail on platforms that don't support 64-bit atomics
        // and use AtomicIsize instead (see note on `IdCursor`).
        let shortfall = IdCursor::try_from(additional).unwrap() - freelist_size;
        if shortfall > 0 {
            self.meta.reserve(shortfall as usize);
        }
    }

    /// Returns true if the [`Entities`] contains [`entity`](Entity).
    // This will return false for entities which have been freed, even if
    // not reallocated since the generation is incremented in `free`
    pub fn contains(&self, entity: Entity) -> bool {
        self.resolve_from_id(entity.index())
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
        if (entity.index as usize) < self.meta.len() {
            let meta = &self.meta[entity.index as usize];
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
    pub fn resolve_from_id(&self, index: u32) -> Option<Entity> {
        let idu = index as usize;
        if let Some(&EntityMeta { generation, .. }) = self.meta.get(idu) {
            Some(Entity { generation, index })
        } else {
            // `id` is outside of the meta list - check whether it is reserved but not yet flushed.
            let free_cursor = self.free_cursor.load(Ordering::Relaxed);
            // If this entity was manually created, then free_cursor might be positive
            // Returning None handles that case correctly
            let num_pending = usize::try_from(-free_cursor).ok()?;
            (idu < self.meta.len() + num_pending).then_some(Entity {
                generation: 0,
                index,
            })
        }
    }

    fn needs_flush(&mut self) -> bool {
        *self.free_cursor.get_mut() != self.pending.len() as IdCursor
    }

    /// Allocates space for entities previously reserved with `reserve_entity` or
    /// `reserve_entities`, then initializes each one using the supplied function.
    ///
    /// # Safety
    /// Flush _must_ set the entity location to the correct [`ArchetypeId`] for the given [`Entity`]
    /// each time init is called. This _can_ be [`ArchetypeId::INVALID`], provided the [`Entity`]
    /// has not been assigned to an [`Archetype`][crate::archetype::Archetype].
    ///
    /// Note: freshly-allocated entities (ones which don't come from the pending list) are guaranteed
    /// to be initialized with the invalid archetype.
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
            for (index, meta) in self.meta.iter_mut().enumerate().skip(old_meta_len) {
                init(
                    Entity {
                        index: index as u32,
                        generation: meta.generation,
                    },
                    &mut meta.location,
                );
            }

            *free_cursor = 0;
            0
        };

        self.len += (self.pending.len() - new_free_cursor) as u32;
        for index in self.pending.drain(new_free_cursor..) {
            let meta = &mut self.meta[index as usize];
            init(
                Entity {
                    index,
                    generation: meta.generation,
                },
                &mut meta.location,
            );
        }
    }

    // Flushes all reserved entities to an "invalid" state. Attempting to retrieve them will return None
    // unless they are later populated with a valid archetype.
    pub fn flush_as_invalid(&mut self) {
        // SAFETY: as per `flush` safety docs, the archetype id can be set to [`ArchetypeId::INVALID`] if
        // the [`Entity`] has not been assigned to an [`Archetype`][crate::archetype::Archetype], which is the case here
        unsafe {
            self.flush(|_entity, location| {
                location.archetype_id = ArchetypeId::INVALID;
            });
        }
    }

    /// # Safety
    ///
    /// This function is safe if and only if the world this Entities is on has no entities.
    pub unsafe fn flush_and_reserve_invalid_assuming_no_entities(&mut self, count: usize) {
        let free_cursor = self.free_cursor.get_mut();
        *free_cursor = 0;
        self.meta.reserve(count);
        // the EntityMeta struct only contains integers, and it is valid to have all bytes set to u8::MAX
        self.meta.as_mut_ptr().write_bytes(u8::MAX, count);
        self.meta.set_len(count);

        self.len = count as u32;
    }

    /// Accessor for getting the length of the vec in `self.meta`
    #[inline]
    pub fn meta_len(&self) -> usize {
        self.meta.len()
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

// Safety:
// This type must not contain any pointers at any level, and be safe to fully fill with u8::MAX.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
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
#[repr(C)]
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
            index: 0xBAADF00D,
        };
        assert_eq!(Entity::from_bits(e.to_bits()), e);
    }

    #[test]
    fn reserve_entity_len() {
        let mut e = Entities::default();
        e.reserve_entity();
        // SAFETY: entity_location is left invalid
        unsafe { e.flush(|_, _| {}) };
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn get_reserved_and_invalid() {
        let mut entities = Entities::default();
        let e = entities.reserve_entity();
        assert!(entities.contains(e));
        assert!(entities.get(e).is_none());

        // SAFETY: entity_location is left invalid
        unsafe {
            entities.flush(|_entity, _location| {
                // do nothing ... leaving entity location invalid
            });
        };

        assert!(entities.contains(e));
        assert!(entities.get(e).is_none());
    }

    #[test]
    fn entity_const() {
        const C1: Entity = Entity::from_raw(42);
        assert_eq!(42, C1.index);
        assert_eq!(0, C1.generation);

        const C2: Entity = Entity::from_bits(0x0000_00ff_0000_00cc);
        assert_eq!(0x0000_00cc, C2.index);
        assert_eq!(0x0000_00ff, C2.generation);

        const C3: u32 = Entity::from_raw(33).index();
        assert_eq!(33, C3);

        const C4: u32 = Entity::from_bits(0x00dd_00ff_0000_0000).generation();
        assert_eq!(0x00dd_00ff, C4);
    }
}
