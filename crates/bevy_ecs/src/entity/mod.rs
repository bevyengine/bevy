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

pub(crate) mod allocator;
mod clone_entities;
mod entity_set;
mod map_entities;

use allocator::{Allocator, RemoteAllocator};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

pub use clone_entities::*;
use derive_more::derive::Display;
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

use nonmax::NonMaxU32;
pub use unique_array::{UniqueEntityArray, UniqueEntityEquivalentArray};
pub use unique_slice::{UniqueEntityEquivalentSlice, UniqueEntitySlice};
pub use unique_vec::{UniqueEntityEquivalentVec, UniqueEntityVec};

use crate::{
    archetype::{ArchetypeId, ArchetypeRow},
    change_detection::MaybeLocation,
    component::Tick,
    storage::{SparseSetIndex, TableId, TableRow},
};
use alloc::vec::Vec;
use bevy_platform::sync::Arc;
use concurrent_queue::ConcurrentQueue;
use core::{fmt, hash::Hash, num::NonZero, panic::Location};
use log::warn;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

/// This represents the row or "index" of an [`Entity`] within the [`Entities`] table.
/// This is a lighter weight version of [`Entity`].
///
/// This is a unique identifier for an entity in the world.
/// This differs from [`Entity`] in that [`Entity`] is unique for all entities total (unless the [`Entity::generation`] wraps),
/// but this is only unique for entities that are active.
///
/// This can be used over [`Entity`] to improve performance in some cases,
/// but improper use can cause this to identify a different entity than intended.
/// Use with caution.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(opaque))]
#[cfg_attr(feature = "bevy_reflect", reflect(Hash, PartialEq, Debug, Clone))]
#[repr(transparent)]
pub struct EntityRow(NonMaxU32);

impl EntityRow {
    const PLACEHOLDER: Self = Self(NonMaxU32::MAX);

    /// Constructs a new [`EntityRow`] from its index.
    pub const fn new(index: NonMaxU32) -> Self {
        Self(index)
    }

    /// Gets the index of the entity.
    #[inline(always)]
    pub const fn index(self) -> u32 {
        self.0.get()
    }

    /// Gets some bits that represent this value.
    /// The bits are opaque and should not be regarded as meaningful.
    #[inline(always)]
    const fn to_bits(self) -> u32 {
        // SAFETY: NonMax is repr transparent.
        unsafe { mem::transmute::<NonMaxU32, u32>(self.0) }
    }

    /// Reconstruct an [`EntityRow`] previously destructured with [`EntityRow::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    ///
    /// # Panics
    ///
    /// This method will likely panic if given `u32` values that did not come from [`EntityRow::to_bits`].
    #[inline]
    const fn from_bits(bits: u32) -> Self {
        Self::try_from_bits(bits).expect("Attempted to initialize invalid bits as an entity row")
    }

    /// Reconstruct an [`EntityRow`] previously destructured with [`EntityRow::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    ///
    /// This method is the fallible counterpart to [`EntityRow::from_bits`].
    #[inline(always)]
    const fn try_from_bits(bits: u32) -> Option<Self> {
        match NonZero::<u32>::new(bits) {
            // SAFETY: NonMax and NonZero are repr transparent.
            Some(underlying) => Some(Self(unsafe {
                mem::transmute::<NonZero<u32>, NonMaxU32>(underlying)
            })),
            None => None,
        }
    }
}

impl SparseSetIndex for EntityRow {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.index() as usize
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Self::from_bits(value as u32)
    }
}

/// This tracks different versions or generations of an [`EntityRow`].
/// Importantly, this can wrap, meaning each generation is not necessarily unique per [`EntityRow`].
///
/// This should be treated as a opaque identifier, and it's internal representation may be subject to change.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(opaque))]
#[cfg_attr(feature = "bevy_reflect", reflect(Hash, PartialEq, Debug, Clone))]
#[repr(transparent)]
pub struct EntityGeneration(u32);

impl EntityGeneration {
    /// Represents the first generation of an [`EntityRow`].
    pub const FIRST: Self = Self(0);

    /// Gets some bits that represent this value.
    /// The bits are opaque and should not be regarded as meaningful.
    #[inline(always)]
    const fn to_bits(self) -> u32 {
        self.0
    }

