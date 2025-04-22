//! Entity handling types.
//!
//! An **entity** exclusively owns zero or more [component] instances, all of different types, and can dynamically acquire or lose them over its lifetime.
//!
//! **empty entity**: Entity with zero components.
//! **pending entity**: Entity reserved, but not flushed yet (see [`Entities::flush`] docs for reference).
//! **reserved entity**: same as **pending entity**.
//! **invalid entity**: **pending entity** flushed with invalid (see [`Entities::flush_as_invalid`] docs for reference).
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
//! |Spawn an entity with components|[`Commands::spawn`]|[`World::spawn`]|
//! |Spawn an entity without components|[`Commands::spawn_empty`]|[`World::spawn_empty`]|
//! |Despawn an entity|[`EntityCommands::despawn`]|[`World::despawn`]|
//! |Insert a component, bundle, or tuple of components and bundles to an entity|[`EntityCommands::insert`]|[`EntityWorldMut::insert`]|
//! |Remove a component, bundle, or tuple of components and bundles from an entity|[`EntityCommands::remove`]|[`EntityWorldMut::remove`]|
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
//! [`EntityWorldMut::insert`]: crate::world::EntityWorldMut::insert
//! [`EntityWorldMut::remove`]: crate::world::EntityWorldMut::remove

mod clone_entities;
mod entity_set;
mod map_entities;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

pub use clone_entities::*;
pub use entity_set::*;
pub use map_entities::*;

mod hash;
pub use hash::*;

pub mod hash_map;
pub mod hash_set;

pub use hash_map::EntityHashMap;
pub use hash_set::EntityHashSet;

pub mod index_map;
pub mod index_set;

pub use index_map::EntityIndexMap;
pub use index_set::EntityIndexSet;

pub mod unique_array;
pub mod unique_slice;
pub mod unique_vec;

pub use unique_array::{UniqueEntityArray, UniqueEntityEquivalentArray};
pub use unique_slice::{UniqueEntityEquivalentSlice, UniqueEntitySlice};
pub use unique_vec::{UniqueEntityEquivalentVec, UniqueEntityVec};

use crate::{
    archetype::{ArchetypeId, ArchetypeRow},
    change_detection::MaybeLocation,
    identifier::{
        error::IdentifierError,
        kinds::IdKind,
        masks::{IdentifierMask, HIGH_MASK},
        Identifier,
    },
    storage::{SparseSetIndex, TableId, TableRow},
};
use alloc::vec::Vec;
use bevy_platform::sync::atomic::Ordering;
use core::{fmt, hash::Hash, mem, num::NonZero, panic::Location};
use log::warn;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

#[cfg(target_has_atomic = "64")]
use bevy_platform::sync::atomic::AtomicI64 as AtomicIdCursor;
#[cfg(target_has_atomic = "64")]
type IdCursor = i64;

/// Most modern platforms support 64-bit atomics, but some less-common platforms
/// do not. This fallback allows compilation using a 32-bit cursor instead, with
/// the caveat that some conversions may fail (and panic) at runtime.
#[cfg(not(target_has_atomic = "64"))]
use bevy_platform::sync::atomic::AtomicIsize as AtomicIdCursor;
#[cfg(not(target_has_atomic = "64"))]
type IdCursor = isize;

/// Lightweight identifier of an [entity](crate::entity).
///
/// The identifier is implemented using a [generational index]: a combination of an index and a generation.
/// This allows fast insertion after data removal in an array while minimizing loss of spatial locality.
///
/// These identifiers are only valid on the [`World`] it's sourced from. Attempting to use an `Entity` to
/// fetch entity components or metadata from a different world will either fail or return unexpected results.
///
/// [generational index]: https://lucassardois.medium.com/generational-indices-guide-8e3c5f7fd594
///
/// # Stability warning
/// For all intents and purposes, `Entity` should be treated as an opaque identifier. The internal bit
/// representation is liable to change from release to release as are the behaviors or performance
/// characteristics of any of its trait implementations (i.e. `Ord`, `Hash`, etc.). This means that changes in
/// `Entity`'s representation, though made readable through various functions on the type, are not considered
/// breaking changes under [SemVer].
///
/// In particular, directly serializing with `Serialize` and `Deserialize` make zero guarantee of long
/// term wire format compatibility. Changes in behavior will cause serialized `Entity` values persisted
/// to long term storage (i.e. disk, databases, etc.) will fail to deserialize upon being updated.
///
/// # Usage
///
/// This data type is returned by iterating a `Query` that has `Entity` as part of its query fetch type parameter ([learn more]).
/// It can also be obtained by calling [`EntityCommands::id`] or [`EntityWorldMut::id`].
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
///     // Calling `spawn` returns `EntityWorldMut`.
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
/// [`EntityWorldMut::id`]: crate::world::EntityWorldMut::id
/// [`EntityCommands`]: crate::system::EntityCommands
/// [`Query::get`]: crate::system::Query::get
/// [`World`]: crate::world::World
/// [SemVer]: https://semver.org/
#[derive(Clone, Copy)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(opaque))]
#[cfg_attr(feature = "bevy_reflect", reflect(Hash, PartialEq, Debug, Clone))]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serialize"),
    reflect(Serialize, Deserialize)
)]
// Alignment repr necessary to allow LLVM to better output
// optimized codegen for `to_bits`, `PartialEq` and `Ord`.
#[repr(C, align(8))]
pub struct Entity {
    // Do not reorder the fields here. The ordering is explicitly used by repr(C)
    // to make this struct equivalent to a u64.
    #[cfg(target_endian = "little")]
    index: u32,
    generation: NonZero<u32>,
    #[cfg(target_endian = "big")]
    index: u32,
}