    /// Reconstruct an [`EntityGeneration`] previously destructured with [`EntityGeneration::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    #[inline]
    const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns the [`EntityGeneration`] that would result from this many more `versions` of the corresponding [`EntityRow`] from passing.
    #[inline]
    pub const fn after_versions(self, versions: u32) -> Self {
        Self(self.0.wrapping_add(versions))
    }

    /// Identical to [`after_versions`](Self::after_versions) but also returns a `bool` indicating if,
    /// after these `versions`, one such version could conflict with a previous one.
    ///
    /// If this happens, this will no longer uniquely identify a version of an [`EntityRow`].
    /// This is called entity aliasing.
    #[inline]
    pub const fn after_versions_and_could_alias(self, versions: u32) -> (Self, bool) {
        let raw = self.0.overflowing_add(versions);
        (Self(raw.0), raw.1)
    }
}

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
    row: EntityRow,
    generation: EntityGeneration,
    #[cfg(target_endian = "big")]
    row: EntityRow,
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

impl Entity {
    /// Construct an [`Entity`] from a raw `row` value and a non-zero `generation` value.
    /// Ensure that the generation value is never greater than `0x7FFF_FFFF`.
    #[inline(always)]
    pub(crate) const fn from_raw_and_generation(
        row: EntityRow,
        generation: EntityGeneration,
    ) -> Entity {
        Self { row, generation }
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
    pub const PLACEHOLDER: Self = Self::from_raw(EntityRow::PLACEHOLDER);

    /// Creates a new entity ID with the specified `row` and a generation of 1.
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
    pub const fn from_raw(row: EntityRow) -> Entity {
        Self::from_raw_and_generation(row, EntityGeneration::FIRST)
    }

    /// This is equivalent to [`from_raw`](Self::from_raw) except that it takes a `u32` instead of an [`EntityRow`].
    ///
    /// Returns `None` if the row is `u32::MAX`.
    #[inline(always)]
    pub const fn from_raw_u32(row: u32) -> Option<Entity> {
        match NonMaxU32::new(row) {
            Some(row) => Some(Self::from_raw(EntityRow::new(row))),
            None => None,
        }
    }

    /// Convert to a form convenient for passing outside of rust.
    ///
    /// Only useful for identifying entities within the same instance of an application. Do not use
    /// for serialization between runs.
    ///
    /// No particular structure is guaranteed for the returned bits.
    #[inline(always)]
    pub const fn to_bits(self) -> u64 {
        self.row.to_bits() as u64 | ((self.generation.to_bits() as u64) << 32)
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
        if let Some(id) = Self::try_from_bits(bits) {
            id
        } else {
            panic!("Attempted to initialize invalid bits as an entity")
        }
    }

    /// Reconstruct an `Entity` previously destructured with [`Entity::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    ///
    /// This method is the fallible counterpart to [`Entity::from_bits`].
    #[inline(always)]
    pub const fn try_from_bits(bits: u64) -> Option<Self> {
        let raw_row = bits as u32;
        let raw_gen = (bits >> 32) as u32;

        if let Some(row) = EntityRow::try_from_bits(raw_row) {
            Some(Self {
                row,
                generation: EntityGeneration::from_bits(raw_gen),
            })
        } else {
            None
        }
    }

    /// Return a transiently unique identifier.
    /// See also [`EntityRow`].
    ///
    /// No two simultaneously-live entities share the same row, but dead entities' indices may collide
    /// with both live and dead entities. Useful for compactly representing entities within a
    /// specific snapshot of the world, such as when serializing.
    #[inline]
    pub const fn row(self) -> EntityRow {
        self.row
    }

    /// Equivalent to `self.row().index()`. See [`Self::row`] for details.
    #[inline]
    pub const fn index(self) -> u32 {
        self.row.index()
    }

    /// Returns the generation of this Entity's row. The generation is incremented each time an
    /// entity with a given row is despawned. This serves as a "count" of the number of times a
    /// given row has been reused (row, generation) pairs uniquely identify a given Entity.
    #[inline]
    pub const fn generation(self) -> EntityGeneration {
        self.generation
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
        Entity::try_from_bits(id)
            .ok_or_else(|| D::Error::custom("Attempting to deserialize an invalid entity."))
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
        self.row().sparse_set_index()
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Entity::from_raw(EntityRow::get_sparse_set_index(value))
    }
}

/// Stores entities that need to be flushed.
#[derive(Clone)]
struct RemotePending {
    pending: Arc<ConcurrentQueue<Entity>>,
}

impl RemotePending {
    fn new() -> Self {
        Self {
            pending: Arc::new(ConcurrentQueue::unbounded()),
        }
    }

    fn queue_flush(&self, entity: Entity) {
        // We don't need the result. If it's closed it doesn't matter, and it can't be full.
        _ = self.pending.push(entity);
    }
}

struct Pending {
    remote: RemotePending,
    #[cfg(feature = "std")]
    local: bevy_utils::Parallel<Vec<Entity>>,
}

impl Pending {
    fn new() -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                remote: RemotePending::new(),
                local: bevy_utils::Parallel::default(),
            }
        }

        #[cfg(not(feature = "std"))]
        {
            Self {
                remote: RemotePending::new(),
            }
        }
    }

    fn queue_flush(&self, entity: Entity) {
        #[cfg(feature = "std")]
        self.local.scope(|pending| pending.push(entity));

        #[cfg(not(feature = "std"))]
        self.remote.queue_flush(entity);
    }

    fn flush_local(&mut self, mut flusher: impl FnMut(Entity)) {
        #[cfg(feature = "std")]
        let pending = self.local.iter_mut().flat_map(|pending| pending.drain(..));

        #[cfg(not(feature = "std"))]
        let pending = self.remote.pending.try_iter();

        for pending in pending {
            flusher(pending);
        }
    }
}

impl fmt::Debug for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "opaque pending entities")
    }
}

/// An [`Iterator`] returning a sequence of [`Entity`] values from [`Entities`].
/// These will be flushed.
///
/// **NOTE:** Dropping will leak the remaining entities!
pub struct ReserveEntitiesIterator<'a> {
    allocator: allocator::AllocEntitiesIterator<'a>,
    entities: &'a Entities,
}

impl<'a> Iterator for ReserveEntitiesIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.allocator
            .next()
            .inspect(|entity| self.entities.pending.queue_flush(*entity))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.allocator.size_hint()
    }
}

impl<'a> core::iter::FusedIterator for ReserveEntitiesIterator<'a> {}

impl<'a> ExactSizeIterator for ReserveEntitiesIterator<'a> {}

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
    allocator: Allocator,
    pending: Pending,
}

impl Entities {
    pub(crate) fn new() -> Self {
        Entities {
            meta: Vec::new(),
            allocator: Allocator::new(),
            pending: Pending::new(),
        }
    }

    /// Reserve entity IDs concurrently.
    ///
    /// Storage for entity generation and location is lazily allocated by calling [`flush`](Entities::flush),
    /// but, if desiered, caller may set the [`EntityLocation`] prior to the flush instead,
    /// via [`flush_entity`](crate::world::World::flush_entity) for example.
    pub fn reserve_entities(&self, count: u32) -> ReserveEntitiesIterator {
        ReserveEntitiesIterator {
            allocator: self.alloc_entities(count),
            entities: self,
        }
    }

    /// Reserve one entity ID concurrently.
    ///
    /// Equivalent to `self.reserve_entities(1).next().unwrap()`, but more efficient.
    pub fn reserve_entity(&self) -> Entity {
        let entity = self.alloc();
        self.pending.queue_flush(entity);
        entity
    }

    /// Allocate an entity ID directly.
    /// Caller is responsible for setting the [`EntityLocation`] if desired,
    /// which must be done before [`get`](Self::get)ing its [`EntityLocation`].
    pub fn alloc(&self) -> Entity {
        self.allocator.alloc()
    }

    /// A more efficient way to [`alloc`](Self::alloc) multiple entities.
    pub fn alloc_entities(&self, count: u32) -> allocator::AllocEntitiesIterator {
        self.allocator.alloc_many(count)
    }