// By not short-circuiting in comparisons, we get better codegen.
// See <https://github.com/rust-lang/rust/issues/117800>
impl PartialEq for Entity {
    #[inline]
    fn eq(&self, other: &Entity) -> bool {
        // By using `to_bits`, the codegen can be optimized out even
        // further potentially. Relies on the correct alignment/field
        // order of `Entity`.
        self.to_bits() == other.to_bits()
    }
}

impl Eq for Entity {}

// The derive macro codegen output is not optimal and can't be optimized as well
// by the compiler. This impl resolves the issue of non-optimal codegen by relying
// on comparing against the bit representation of `Entity` instead of comparing
// the fields. The result is then LLVM is able to optimize the codegen for Entity
// far beyond what the derive macro can.
// See <https://github.com/rust-lang/rust/issues/106107>
impl PartialOrd for Entity {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        // Make use of our `Ord` impl to ensure optimal codegen output
        Some(self.cmp(other))
    }
}

// The derive macro codegen output is not optimal and can't be optimized as well
// by the compiler. This impl resolves the issue of non-optimal codegen by relying
// on comparing against the bit representation of `Entity` instead of comparing
// the fields. The result is then LLVM is able to optimize the codegen for Entity
// far beyond what the derive macro can.
// See <https://github.com/rust-lang/rust/issues/106107>
impl Ord for Entity {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // This will result in better codegen for ordering comparisons, plus
        // avoids pitfalls with regards to macro codegen relying on property
        // position when we want to compare against the bit representation.
        self.to_bits().cmp(&other.to_bits())
    }
}

impl Hash for Entity {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.to_bits().hash(state);
    }
}

#[deprecated(
    since = "0.16.0",
    note = "This is exclusively used with the now deprecated `Entities::alloc_at_without_replacement`."
)]
pub(crate) enum AllocAtWithoutReplacement {
    Exists(EntityLocation),
    DidNotExist,
    ExistsWithWrongGeneration,
}

impl Entity {
    /// Construct an [`Entity`] from a raw `index` value and a non-zero `generation` value.
    /// Ensure that the generation value is never greater than `0x7FFF_FFFF`.
    #[inline(always)]
    pub(crate) const fn from_raw_and_generation(index: u32, generation: NonZero<u32>) -> Entity {
        debug_assert!(generation.get() <= HIGH_MASK);

        Self { index, generation }
    }

    /// An entity ID with a placeholder value. This may or may not correspond to an actual entity,
    /// and should be overwritten by a new value before being used.
    ///
    /// ## Examples
    ///
    /// Initializing a collection (e.g. `array` or `Vec`) with a known size:
    ///
    /// ```no_run
    /// # use bevy_ecs::prelude::*;
    /// // Create a new array of size 10 filled with invalid entity ids.
    /// let mut entities: [Entity; 10] = [Entity::PLACEHOLDER; 10];
    ///
    /// // ... replace the entities with valid ones.
    /// ```
    ///
    /// Deriving [`Reflect`] for a component that has an `Entity` field:
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
    ///             entity: Entity::PLACEHOLDER,
    ///         }
    ///     }
    /// }
    /// ```
    pub const PLACEHOLDER: Self = Self::from_raw(u32::MAX);

    /// Creates a new entity ID with the specified `index` and a generation of 1.
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
    #[inline(always)]
    pub const fn from_raw(index: u32) -> Entity {
        Self::from_raw_and_generation(index, NonZero::<u32>::MIN)
    }