    /// A version of [`alloc_entities`](Self::alloc_entities) that requires the caller to ensure safety.
    ///
    /// # Safety
    ///
    /// Caller ensures [`Self::free`] is not called for the duration of the iterator.
    /// Caller ensures this allocator is not dropped for the lifetime of the iterator.
    pub(crate) unsafe fn alloc_entities_unsafe(
        &self,
        count: u32,
    ) -> allocator::AllocEntitiesIterator<'static> {
        self.allocator.alloc_many_unsafe(count)
    }
  
    /// This is the same as [`free`](Entities::free), but it allows skipping some generations.
    /// When the entity is reused, it will have a generation greater than the current generation + `generations`.
    #[inline]
    pub(crate) fn free_current_and_future_generations(
        &mut self,
        entity: Entity,
        generations: u32,
    ) -> Option<EntityLocation> {
        let theoretical = self.resolve_from_id(entity.index());
        if theoretical.is_none_or(|theoretcal| theoretcal != entity) {
            return None;
        }

        // SAFETY: We resolved its id to ensure it is valid.
        let meta = unsafe { self.force_get_meta_mut(entity.index() as usize) };
        let prev_generation = meta.generation;

        meta.generation = IdentifierMask::inc_masked_high_by(meta.generation, 1 + generations);

        if prev_generation > meta.generation || generations == u32::MAX {
            warn!(
                "Entity({}) generation wrapped on Entities::free, aliasing may occur",
                entity.row()
            );
        }

        let new_entity = Entity::from_raw_and_generation(entity.index, meta.generation);
        let loc = core::mem::replace(&mut meta.location, EntityLocation::INVALID);
        self.allocator.free(new_entity);

        Some(loc)
    }

    /// Destroy an entity, allowing it to be reused.
    pub fn free(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.free_current_and_future_generations(entity, 1)
    }

    /// Prepares the for `additional` allocations/reservations.
    /// This can prevent reallocation, etc, but since allocation can happen from anywhere, it is not guaranteed.
    pub fn prepare(&mut self, additional: u32) {
        let shortfall = additional.saturating_sub(self.allocator.num_free());
        self.meta.reserve(shortfall as usize);
    }

    /// Returns true if the [`Entities`] contains [`entity`](Entity).
    // This will return false for entities which have been freed, even if
    // not reallocated since the generation is incremented in `free`
    pub fn contains(&self, entity: Entity) -> bool {
        self.resolve_from_id(entity.row())
            .is_some_and(|e| e.generation() == entity.generation())
    }

    /// Clears all [`Entity`] from the World.
    pub fn clear(&mut self) {
        self.meta.clear();
        self.allocator = Allocator::new();
    }

    /// Returns the location of an [`Entity`].
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
    /// the existing entity around in storage.
    ///
    /// For spawning and despawning entities, [`set_spawn_despawn`](Self::set_spawn_despawn) must
    /// be used instead.
    ///
    /// # Safety
    ///  - `index` must be a valid entity index.
    ///  - `location` must be valid for the entity at `index` or immediately made valid afterwards
    ///    before handing control to unknown code.
    #[inline]
    pub(crate) unsafe fn set(&mut self, index: u32, location: EntityLocation) {
        // SAFETY: Caller guarantees that `index` a valid entity index
        let meta = unsafe { self.force_get_meta_mut(index as usize) };
        meta.location = location;
    }

    /// Gets the meta for this index mutably, creating it if it did not exist.
    ///
    /// # Safety
    ///
    /// `index` must be a valid index
    #[inline]
    unsafe fn force_get_meta_mut(&mut self, index: usize) -> &mut EntityMeta {
        if index >= self.meta.len() {
            self.resize_meta_for_index_risky(index)
        } else {
            // SAFETY: index is in bounds
            unsafe { self.meta.get_unchecked_mut(index) }
        }
    }

    /// Changes the size of [`Self::meta`] to support this index.
    /// This is risky because it assumes the index is not already in bounds.
    ///
    /// This is only used in `force_get_meta_mut` just to help branch prediction.
    // TODO: Hint unlikely instead of #[cold] once it is stabilized.
    #[cold]
    fn resize_meta_for_index_risky(&mut self, index: usize) -> &mut EntityMeta {
        self.meta.resize(index + 1, EntityMeta::FRESH);
        // SAFETY: We just added it
        unsafe { self.meta.get_unchecked_mut(index) }
    }

    /// Get the [`Entity`] with a given id, if it exists in this [`Entities`] collection
    /// Returns `None` if this [`Entity`] is outside of the range of currently allocated Entities
    ///
    /// Note: This method may return [`Entities`](Entity) which are currently free
    /// Note that [`contains`](Entities::contains) will correctly return false for freed
    /// entities, since it checks the generation
    #[inline]
    pub fn resolve_from_id(&self, index: u32) -> Option<Entity> {
        let idu = index as usize;
        if let Some(&EntityMeta { generation, .. }) = self.meta.get(idu) {
            Some(Entity::from_raw_and_generation(row, generation))
        } else {
            self.allocator
                .is_valid_index(index)
                .then_some(Entity::from_raw(index))
        }
    }

    /// Entities reserved via [`RemoteEntities::reserve`] may or may not be flushed naturally.
    /// Before using an entity reserved remotely, either set its location manually (usually though [`flush_entity`](crate::world::World::flush_entity)),
    /// or call this method to queue remotely reserved entities to be flushed with the rest.
    pub fn queue_remote_pending_to_be_flushed(&self) {
        #[cfg(feature = "std")]
        {
            let remote = self.pending.remote.pending.try_iter();
            self.pending.local.scope(|pending| pending.extend(remote));
        }
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
        let total = self.allocator.total_entity_indices() as usize;
        self.meta.resize(total, EntityMeta::FRESH);
        self.pending.flush_local(|entity| {
            // SAFETY: `meta` has been resized to include all entities.
            let meta = unsafe { self.meta.get_unchecked_mut(entity.index() as usize) };
            if meta.generation == entity.generation && meta.location == EntityLocation::INVALID {
                init(entity, &mut meta.location);
            }
        });
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
    /// including the entities that are currently pending reuse.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn total_count(&self) -> u64 {
        self.allocator.total_entity_indices()
    }

    /// The count of currently allocated entities.
    #[inline]
    pub fn len(&self) -> u64 {
        self.allocator.total_entity_indices() - self.allocator.num_free() as u64
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
            if !self.allocator.is_valid_index(index) {
                panic!("Entity index invalid")
            }
            // SAFETY: We just checked that it is valid
            let meta = unsafe { self.force_get_meta_mut(index as usize) };
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
            self.entity_get_spawned_or_despawned(entity)
                .map(|spawned_or_despawned| spawned_or_despawned.by)
        })
    }

    /// Returns the [`Tick`] at which this entity has last been spawned or despawned.
    /// Returns `None` if its index has been reused by another entity or if this entity
    /// has never existed.
    pub fn entity_get_spawned_or_despawned_at(&self, entity: Entity) -> Option<Tick> {
        self.entity_get_spawned_or_despawned(entity)
            .map(|spawned_or_despawned| spawned_or_despawned.at)
    }

    /// Returns the [`SpawnedOrDespawned`] related to the entity's last spawn or
    /// respawn. Returns `None` if its index has been reused by another entity or if
    /// this entity has never existed.
    #[inline]
    fn entity_get_spawned_or_despawned(&self, entity: Entity) -> Option<SpawnedOrDespawned> {
        self.meta
            .get(entity.index() as usize)
            .filter(|meta|
            // Generation is incremented immediately upon despawn
            (meta.generation == entity.generation)
            || (meta.location.archetype_id == ArchetypeId::INVALID)
            && (meta.generation == entity.generation.after_versions(1)))
            .map(|meta| {
                // SAFETY: valid archetype or non-min generation is proof this is init
                unsafe { meta.spawned_or_despawned.assume_init() }
            })
    }

    /// Returns the source code location from which this entity has last been spawned
    /// or despawned and the Tick of when that happened.
    ///
    /// # Safety
    ///
    /// The entity index must belong to an entity that is currently alive or, if it
    /// despawned, was not overwritten by a new entity of the same index.
    #[inline]
    pub(crate) unsafe fn entity_get_spawned_or_despawned_unchecked(
        &self,
        entity: Entity,
    ) -> (MaybeLocation, Tick) {
        // SAFETY: caller ensures entity is allocated
        let meta = unsafe { self.meta.get_unchecked(entity.index() as usize) };
        // SAFETY: caller ensures entities of this index were at least spawned
        let spawned_or_despawned = unsafe { meta.spawned_or_despawned.assume_init() };
        (spawned_or_despawned.by, spawned_or_despawned.at)
    }

    #[inline]
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for meta in &mut self.meta {
            if meta.generation != EntityGeneration::FIRST
                || meta.location.archetype_id != ArchetypeId::INVALID
            {
                // SAFETY: non-min generation or valid archetype is proof this is init
                let spawned_or_despawned = unsafe { meta.spawned_or_despawned.assume_init_mut() };
                spawned_or_despawned.at.check_tick(change_tick);
            }
        }
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