    /// Convert to a form convenient for passing outside of rust.
    ///
    /// Only useful for identifying entities within the same instance of an application. Do not use
    /// for serialization between runs.
    ///
    /// No particular structure is guaranteed for the returned bits.
    #[inline(always)]
    pub const fn to_bits(self) -> u64 {
        IdentifierMask::pack_into_u64(self.index, self.generation.get())
    }

    /// Reconstruct an `Entity` previously destructured with [`Entity::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    ///
    /// # Panics
    ///
    /// This method will likely panic if given `u64` values that did not come from [`Entity::to_bits`].
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        // Construct an Identifier initially to extract the kind from.
        let id = Self::try_from_bits(bits);

        match id {
            Ok(entity) => entity,
            Err(_) => panic!("Attempted to initialize invalid bits as an entity"),
        }
    }

    /// Reconstruct an `Entity` previously destructured with [`Entity::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    ///
    /// This method is the fallible counterpart to [`Entity::from_bits`].
    #[inline(always)]
    pub const fn try_from_bits(bits: u64) -> Result<Self, IdentifierError> {
        if let Ok(id) = Identifier::try_from_bits(bits) {
            let kind = id.kind() as u8;

            if kind == (IdKind::Entity as u8) {
                return Ok(Self {
                    index: id.low(),
                    generation: id.high(),
                });
            }
        }

        Err(IdentifierError::InvalidEntityId(bits))
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
        // Mask so not to expose any flags
        IdentifierMask::extract_value_from_high(self.generation.get())
    }
}

impl TryFrom<Identifier> for Entity {
    type Error = IdentifierError;

    #[inline]
    fn try_from(value: Identifier) -> Result<Self, Self::Error> {
        Self::try_from_bits(value.to_bits())
    }
}

impl From<Entity> for Identifier {
    #[inline]
    fn from(value: Entity) -> Self {
        Identifier::from_bits(value.to_bits())
    }
}

#[cfg(feature = "serialize")]
impl Serialize for Entity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.to_bits())
    }
}

#[cfg(feature = "serialize")]
impl<'de> Deserialize<'de> for Entity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let id: u64 = Deserialize::deserialize(deserializer)?;
        Entity::try_from_bits(id).map_err(D::Error::custom)
    }
}

/// Outputs the full entity identifier, including the index, generation, and the raw bits.
///
/// This takes the format: `{index}v{generation}#{bits}`.
///
/// For [`Entity::PLACEHOLDER`], this outputs `PLACEHOLDER`.
///
/// # Usage
///
/// Prefer to use this format for debugging and logging purposes. Because the output contains
/// the raw bits, it is easy to check it against serialized scene data.
///
/// Example serialized scene data:
/// ```text
/// (
///   ...
///   entities: {
///     4294967297: (  <--- Raw Bits
///       components: {
///         ...
///       ),
///   ...
/// )
/// ```
impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self == &Self::PLACEHOLDER {
            write!(f, "PLACEHOLDER")
        } else {
            write!(
                f,
                "{}v{}#{}",
                self.index(),
                self.generation(),
                self.to_bits()
            )
        }
    }
}

/// Outputs the short entity identifier, including the index and generation.
///
/// This takes the format: `{index}v{generation}`.
///
/// For [`Entity::PLACEHOLDER`], this outputs `PLACEHOLDER`.
impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self == &Self::PLACEHOLDER {
            write!(f, "PLACEHOLDER")
        } else {
            write!(f, "{}v{}", self.index(), self.generation())
        }
    }
}

impl SparseSetIndex for Entity {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.index() as usize
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Entity::from_raw(value as u32)
    }
}

/// An [`Iterator`] returning a sequence of [`Entity`] values from
pub struct ReserveEntitiesIterator<'a> {
    // Metas, so we can recover the current generation for anything in the freelist.
    meta: &'a [EntityMeta],

    // Reserved indices formerly in the freelist to hand out.
    freelist_indices: core::slice::Iter<'a, u32>,

    // New Entity indices to hand out, outside the range of meta.len().
    new_indices: core::ops::Range<u32>,
}

impl<'a> Iterator for ReserveEntitiesIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.freelist_indices
            .next()
            .map(|&index| {
                Entity::from_raw_and_generation(index, self.meta[index as usize].generation)
            })
            .or_else(|| self.new_indices.next().map(Entity::from_raw))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.freelist_indices.len() + self.new_indices.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for ReserveEntitiesIterator<'a> {}
impl<'a> core::iter::FusedIterator for ReserveEntitiesIterator<'a> {}

// SAFETY: Newly reserved entity values are unique.
unsafe impl EntitySetIterator for ReserveEntitiesIterator<'_> {}

/// A [`World`]'s internal metadata store on all of its entities.
///
/// Contains metadata on:
///  - The generation of every entity.
///  - The alive/dead status of a particular entity. (i.e. "has entity 3 been despawned?")
///  - The location of the entity's components in memory (via [`EntityLocation`])
///
/// [`World`]: crate::world::World
#[derive(Debug)]
pub struct Entities {
    meta: Vec<EntityMeta>,

    /// The `pending` and `free_cursor` fields describe three sets of Entity IDs
    /// that have been freed or are in the process of being allocated:
    ///
    /// - The `freelist` IDs, previously freed by `free()`. These IDs are available to any of
    ///   [`alloc`], [`reserve_entity`] or [`reserve_entities`]. Allocation will always prefer
    ///   these over brand new IDs.
    ///
    /// - The `reserved` list of IDs that were once in the freelist, but got reserved by
    ///   [`reserve_entities`] or [`reserve_entity`]. They are now waiting for [`flush`] to make them
    ///   fully allocated.
    ///
    /// - The count of new IDs that do not yet exist in `self.meta`, but which we have handed out
    ///   and reserved. [`flush`] will allocate room for them in `self.meta`.
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
    /// Once [`flush`] is done, `free_cursor` will equal `pending.len()`.
    ///
    /// [`alloc`]: Entities::alloc
    /// [`reserve_entity`]: Entities::reserve_entity
    /// [`reserve_entities`]: Entities::reserve_entities
    /// [`flush`]: Entities::flush
    pending: Vec<u32>,
    free_cursor: AtomicIdCursor,
}

impl Entities {
    pub(crate) const fn new() -> Self {
        Entities {
            meta: Vec::new(),
            pending: Vec::new(),
            free_cursor: AtomicIdCursor::new(0),
        }
    }