/// A remote version of [`Entities`] with limited functionality.
#[derive(Clone)]
pub struct RemoteEntities {
    allocator: RemoteAllocator,
    pending: RemotePending,
}

impl RemoteEntities {
    /// Creates a new [`RemoteEntities`] with this [`Entities`] as its source.
    /// Note that this can be closed at any time,
    /// so before using an allocated [`Entity`],
    /// check [`is_closed`](Self::is_closed).
    pub fn new(source: &Entities) -> Self {
        Self {
            allocator: RemoteAllocator::new(&source.allocator),
            pending: source.pending.remote.clone(),
        }
    }
    /// Allocates an [`Entity`]. Note that if the source [`Entities`] has been cleared or dropped, this will return a garbage value.
    /// Use [`is_closed`](Self::is_closed) to ensure the entities are valid before using them!
    ///
    /// The caller takes responsibility for eventually setting the [`EntityLocation`],
    /// usually via [`flush_entity`](crate::world::World::flush_entity).
    pub fn alloc(&self) -> Entity {
        self.allocator.alloc()
    }

    /// Reserves an [`Entity`]. Note that if the source [`Entities`] has been cleared or dropped, this will return a garbage value.
    /// Use [`is_closed`](Self::is_closed) to ensure the entities are valid before using them!
    ///
    /// This also queues it to be flushed after [`Entities::queue_remote_pending_to_be_flushed`] is called.
    /// If waiting for that is not an option, it is also possible to set the [`EntityLocation`] manually,
    /// usually via [`flush_entity`](crate::world::World::flush_entity).
    pub fn reserve(&self) -> Entity {
        let entity = self.alloc();
        self.pending.queue_flush(entity);
        entity
    }