    /// Reserve entity IDs concurrently.
    ///
    /// Storage for entity generation and location is lazily allocated by calling [`flush`](Entities::flush).
    #[expect(
        clippy::allow_attributes,
        reason = "`clippy::unnecessary_fallible_conversions` may not always lint."
    )]
    #[allow(
        clippy::unnecessary_fallible_conversions,
        reason = "`IdCursor::try_from` may fail on 32-bit platforms."
    )]
    pub fn reserve_entities(&self, count: u32) -> ReserveEntitiesIterator {
        // Use one atomic subtract to grab a range of new IDs. The range might be
        // entirely nonnegative, meaning all IDs come from the freelist, or entirely
        // negative, meaning they are all new IDs to allocate, or a mix of both.
        let range_end = self.free_cursor.fetch_sub(
            IdCursor::try_from(count)
                .expect("64-bit atomic operations are not supported on this platform."),
            Ordering::Relaxed,
        );
        let range_start = range_end
            - IdCursor::try_from(count)
                .expect("64-bit atomic operations are not supported on this platform.");

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
            freelist_indices: self.pending[freelist_range].iter(),
            new_indices: new_id_start..new_id_end,
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
            Entity::from_raw_and_generation(index, self.meta[index as usize].generation)
        } else {
            // Grab a new ID, outside the range of `meta.len()`. `flush()` must
            // eventually be called to make it valid.
            //
            // As `self.free_cursor` goes more and more negative, we return IDs farther
            // and farther beyond `meta.len()`.
            Entity::from_raw(
                u32::try_from(self.meta.len() as IdCursor - n).expect("too many entities"),
            )
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
        if let Some(index) = self.pending.pop() {
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            Entity::from_raw_and_generation(index, self.meta[index as usize].generation)
        } else {
            let index = u32::try_from(self.meta.len()).expect("too many entities");
            self.meta.push(EntityMeta::EMPTY);
            Entity::from_raw(index)
        }
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any. Location should be
    /// written immediately.
    #[deprecated(
        since = "0.16.0",
        note = "This can cause extreme performance problems when used after freeing a large number of entities and requesting an arbitrary entity. See #18054 on GitHub."
    )]
    pub fn alloc_at(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.verify_flushed();

        let loc = if entity.index() as usize >= self.meta.len() {
            self.pending
                .extend((self.meta.len() as u32)..entity.index());
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.meta
                .resize(entity.index() as usize + 1, EntityMeta::EMPTY);
            None
        } else if let Some(index) = self.pending.iter().position(|item| *item == entity.index()) {
            self.pending.swap_remove(index);
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            None
        } else {
            Some(mem::replace(
                &mut self.meta[entity.index() as usize].location,
                EntityMeta::EMPTY.location,
            ))
        };

        self.meta[entity.index() as usize].generation = entity.generation;

        loc
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any.
    #[deprecated(
        since = "0.16.0",
        note = "This can cause extreme performance problems when used after freeing a large number of entities and requesting an arbitrary entity. See #18054 on GitHub."
    )]
    #[expect(
        deprecated,
        reason = "We need to support `AllocAtWithoutReplacement` for now."
    )]
    pub(crate) fn alloc_at_without_replacement(
        &mut self,
        entity: Entity,
    ) -> AllocAtWithoutReplacement {
        self.verify_flushed();

        let result = if entity.index() as usize >= self.meta.len() {
            self.pending
                .extend((self.meta.len() as u32)..entity.index());
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            self.meta
                .resize(entity.index() as usize + 1, EntityMeta::EMPTY);
            AllocAtWithoutReplacement::DidNotExist
        } else if let Some(index) = self.pending.iter().position(|item| *item == entity.index()) {
            self.pending.swap_remove(index);
            let new_free_cursor = self.pending.len() as IdCursor;
            *self.free_cursor.get_mut() = new_free_cursor;
            AllocAtWithoutReplacement::DidNotExist
        } else {
            let current_meta = &self.meta[entity.index() as usize];
            if current_meta.location.archetype_id == ArchetypeId::INVALID {
                AllocAtWithoutReplacement::DidNotExist
            } else if current_meta.generation == entity.generation {
                AllocAtWithoutReplacement::Exists(current_meta.location)
            } else {
                return AllocAtWithoutReplacement::ExistsWithWrongGeneration;
            }
        };

        self.meta[entity.index() as usize].generation = entity.generation;
        result
    }

    /// Destroy an entity, allowing it to be reused.
    ///
    /// Must not be called while reserved entities are awaiting `flush()`.
    pub fn free(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.verify_flushed();

        let meta = &mut self.meta[entity.index() as usize];
        if meta.generation != entity.generation {
            return None;
        }

        meta.generation = IdentifierMask::inc_masked_high_by(meta.generation, 1);

        if meta.generation == NonZero::<u32>::MIN {
            warn!(
                "Entity({}) generation wrapped on Entities::free, aliasing may occur",
                entity.index
            );
        }

        let loc = mem::replace(&mut meta.location, EntityMeta::EMPTY.location);

        self.pending.push(entity.index());

        let new_free_cursor = self.pending.len() as IdCursor;
        *self.free_cursor.get_mut() = new_free_cursor;
        Some(loc)
    }

    /// Ensure at least `n` allocations can succeed without reallocating.
    #[expect(
        clippy::allow_attributes,
        reason = "`clippy::unnecessary_fallible_conversions` may not always lint."
    )]
    #[allow(
        clippy::unnecessary_fallible_conversions,
        reason = "`IdCursor::try_from` may fail on 32-bit platforms."
    )]
    pub fn reserve(&mut self, additional: u32) {
        self.verify_flushed();

        let freelist_size = *self.free_cursor.get_mut();
        let shortfall = IdCursor::try_from(additional)
            .expect("64-bit atomic operations are not supported on this platform.")
            - freelist_size;
        if shortfall > 0 {
            self.meta.reserve(shortfall as usize);
        }
    }

    /// Returns true if the [`Entities`] contains [`entity`](Entity).
    // This will return false for entities which have been freed, even if
    // not reallocated since the generation is incremented in `free`
    pub fn contains(&self, entity: Entity) -> bool {
        self.resolve_from_id(entity.index())
            .is_some_and(|e| e.generation() == entity.generation())
    }

    /// Clears all [`Entity`] from the World.
    pub fn clear(&mut self) {
        self.meta.clear();
        self.pending.clear();
        *self.free_cursor.get_mut() = 0;
    }

    /// Returns the location of an [`Entity`].
    /// Note: for pending entities, returns `None`.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<EntityLocation> {
        if let Some(meta) = self.meta.get(entity.index() as usize) {
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

    /// Updates the location of an [`Entity`]. This must be called when moving the components of
    /// the entity around in storage.
    ///
    /// # Safety
    ///  - `index` must be a valid entity index.
    ///  - `location` must be valid for the entity at `index` or immediately made valid afterwards
    ///    before handing control to unknown code.
    #[inline]
    pub(crate) unsafe fn set(&mut self, index: u32, location: EntityLocation) {
        // SAFETY: Caller guarantees that `index` a valid entity index
        let meta = unsafe { self.meta.get_unchecked_mut(index as usize) };
        meta.location = location;
    }

    /// Increments the `generation` of a freed [`Entity`]. The next entity ID allocated with this
    /// `index` will count `generation` starting from the prior `generation` + the specified
    /// value + 1.
    ///
    /// Does nothing if no entity with this `index` has been allocated yet.
    pub(crate) fn reserve_generations(&mut self, index: u32, generations: u32) -> bool {
        if (index as usize) >= self.meta.len() {
            return false;
        }

        let meta = &mut self.meta[index as usize];
        if meta.location.archetype_id == ArchetypeId::INVALID {
            meta.generation = IdentifierMask::inc_masked_high_by(meta.generation, generations);
            true
        } else {
            false
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
            Some(Entity::from_raw_and_generation(index, generation))
        } else {
            // `id` is outside of the meta list - check whether it is reserved but not yet flushed.
            let free_cursor = self.free_cursor.load(Ordering::Relaxed);
            // If this entity was manually created, then free_cursor might be positive
            // Returning None handles that case correctly
            let num_pending = usize::try_from(-free_cursor).ok()?;
            (idu < self.meta.len() + num_pending).then_some(Entity::from_raw(index))
        }
    }

    fn needs_flush(&mut self) -> bool {
        *self.free_cursor.get_mut() != self.pending.len() as IdCursor
    }

    /// Allocates space for entities previously reserved with [`reserve_entity`](Entities::reserve_entity) or
    /// [`reserve_entities`](Entities::reserve_entities), then initializes each one using the supplied function.
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
            for (index, meta) in self.meta.iter_mut().enumerate().skip(old_meta_len) {
                init(
                    Entity::from_raw_and_generation(index as u32, meta.generation),
                    &mut meta.location,
                );
            }

            *free_cursor = 0;
            0
        };

        for index in self.pending.drain(new_free_cursor..) {
            let meta = &mut self.meta[index as usize];
            init(
                Entity::from_raw_and_generation(index, meta.generation),
                &mut meta.location,
            );
        }
    }

    /// Flushes all reserved entities to an "invalid" state. Attempting to retrieve them will return `None`
    /// unless they are later populated with a valid archetype.
    pub fn flush_as_invalid(&mut self) {
        // SAFETY: as per `flush` safety docs, the archetype id can be set to [`ArchetypeId::INVALID`] if
        // the [`Entity`] has not been assigned to an [`Archetype`][crate::archetype::Archetype], which is the case here
        unsafe {
            self.flush(|_entity, location| {
                location.archetype_id = ArchetypeId::INVALID;
            });
        }
    }

    /// The count of all entities in the [`World`] that have ever been allocated
    /// including the entities that are currently freed.
    ///
    /// This does not include entities that have been reserved but have never been
    /// allocated yet.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn total_count(&self) -> usize {
        self.meta.len()
    }

    /// The count of all entities in the [`World`] that are used,
    /// including both those allocated and those reserved, but not those freed.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn used_count(&self) -> usize {
        (self.meta.len() as isize - self.free_cursor.load(Ordering::Relaxed) as isize) as usize
    }

    /// The count of all entities in the [`World`] that have ever been allocated or reserved, including those that are freed.
    /// This is the value that [`Self::total_count()`] would return if [`Self::flush()`] were called right now.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn total_prospective_count(&self) -> usize {
        self.meta.len() + (-self.free_cursor.load(Ordering::Relaxed)).min(0) as usize
    }

    /// The count of currently allocated entities.
    #[inline]
    pub fn len(&self) -> u32 {
        // `pending`, by definition, can't be bigger than `meta`.
        (self.meta.len() - self.pending.len()) as u32
    }

    /// Checks if any entity is currently active.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets the source code location from which this entity has last been spawned
    /// or despawned.
    #[inline]
    pub(crate) fn set_spawned_or_despawned_by(&mut self, index: u32, caller: MaybeLocation) {
        caller.map(|caller| {
            let meta = self
                .meta
                .get_mut(index as usize)
                .expect("Entity index invalid");
            meta.spawned_or_despawned_by = MaybeLocation::new(Some(caller));
        });
    }

    /// Returns the source code location from which this entity has last been spawned
    /// or despawned. Returns `None` if its index has been reused by another entity
    /// or if this entity has never existed.
    pub fn entity_get_spawned_or_despawned_by(
        &self,
        entity: Entity,
    ) -> MaybeLocation<Option<&'static Location<'static>>> {
        MaybeLocation::new_with_flattened(|| {
            self.meta
                .get(entity.index() as usize)
                .filter(|meta|
                // Generation is incremented immediately upon despawn
                (meta.generation == entity.generation)
                || (meta.location.archetype_id == ArchetypeId::INVALID)
                && (meta.generation == IdentifierMask::inc_masked_high_by(entity.generation, 1)))
                .map(|meta| meta.spawned_or_despawned_by)
        })
        .map(Option::flatten)
    }

    /// Constructs a message explaining why an entity does not exist, if known.
    pub(crate) fn entity_does_not_exist_error_details(
        &self,
        entity: Entity,
    ) -> EntityDoesNotExistDetails {
        EntityDoesNotExistDetails {
            location: self.entity_get_spawned_or_despawned_by(entity),
        }
    }
}