    /// Returns true if this [`RemoteEntities`] is still connected to its source [`Entities`].
    /// This will return `false` if its source has been dropped or [`Entities::clear`]ed.
    ///
    /// Note that this can be closed immediately after returning false.
    ///
    /// Holding a reference to the source [`Entities`] while calling this will ensure the value does not change unknowingly.
    pub fn is_closed(&self) -> bool {
        self.allocator.is_closed()
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
    /// The current [`EntityGeneration`] of the [`EntityRow`].
    pub generation: EntityGeneration,
    /// The current location of the [`EntityRow`]
    pub location: EntityLocation,
    /// Location of the last spawn or despawn of this entity
    spawned_or_despawned: MaybeUninit<SpawnedOrDespawned>,
}

#[derive(Copy, Clone, Debug)]
struct SpawnedOrDespawned {
    by: MaybeLocation,
    at: Tick,
}

impl EntityMeta {
    /// This is the metadata for an entity index that has never had its location set or been freed.
    const FRESH: EntityMeta = EntityMeta {
        generation: EntityGeneration::FIRST,
        location: EntityLocation::INVALID,
        spawned_or_despawned: MaybeUninit::uninit(),
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
        let r = EntityRow::new(NonMaxU32::new(0xDEADBEEF).unwrap());
        assert_eq!(EntityRow::from_bits(r.to_bits()), r);

        // Generation cannot be greater than 0x7FFF_FFFF else it will be an invalid Entity id
        let e = Entity::from_raw_and_generation(
            EntityRow::new(NonMaxU32::new(0xDEADBEEF).unwrap()),
            EntityGeneration::from_bits(0x5AADF00D),
        );
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
        const C1: Entity = Entity::from_raw(EntityRow::new(NonMaxU32::new(42).unwrap()));
        assert_eq!(42, C1.index());
        assert_eq!(0, C1.generation().to_bits());

        const C2: Entity = Entity::from_bits(0x0000_00ff_0000_00cc);
        assert_eq!(!0x0000_00cc, C2.index());
        assert_eq!(0x0000_00ff, C2.generation().to_bits());

        const C3: u32 = Entity::from_raw(EntityRow::new(NonMaxU32::new(33).unwrap())).index();
        assert_eq!(33, C3);

        const C4: u32 = Entity::from_bits(0x00dd_00ff_1111_1111)
            .generation()
            .to_bits();
        assert_eq!(0x00dd_00ff, C4);
    }

    #[test]
    fn reserve_generations_and_alloc() {
        const GENERATIONS: u32 = 10;

        let mut entities = Entities::new();
        let entity = entities.alloc();

        assert!(entities
            .free_current_and_future_generations(entity, GENERATIONS)
            .is_some());

        // The very next entity allocated should be a further generation on the same index
        let next_entity = entities.alloc();
        assert_eq!(next_entity.index(), entity.index());
        assert!(next_entity.generation() > entity.generation().after_versions(GENERATIONS));
    }

    #[test]
    #[expect(
        clippy::nonminimal_bool,
        reason = "This intentionally tests all possible comparison operators as separate functions; thus, we don't want to rewrite these comparisons to use different operators."
    )]
    fn entity_comparison() {
        assert_eq!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ),
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            )
        );
        assert_ne!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(789)
            ),
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            )
        );
        assert_ne!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ),
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(789)
            )
        );
        assert_ne!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ),
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(456).unwrap()),
                EntityGeneration::from_bits(123)
            )
        );

        // ordering is by generation then by index

        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ) >= Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            )
        );
        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ) <= Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            )
        );
        assert!(
            !(Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ) < Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ))
        );
        assert!(
            !(Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ) > Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(123).unwrap()),
                EntityGeneration::from_bits(456)
            ))
        );

        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(9).unwrap()),
                EntityGeneration::from_bits(1)
            ) < Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(1).unwrap()),
                EntityGeneration::from_bits(9)
            )
        );
        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(1).unwrap()),
                EntityGeneration::from_bits(9)
            ) > Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(9).unwrap()),
                EntityGeneration::from_bits(1)
            )
        );

        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(1).unwrap()),
                EntityGeneration::from_bits(1)
            ) > Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(2).unwrap()),
                EntityGeneration::from_bits(1)
            )
        );
        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(1).unwrap()),
                EntityGeneration::from_bits(1)
            ) >= Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(2).unwrap()),
                EntityGeneration::from_bits(1)
            )
        );
        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(2).unwrap()),
                EntityGeneration::from_bits(2)
            ) < Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(1).unwrap()),
                EntityGeneration::from_bits(2)
            )
        );
        assert!(
            Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(2).unwrap()),
                EntityGeneration::from_bits(2)
            ) <= Entity::from_raw_and_generation(
                EntityRow::new(NonMaxU32::new(1).unwrap()),
                EntityGeneration::from_bits(2)
            )
        );
    }

    // Feel free to change this test if needed, but it seemed like an important
    // part of the best-case performance changes in PR#9903.
    #[test]
    fn entity_hash_keeps_similar_ids_together() {
        use core::hash::BuildHasher;
        let hash = EntityHash;

        let first_id = 0xC0FFEE << 8;
        let first_hash = hash.hash_one(Entity::from_raw(EntityRow::new(
            NonMaxU32::new(first_id).unwrap(),
        )));

        for i in 1..=255 {
            let id = first_id + i;
            let hash = hash.hash_one(Entity::from_raw(EntityRow::new(
                NonMaxU32::new(id).unwrap(),
            )));
            assert_eq!(first_hash.wrapping_sub(hash) as u32, i);
        }
    }

    #[test]
    fn entity_hash_id_bitflip_affects_high_7_bits() {
        use core::hash::BuildHasher;

        let hash = EntityHash;

        let first_id = 0xC0FFEE;
        let first_hash = hash.hash_one(Entity::from_raw(EntityRow::new(
            NonMaxU32::new(first_id).unwrap(),
        ))) >> 57;

        for bit in 0..u32::BITS {
            let id = first_id ^ (1 << bit);
            let hash = hash.hash_one(Entity::from_raw(EntityRow::new(
                NonMaxU32::new(id).unwrap(),
            ))) >> 57;
            assert_ne!(hash, first_hash);
        }
    }

    #[test]
    fn entity_debug() {
        let entity = Entity::from_raw(EntityRow::new(NonMaxU32::new(42).unwrap()));
        let string = format!("{:?}", entity);
        assert_eq!(string, "42v0#4294967253");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{:?}", entity);
        assert_eq!(string, "PLACEHOLDER");
    }

    #[test]
    fn entity_display() {
        let entity = Entity::from_raw(EntityRow::new(NonMaxU32::new(42).unwrap()));
        let string = format!("{}", entity);
        assert_eq!(string, "42v0");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{}", entity);
        assert_eq!(string, "PLACEHOLDER");
    }
}