/// An error that occurs when a specified [`Entity`] does not exist.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("The entity with ID {entity} {details}")]
pub struct EntityDoesNotExistError {
    /// The entity's ID.
    pub entity: Entity,
    /// Details on why the entity does not exist, if available.
    pub details: EntityDoesNotExistDetails,
}

impl EntityDoesNotExistError {
    pub(crate) fn new(entity: Entity, entities: &Entities) -> Self {
        Self {
            entity,
            details: entities.entity_does_not_exist_error_details(entity),
        }
    }
}

/// Helper struct that, when printed, will write the appropriate details
/// regarding an entity that did not exist.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityDoesNotExistDetails {
    location: MaybeLocation<Option<&'static Location<'static>>>,
}

impl fmt::Display for EntityDoesNotExistDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.location.into_option() {
            Some(Some(location)) => write!(f, "was despawned by {location}"),
            Some(None) => write!(
                f,
                "does not exist (index has been reused or was never spawned)"
            ),
            None => write!(
                f,
                "does not exist (enable `track_location` feature for more details)"
            ),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct EntityMeta {
    /// The current generation of the [`Entity`].
    pub generation: NonZero<u32>,
    /// The current location of the [`Entity`]
    pub location: EntityLocation,
    /// Location of the last spawn or despawn of this entity
    spawned_or_despawned_by: MaybeLocation<Option<&'static Location<'static>>>,
}

impl EntityMeta {
    /// meta for **pending entity**
    const EMPTY: EntityMeta = EntityMeta {
        generation: NonZero::<u32>::MIN,
        location: EntityLocation::INVALID,
        spawned_or_despawned_by: MaybeLocation::new(None),
    };
}

/// A location of an entity in an archetype.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EntityLocation {
    /// The ID of the [`Archetype`] the [`Entity`] belongs to.
    ///
    /// [`Archetype`]: crate::archetype::Archetype
    pub archetype_id: ArchetypeId,

    /// The index of the [`Entity`] within its [`Archetype`].
    ///
    /// [`Archetype`]: crate::archetype::Archetype
    pub archetype_row: ArchetypeRow,

    /// The ID of the [`Table`] the [`Entity`] belongs to.
    ///
    /// [`Table`]: crate::storage::Table
    pub table_id: TableId,

    /// The index of the [`Entity`] within its [`Table`].
    ///
    /// [`Table`]: crate::storage::Table
    pub table_row: TableRow,
}

impl EntityLocation {
    /// location for **pending entity** and **invalid entity**
    pub(crate) const INVALID: EntityLocation = EntityLocation {
        archetype_id: ArchetypeId::INVALID,
        archetype_row: ArchetypeRow::INVALID,
        table_id: TableId::INVALID,
        table_row: TableRow::INVALID,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn entity_niche_optimization() {
        assert_eq!(size_of::<Entity>(), size_of::<Option<Entity>>());
    }

    #[test]
    fn entity_bits_roundtrip() {
        // Generation cannot be greater than 0x7FFF_FFFF else it will be an invalid Entity id
        let e =
            Entity::from_raw_and_generation(0xDEADBEEF, NonZero::<u32>::new(0x5AADF00D).unwrap());
        assert_eq!(Entity::from_bits(e.to_bits()), e);
    }

    #[test]
    fn reserve_entity_len() {
        let mut e = Entities::new();
        e.reserve_entity();
        // SAFETY: entity_location is left invalid
        unsafe { e.flush(|_, _| {}) };
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn get_reserved_and_invalid() {
        let mut entities = Entities::new();
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
        assert_eq!(42, C1.index());
        assert_eq!(1, C1.generation());

        const C2: Entity = Entity::from_bits(0x0000_00ff_0000_00cc);
        assert_eq!(0x0000_00cc, C2.index());
        assert_eq!(0x0000_00ff, C2.generation());

        const C3: u32 = Entity::from_raw(33).index();
        assert_eq!(33, C3);

        const C4: u32 = Entity::from_bits(0x00dd_00ff_0000_0000).generation();
        assert_eq!(0x00dd_00ff, C4);
    }

    #[test]
    fn reserve_generations() {
        let mut entities = Entities::new();
        let entity = entities.alloc();
        entities.free(entity);

        assert!(entities.reserve_generations(entity.index(), 1));
    }

    #[test]
    fn reserve_generations_and_alloc() {
        const GENERATIONS: u32 = 10;

        let mut entities = Entities::new();
        let entity = entities.alloc();
        entities.free(entity);

        assert!(entities.reserve_generations(entity.index(), GENERATIONS));

        // The very next entity allocated should be a further generation on the same index
        let next_entity = entities.alloc();
        assert_eq!(next_entity.index(), entity.index());
        assert!(next_entity.generation() > entity.generation() + GENERATIONS);
    }

    #[test]
    #[expect(
        clippy::nonminimal_bool,
        reason = "This intentionally tests all possible comparison operators as separate functions; thus, we don't want to rewrite these comparisons to use different operators."
    )]
    fn entity_comparison() {
        assert_eq!(
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap()),
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
        );
        assert_ne!(
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(789).unwrap()),
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
        );
        assert_ne!(
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap()),
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(789).unwrap())
        );
        assert_ne!(
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap()),
            Entity::from_raw_and_generation(456, NonZero::<u32>::new(123).unwrap())
        );

        // ordering is by generation then by index

        assert!(
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
                >= Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
        );
        assert!(
            Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
                <= Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
        );
        assert!(
            !(Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
                < Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap()))
        );
        assert!(
            !(Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap())
                > Entity::from_raw_and_generation(123, NonZero::<u32>::new(456).unwrap()))
        );

        assert!(
            Entity::from_raw_and_generation(9, NonZero::<u32>::new(1).unwrap())
                < Entity::from_raw_and_generation(1, NonZero::<u32>::new(9).unwrap())
        );
        assert!(
            Entity::from_raw_and_generation(1, NonZero::<u32>::new(9).unwrap())
                > Entity::from_raw_and_generation(9, NonZero::<u32>::new(1).unwrap())
        );

        assert!(
            Entity::from_raw_and_generation(1, NonZero::<u32>::new(1).unwrap())
                < Entity::from_raw_and_generation(2, NonZero::<u32>::new(1).unwrap())
        );
        assert!(
            Entity::from_raw_and_generation(1, NonZero::<u32>::new(1).unwrap())
                <= Entity::from_raw_and_generation(2, NonZero::<u32>::new(1).unwrap())
        );
        assert!(
            Entity::from_raw_and_generation(2, NonZero::<u32>::new(2).unwrap())
                > Entity::from_raw_and_generation(1, NonZero::<u32>::new(2).unwrap())
        );
        assert!(
            Entity::from_raw_and_generation(2, NonZero::<u32>::new(2).unwrap())
                >= Entity::from_raw_and_generation(1, NonZero::<u32>::new(2).unwrap())
        );
    }

    // Feel free to change this test if needed, but it seemed like an important
    // part of the best-case performance changes in PR#9903.
    #[test]
    fn entity_hash_keeps_similar_ids_together() {
        use core::hash::BuildHasher;
        let hash = EntityHash;

        let first_id = 0xC0FFEE << 8;
        let first_hash = hash.hash_one(Entity::from_raw(first_id));

        for i in 1..=255 {
            let id = first_id + i;
            let hash = hash.hash_one(Entity::from_raw(id));
            assert_eq!(hash.wrapping_sub(first_hash) as u32, i);
        }
    }

    #[test]
    fn entity_hash_id_bitflip_affects_high_7_bits() {
        use core::hash::BuildHasher;

        let hash = EntityHash;

        let first_id = 0xC0FFEE;
        let first_hash = hash.hash_one(Entity::from_raw(first_id)) >> 57;

        for bit in 0..u32::BITS {
            let id = first_id ^ (1 << bit);
            let hash = hash.hash_one(Entity::from_raw(id)) >> 57;
            assert_ne!(hash, first_hash);
        }
    }

    #[test]
    fn entity_debug() {
        let entity = Entity::from_raw(42);
        let string = format!("{:?}", entity);
        assert_eq!(string, "42v1#4294967338");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{:?}", entity);
        assert_eq!(string, "PLACEHOLDER");
    }

    #[test]
    fn entity_display() {
        let entity = Entity::from_raw(42);
        let string = format!("{}", entity);
        assert_eq!(string, "42v1");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{}", entity);
        assert_eq!(string, "PLACEHOLDER");
    }
}
