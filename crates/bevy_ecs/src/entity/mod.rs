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
mod visit_entities;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

pub use clone_entities::*;
pub use entity_set::*;
pub use map_entities::*;
use smallvec::SmallVec;
pub use visit_entities::*;

mod hash;
pub use hash::*;

pub mod hash_map;
pub mod hash_set;

pub mod index_map;
pub mod index_set;

pub mod unique_array;
pub mod unique_slice;
pub mod unique_vec;

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
use bevy_platform_support::sync::{
    atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering},
    Arc, Mutex, PoisonError, Weak,
};
use core::{
    fmt,
    hash::Hash,
    mem::{self, ManuallyDrop},
    num::NonZero,
    panic::Location,
};
use log::warn;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

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
#[cfg_attr(feature = "bevy_reflect", reflect(Hash, PartialEq, Debug))]
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
    note = "This is exclusively used with the now deprecated `Entities::alloc_at_without_replacement`."
)]
#[expect(
    unused,
    reason = "We are not supporting this deprecated on this branch yet. (It will be removed very soon.)"
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
pub struct ReserveEntitiesIterator<T: Iterator<Item = Entity>> {
    reused_entities: T,
    // New Entity indices to hand out, outside the range of meta.len().
    new_indices: core::ops::Range<u32>,
}

impl<T: Iterator<Item = Entity>> Iterator for ReserveEntitiesIterator<T> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.reused_entities
            .next()
            .or_else(|| self.new_indices.next().map(Entity::from_raw))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.reused_entities.size_hint();
        let additional = self.new_indices.len();
        (min + additional, max.map(|max| max + additional))
    }
}

impl<T: Iterator<Item = Entity> + ExactSizeIterator> ExactSizeIterator
    for ReserveEntitiesIterator<T>
{
}
impl<T: Iterator<Item = Entity> + core::iter::FusedIterator> core::iter::FusedIterator
    for ReserveEntitiesIterator<T>
{
}

// SAFETY: Newly reserved entity values are unique.
unsafe impl<T: Iterator<Item = Entity>> EntitySetIterator for ReserveEntitiesIterator<T> {}

/// This represents some entities that were at some point pending.
///
/// The first slice was pending and is flushed. The length of that slice is [`Self::flushed`].
///
/// The second slice was pending and is reserved (awaiting flush).
/// The length of that slice is [`Self::reserved`] - [`Self::flushed`].
///
/// The last slice is entities that are still pending. This fills the rest of [`Self::entities`] up to its length.
#[derive(Default)]
struct PendingEntitiesChunk {
    /// # Safety
    ///
    /// Length must be less than `u32::MAX`.
    entities: Vec<Entity>,
    /// The number of entities that have been attempted to reserve.
    /// This may exceed the length of [`Self::entities`].
    reserved: AtomicUsize,
    /// The number of entities that have been attempted to flush.
    /// This may exceed the length of [`Self::entities`].
    flushed: AtomicUsize,
}

impl PendingEntitiesChunk {
    /// Reserves a slice of num entities from pending.
    /// Note that the slice may not be of length `num`, but it will not be more than `num`.
    ///
    /// If it is less than `num`, the chunk is empty.
    fn reserve(&self, num: u32) -> core::ops::Range<usize> {
        let num_reserved_so_far = self.reserved.fetch_add(num as usize, Ordering::Relaxed);
        let ideal_new_reserved = num_reserved_so_far + num as usize;
        let start = self.entities.len().min(num_reserved_so_far);
        let end = self.entities.len().min(ideal_new_reserved);
        start..end
    }

    /// Reserves just one entity.
    fn reserve_one(&self) -> Option<Entity> {
        let num_reserved_so_far = self.reserved.fetch_add(1, Ordering::Relaxed);
        self.entities.get(num_reserved_so_far).copied()
    }

    /// Flushes the reserved slice with the passed `flusher`
    ///
    /// # Safety
    ///
    /// To prevent double flushing, this must not be called concurrently.
    unsafe fn flush(&self, mut flusher: impl FnMut(Entity)) {
        let new_flushed = self.reserved.load(Ordering::Relaxed);
        let flushed = self.flushed.swap(new_flushed, Ordering::Relaxed);
        if self.entities.len() <= flushed {
            return;
        }

        for to_flush in self.entities[flushed..new_flushed.min(self.entities.len())]
            .iter()
            .copied()
        {
            flusher(to_flush);
        }
    }

    /// Clears the chunk, resetting it for use.
    fn clear(&mut self) {
        self.entities.clear();
        *self.reserved.get_mut() = 0;
        *self.flushed.get_mut() = 0;
    }
}

/// Allows atomic access to [`PendingEntitiesChunk`]. This is effectively an [`Arc`] itself.
struct AtomicPendingEntitiesChunk(AtomicPtr<PendingEntitiesChunk>);

/// See [`AtomicPendingEntitiesChunk::get_into`].
enum AtomicPendingEntitiesChunkGetIntoResult {
    /// The [`Arc`]s are the same, so no change.
    SameArc,
    /// The [`Arc`]s were different and have been unified.
    Updated,
    /// The [`Arc`]s were different but were not able to be unified.
    Waiting,
}

impl AtomicPendingEntitiesChunk {
    fn ptr_to_arc(ptr: *mut PendingEntitiesChunk) -> Option<Arc<PendingEntitiesChunk>> {
        // SAFETY: we know the pointer is not null, and we are not dropping the `Weak`.
        let weak = unsafe {
            // We use `Weak` instead of `Arc` here because if we were the only owner of the `Arc`, and a `Self::swap`
            // happened inbetween loading the pointer and using it, we could have a use after free.
            // `Weak::from_raw` checks if the ptr is valid, and can be called after and during the `Arc`'s drop.
            // If the `Arc` no longer exists, it will simply return `None` when upgraded.
            ManuallyDrop::new(Weak::from_raw(ptr))
        };
        weak.upgrade()
    }

    /// Scopes access to the inner arc. If this is `None`, we must be doing a [`Self::swap`].
    fn get(&self) -> Option<Arc<PendingEntitiesChunk>> {
        let ptr = self.0.load(Ordering::Relaxed);
        Self::ptr_to_arc(ptr)
    }

    /// This is the same as putting [`Self::get`] into `dest` if it is `Some`, but it's more efficient.
    ///
    /// Returns `true` if and only if `dest` is outdated but could not be replaced at the moment.
    fn get_into(
        &self,
        dest: &mut Arc<PendingEntitiesChunk>,
    ) -> AtomicPendingEntitiesChunkGetIntoResult {
        let ptr = self.0.load(Ordering::Relaxed);
        if Arc::as_ptr(dest) == ptr.cast_const() {
            // They're the same arc, so nothing changes
            return AtomicPendingEntitiesChunkGetIntoResult::SameArc;
        }

        if let Some(new) = Self::ptr_to_arc(ptr) {
            *dest = new;
            AtomicPendingEntitiesChunkGetIntoResult::Updated
        } else {
            // The arc isn't available right now
            AtomicPendingEntitiesChunkGetIntoResult::Waiting
        }
    }

    // Prepares the [`Arc`] to be owned by [`Self`]
    //
    // # Safety
    //
    // The returned pointer must be used in [`Self::on_arc_go_out`], and it must never be used mutably.
    unsafe fn on_arc_come_in(new: &Arc<PendingEntitiesChunk>) -> *mut PendingEntitiesChunk {
        let new_arc_ptr = Arc::as_ptr(new);
        // SAFETY: `new` has not been dropped, and the ptr is correct.
        unsafe {
            // We are retaining a strong count for ourselves, so we act as an arc too.
            Arc::increment_strong_count(new_arc_ptr);
        }

        let new_weak = Arc::downgrade(new);
        new_weak.into_raw().cast_mut()
    }

    // Returns an [`Arc`] to normal after being owned by [`Self`]
    //
    // # Safety
    //
    // The passed pointer must have come from [`Self::on_arc_come_in`].
    unsafe fn on_arc_go_out(old_ptr: *mut PendingEntitiesChunk) -> Arc<PendingEntitiesChunk> {
        // SAFETY: the pointer is valid
        let old_weak = unsafe { Weak::from_raw(old_ptr) };

        // we incremented the strong count when it came in, so it must still be valid.
        let old = old_weak.upgrade().unwrap();
        let old_arc_ptr = Arc::as_ptr(&old);

        // SAFETY: The pointer is valid. `old` is not dropped.
        // We need to decrement the strong count because we aren't owning it anymore
        unsafe {
            Arc::decrement_strong_count(old_arc_ptr);
        }

        old
    }

    /// Puts the new arc in, returning the old one.
    fn swap(&self, new: Arc<PendingEntitiesChunk>) -> Arc<PendingEntitiesChunk> {
        // SAFETY: The ordering is preserved, and we don't mutate it.
        unsafe {
            let new_ptr = Self::on_arc_come_in(&new);
            let old_ptr = self.0.swap(new_ptr, Ordering::Relaxed);
            Self::on_arc_go_out(old_ptr)
        }
    }

    /// Constructs a new [`Self`]
    fn new(chunk: Arc<PendingEntitiesChunk>) -> Self {
        // SAFETY: Safety ensured by either [`Self::swap`] or `drop`
        let ptr = unsafe { Self::on_arc_come_in(&chunk) };
        Self(AtomicPtr::new(ptr))
    }
}

impl Drop for AtomicPendingEntitiesChunk {
    fn drop(&mut self) {
        let old_ptr = *self.0.get_mut();
        // SAFETY: The pointer must have come from [`Self::new`] or [`Self::swap`].
        let _old = unsafe { Self::on_arc_go_out(old_ptr) };
    }
}

/// Coordinates entity reservation
struct AtomicEntityReservations {
    /// The current chunk of pending entities
    pending_chunk: AtomicPendingEntitiesChunk,
    /// The total number of entities, including those reserved.
    meta_len: AtomicU32,
    /// The value of [`Self::meta_len`] when the most recent flush started.
    flushed_meta_len: AtomicU32,
    /// This is true if and only if the backing [`Entities`] has been cut off.
    closed: AtomicBool,

    /// The pending list being added to. This will become the next [`Self::pending_chunk`].
    growing_pending: Mutex<PendingEntitiesChunk>,
    /// This is true if [`Self::growing_pending`] is worth pushing into [`Self::pending_chunk`].
    worth_swap: AtomicBool,
    /// A pool of pending entities to prevent allocations.
    pending_chunk_pool: Mutex<SmallVec<[PendingEntitiesChunk; 2]>>,
    /// The newly created pending chunks.
    new_pending_chunks: Mutex<SmallVec<[Arc<PendingEntitiesChunk>; 2]>>,
}

impl AtomicEntityReservations {
    /// Creates a new [`EntityReservations`]
    fn new() -> Self {
        Self {
            pending_chunk: AtomicPendingEntitiesChunk::new(Arc::default()),
            meta_len: AtomicU32::default(),
            flushed_meta_len: AtomicU32::default(),
            growing_pending: Mutex::default(),
            worth_swap: AtomicBool::new(false),
            pending_chunk_pool: Mutex::default(),
            new_pending_chunks: Mutex::default(),
            closed: AtomicBool::default(),
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    /// Reserves a `num` entities by appending to the meta list.
    /// This should only be called if reusing an entity from a pending list is not practical.
    fn reserve_append(&self, num: u32) -> core::ops::Range<u32> {
        let current_meta = self.meta_len.fetch_add(num, Ordering::Relaxed);
        let Some(new_meta) = current_meta.checked_add(num) else {
            // Since this is relaxed, this may allow other reservations with the wrapped value before we set it back to the max.
            // But future reservations would panic again, and this is extremely rare
            // and would only cause a problem if it happened on an entirely separate thread that chose to ignore a poison error.
            // So while this leaves a narrow opeining for invalid reservations, it is worth it for the speed.
            self.meta_len.store(u32::MAX, Ordering::Release);
            panic!("too many entities");
        };
        current_meta..new_meta
    }

    /// Flushes the meta data for entities from [`Self::reserve_append`].
    ///
    /// `unseen_flusher` refers to [`EntityMeta`] that existed before this call but that this [`AtomicEntityReservations`] has not seen.
    /// `brand_new_flusher` refers to [`EntityMeta`] that did not exist before this call.
    fn flush_appended(
        &self,
        metas: &mut Vec<EntityMeta>,
        mut should_flush_unseen: impl FnMut(&mut EntityMeta, u32) -> bool,
        mut flusher: impl FnMut(&mut EntityMeta, u32),
    ) {
        let current_len = metas.len() as u32;
        let theoretical_len = self.meta_len.load(Ordering::Relaxed);
        let flushed_up_to = self
            .flushed_meta_len
            .swap(theoretical_len, Ordering::Relaxed);
        debug_assert!(current_len <= theoretical_len);

        // flush those we haven't seen/flushed yet
        for index in flushed_up_to..current_len {
            // SAFETY: It is known to be a valid index.
            let meta = unsafe { metas.get_unchecked_mut(index as usize) };
            if should_flush_unseen(meta, index) {
                flusher(meta, index);
            }
        }

        // flush those we need to create
        metas.resize(theoretical_len as usize, EntityMeta::EMPTY);
        for index in current_len..theoretical_len {
            // SAFETY: It is known to be a valid index.
            let meta = unsafe { metas.get_unchecked_mut(index as usize) };
            flusher(meta, index);
        }
    }

    /// If beneficial, swaps pending arcs to make more pending entities available.
    /// Returns `true` if and only if the swap happened.
    fn refresh(&self) -> bool {
        // This will early return unless a recent `Entities::free` has occored.
        // We do this redundantly before locking `growing_pending` to this common early return faster.
        if !self.worth_swap.load(Ordering::Relaxed) {
            return false;
        }

        // We lock early to prevent concurrent refreshes.
        let mut pending = self
            .growing_pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        if !self.worth_swap.swap(false, Ordering::Relaxed) {
            return false;
        }

        let next_pending = self
            .pending_chunk_pool
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
            .unwrap_or_default();
        let ready_pending = mem::replace::<PendingEntitiesChunk>(&mut pending, next_pending);
        let new_pending = Arc::new(ready_pending);
        self.pending_chunk.swap(new_pending.clone());

        // release the pending lock since we don't need it anymore
        drop(pending);

        // keep a m arc of the new pending list so we can ensure it is flushed before it is dropped.
        self.new_pending_chunks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(new_pending);

        true
    }

    /// Adds `reuse` to a pool to prevent further allocation.
    fn reuse_pending(&self, mut reuse: PendingEntitiesChunk) {
        reuse.clear();
        self.pending_chunk_pool
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(reuse);
    }

    /// Collects the new [`PendingEntitiesChunk`]s so that they can be flushed as needed.
    fn get_new_pending_chunks(
        &self,
        getter: impl FnOnce(smallvec::Drain<[Arc<PendingEntitiesChunk>; 2]>),
    ) {
        getter(
            self.new_pending_chunks
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .drain(..),
        );
    }

    /// Gets access to a growing list of pending entities.
    fn pending_mut<T: 'static>(&self, func: impl FnOnce(&mut PendingEntitiesChunk) -> T) -> T {
        let mut pending = self
            .growing_pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let res = func(&mut pending);
        self.worth_swap.store(
            pending.entities.len() - *pending.reserved.get_mut() > 0,
            Ordering::Relaxed,
        );
        res
    }
}

/// Performs entity reservation.
#[derive(Clone)]
pub struct EntityReserver {
    coordinator: Arc<AtomicEntityReservations>,
    pending: Arc<PendingEntitiesChunk>,
    /// This is the number of entities the reserver is allowed to reserve by extension
    /// before checking for more pending entities.
    ///
    /// Setting this too high can increase memory use and, in rare cases, exhaust the entity count, causing a crash.
    /// Setting this too low will slow down reservation if there are no pending entities.
    /// Setting this to `0` will ensure no pending entities are missed.
    /// Setting this to `u32::MAX` effectively turns this off. You can use [`Self::refresh`] to do this manually.
    ///
    /// The default value is 10.
    pub tolerance: u32,
    /// This is the [`Self::tolerance`] left. When this hits 0, a [`Self::refresh`] will occor.
    pub tolerance_left: u32,
    /// This is just a temporary list of reserved entities.
    tmp_reserved: Vec<Entity>,
}

impl EntityReserver {
    /// Sets the [`Self::tolerance`] for this reserver.
    pub fn with_tolerance(mut self, tolerance: u32) -> Self {
        self.tolerance = tolerance;
        self.tolerance_left = tolerance;
        self
    }

    /// Gets the [`RemoteEntities`] for this reserver.
    pub fn remote_entities(&self) -> RemoteEntities {
        RemoteEntities {
            coordinator: self.coordinator.clone(),
        }
    }

    /// Constructs a new [`EntityReserver`].
    fn new(coordinator: Arc<AtomicEntityReservations>) -> Self {
        const TIMEOUT: u8 = 5;
        let mut timeout = TIMEOUT;
        let pending = loop {
            if let Some(found) = coordinator.pending_chunk.get() {
                break Some(found);
            }
            // We must be swapping. Wait for the swap to succeed.
            core::hint::spin_loop();

            #[cfg(feature = "std")]
            {
                if let Some(found) = coordinator.pending_chunk.get() {
                    break Some(found);
                }
                // The swap is taking longer than usual. There must have been an interrupt, so let's yield.
                std::thread::yield_now();
            }

            if timeout == 0 {
                break None;
            }

            timeout -= 1;
        }
        // This it a "just in case" measure.
        .unwrap_or_default();

        Self {
            coordinator,
            pending,
            tolerance: 0,
            tolerance_left: 0,
            tmp_reserved: Vec::new(),
        }
        .with_tolerance(10)
    }

    /// Returns true if and only if the backing [`Entities`] has been closed.
    /// That means entities reserved by this will not be flushed and are not valid.
    pub fn is_closed(&self) -> bool {
        self.coordinator.is_closed()
    }

    /// Refreshes the reserver to improve the quality of the reservations, setting [`Self::tolerance_left`].
    ///
    /// This will make it more likely that reserved entities are reused.
    ///
    /// Returns a `bool` that is true if and only if the refresh was effective.
    pub fn refresh(&mut self) -> bool {
        // We don't care if the swap happened here since another resrver may has swapped already, requiring us to "catch up".
        let _swapped = self.coordinator.refresh();
        let (new_tolerance, result) =
            match self.coordinator.pending_chunk.get_into(&mut self.pending) {
                AtomicPendingEntitiesChunkGetIntoResult::SameArc => (self.tolerance, false),
                AtomicPendingEntitiesChunkGetIntoResult::Updated => (self.tolerance, true),
                // We are waiting for a refresh to be complete, so the result is false, but keep the tolerance at 0 so we try again soon.
                AtomicPendingEntitiesChunkGetIntoResult::Waiting => (0, false),
            };
        self.tolerance_left = new_tolerance;
        result
    }

    /// Based on [`Self::tolerance_left`], may [`Self::refresh`].
    /// Returns `true` if and only if it refreshed effectively.
    ///
    /// `pre_on_refresh` runs with [`Self::tmp_reserved`] and [`Self::pending`] right before a refresh happens.
    /// Note that it may run even if this returns `false`, but if it returns `true`, this must have run.
    fn on_reserve_extended(
        &mut self,
        num_extended: u32,
        pre_on_refresh: impl FnOnce(&mut Vec<Entity>, &PendingEntitiesChunk),
    ) -> bool {
        if num_extended == 0 {
            return false;
        }
        match self.tolerance_left.checked_sub(num_extended) {
            Some(new_left) => {
                self.tolerance_left = new_left;
                false
            }
            None => {
                pre_on_refresh(&mut self.tmp_reserved, &self.pending);
                self.refresh()
            }
        }
    }

    /// Reserves `num` entities in a [`ReserveEntitiesIterator`].
    pub fn reserve_entities(
        &mut self,
        num: u32,
    ) -> ReserveEntitiesIterator<
        core::iter::Chain<
            alloc::vec::Drain<'_, Entity>,
            core::iter::Copied<core::slice::Iter<'_, Entity>>,
        >,
    > {
        let reserved = self.pending.reserve(num);
        let reserved_copy = reserved.clone();
        let mut still_needed = num - reserved.len() as u32;

        let reused = if self.on_reserve_extended(still_needed, move |tmp_reserved, pending| {
            // We're about to loose our current pending arc, so we need to put what we've reserved so far into a new slice.
            // It's already empty since this is the only place we use it, and we drain it as part of the iterator.
            tmp_reserved.extend_from_slice(&pending.entities[reserved_copy]);
        }) {
            // We have a new pending arc, so we need to extend everything
            // by trying to reserve the remaining from the new arc.
            let more_reserved = self.pending.reserve(still_needed);
            still_needed -= more_reserved.len() as u32;
            self.pending.entities[more_reserved].iter()
        } else {
            // This is the usual case. We can just use our existing arc.
            self.pending.entities[reserved].iter()
        };
        let reused = self.tmp_reserved.drain(..).chain(reused.copied());

        // reserve any final entities by appending.
        let new = if still_needed == 0 {
            0..0
        } else {
            self.coordinator.reserve_append(still_needed)
        };

        ReserveEntitiesIterator {
            reused_entities: reused,
            new_indices: new,
        }
    }

    /// Reserves just 1 entity.
    pub fn reserve_entity(&mut self) -> Entity {
        // try to use pending
        self.pending
            .reserve_one()
            .or_else(|| {
                // if we refresh, try the new pending
                if self.on_reserve_extended(1, |_, _| {}) {
                    self.pending.reserve_one()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // if there are no pending available, append reserve.

                // SAFETY: The range is known to have length 1.
                Entity::from_raw(unsafe {
                    self.coordinator.reserve_append(1).next().unwrap_unchecked()
                })
            })
    }

    /// Reserves `num` entities in a [`ReserveEntitiesIterator`].
    ///
    /// **NOTE:** This ignores [`Self::tolerance_left`], so this may miss pending entities.
    /// When possible, prefer [`reserve_entities`](Self::reserve_entities).
    pub fn reserve_entities_no_refresh(
        &self,
        num: u32,
    ) -> ReserveEntitiesIterator<core::iter::Copied<core::slice::Iter<'_, Entity>>> {
        let reserved = self.pending.reserve(num);
        let still_needed = num - reserved.len() as u32;

        let new = if still_needed == 0 {
            0..0
        } else {
            self.coordinator.reserve_append(still_needed)
        };

        ReserveEntitiesIterator {
            reused_entities: self.pending.entities[reserved].iter().copied(),
            new_indices: new,
        }
    }

    /// Reserves just 1 entity.
    ///
    /// **NOTE:** This ignores [`Self::tolerance_left`], so this may miss pending entities.
    /// When possible, prefer [`reserve_entity`](Self::reserve_entity).
    pub fn reserve_entity_no_refresh(&self) -> Entity {
        self.pending.reserve_one().unwrap_or_else(|| {
            // SAFETY: The range is known to have length 1.
            Entity::from_raw(unsafe {
                self.coordinator.reserve_append(1).next().unwrap_unchecked()
            })
        })
    }
}

/// A version of [`Entities`] that can be used remotely.
#[derive(Clone)]
pub struct RemoteEntities {
    coordinator: Arc<AtomicEntityReservations>,
}

impl RemoteEntities {
    /// Creates a new [`EntityReserver`]. Use this to reserve entities in bulk.
    pub fn reserver(&self) -> EntityReserver {
        EntityReserver::new(self.coordinator.clone())
    }

    /// Creates a new [`EntityReserver`]. Use this to reserve entities in bulk.
    pub fn into_reserver(self) -> EntityReserver {
        EntityReserver::new(self.coordinator)
    }

    /// Returns true if and only if the backing [`Entities`] has been closed.
    /// That means entities reserved by this will not be flushed and are not valid.
    pub fn is_closed(&self) -> bool {
        self.coordinator.is_closed()
    }

    /// Reserves just 1 entity.
    ///
    /// If you only need one, this is faster than using [`Self::reserver`].
    /// Otherwise, prefer [`EntityReserver`].
    pub fn reserve_entity(&self) -> Entity {
        self.coordinator
            .pending_chunk
            .get()
            .and_then(|pending| pending.reserve_one())
            .unwrap_or_else(|| {
                // SAFETY: the range has exactly 1 item
                let index = unsafe { self.coordinator.reserve_append(1).next().unwrap_unchecked() };
                Entity::from_raw(index)
            })
    }
}

/// A [`World`]'s internal metadata store on all of its entities.
///
/// Contains metadata on:
///  - The generation of every entity.
///  - The alive/dead status of a particular entity. (i.e. "has entity 3 been despawned?")
///  - The location of the entity's components in memory (via [`EntityLocation`])
///
/// [`World`]: crate::world::World
pub struct Entities {
    /// The metadata for each entity.
    meta: Vec<EntityMeta>,
    /// Coordinates all reservations, locally and otherwise.
    coordinator: Arc<AtomicEntityReservations>,
    /// The local reserver for when we are unable to reserve anything directly.
    reserver: EntityReserver,
    /// These are the pending chunks that have been distributed.
    /// They are "wild" and could be anywhere. We can reuse them as soon as the [`Arc`] is unique.
    wild_pending_chunks: Vec<Arc<PendingEntitiesChunk>>,
    /// These are entities that neither exist nor need to be flushed into existence.
    /// We own these entities and can do with them as we like.
    /// These are marked with [`EntityLocation::OWNED`] to prevent them from being flushed.
    owned: Vec<Entity>,
    /// The number of [`entities`](Entity) to have on standby for allocations.
    /// The higher the number, the faster allocations will be, but the more memory and entities will be taken up but unused.
    /// The default value is 256.
    pub ideal_owned: NonZero<u32>,
}

impl Entities {
    pub(crate) fn new() -> Self {
        let coordinator = Arc::new(AtomicEntityReservations::new());
        let reserver = EntityReserver::new(coordinator.clone());
        Entities {
            meta: Vec::new(),
            coordinator,
            reserver,
            wild_pending_chunks: Vec::new(),
            owned: Vec::new(),
            // SAFETY: 256 > 0
            ideal_owned: unsafe { NonZero::new_unchecked(256) },
        }
    }

    /// Reserve entity IDs concurrently.
    ///
    /// Storage for entity generation and location is lazily allocated by calling [`flush`](Entities::flush).
    pub fn reserve_entities(
        &self,
        count: u32,
    ) -> ReserveEntitiesIterator<core::iter::Copied<core::slice::Iter<'_, Entity>>> {
        self.reserver.reserve_entities_no_refresh(count)
    }

    /// Reserve one entity ID concurrently.
    ///
    /// Equivalent to `self.reserve_entities(1).next().unwrap()`, but more efficient.
    pub fn reserve_entity(&self) -> Entity {
        self.reserver.reserve_entity_no_refresh()
    }

    /// Safely extends [`Self::owned`].
    fn extend_owned(&mut self, num: NonZero<u32>) {
        let new = self.reserver.reserve_entities(num.get());

        // Make sure we don't actually flush these
        self.meta
            .resize(new.new_indices.end as usize, EntityMeta::EMPTY);

        let new = new.inspect(|entity| {
            // SAFETY: we just resized to ensure all are valid.
            unsafe {
                self.meta
                    .get_unchecked_mut(entity.index() as usize)
                    .location = EntityLocation::OWNED;
            }
        });

        self.owned.extend(new);
    }

    /// Allocate an entity ID directly.
    pub fn alloc(&mut self) -> Entity {
        self.owned.pop().unwrap_or_else(|| {
            self.extend_owned(self.ideal_owned);
            // SAFETY: We just extended this by a non zero
            unsafe { self.owned.pop().unwrap_unchecked() }
        })
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any. Location should be
    /// written immediately.
    #[deprecated(
        note = "This can cause extreme performance problems when used after freeing a large number of entities and requesting an arbitrary entity. See #18054 on GitHub."
    )]
    pub fn alloc_at(&mut self, _entity: Entity) -> Option<EntityLocation> {
        todo!()
    }

    /// Allocate a specific entity ID, overwriting its generation.
    ///
    /// Returns the location of the entity currently using the given ID, if any.
    #[deprecated(
        note = "This can cause extreme performance problems when used after freeing a large number of entities and requesting an arbitrary entity. See #18054 on GitHub."
    )]
    #[expect(
        deprecated,
        reason = "We need to support `AllocAtWithoutReplacement` for now."
    )]
    pub(crate) fn alloc_at_without_replacement(
        &mut self,
        _entity: Entity,
    ) -> AllocAtWithoutReplacement {
        todo!()
    }

    /// Destroy an entity, allowing it to be reused.
    pub fn free(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.free_current_and_future_generations(entity, 0)
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

        let meta = match self.meta.get_mut(entity.index() as usize) {
            Some(found) => found,
            None => {
                // The entity must have been reserved and needs to be flushed.
                self.meta
                    .resize(entity.index() as usize + 1, EntityMeta::EMPTY);
                // SAFETY: We just added it.
                unsafe { self.meta.get_unchecked_mut(entity.index() as usize) }
            }
        };

        let prev_generation = meta.generation;
        meta.generation = IdentifierMask::inc_masked_high_by(meta.generation, 1 + generations);

        if prev_generation > meta.generation {
            warn!(
                "Entity({}) generation wrapped on Entities::free, aliasing may occur",
                entity.index
            );
        }

        let loc = mem::replace(&mut meta.location, EntityLocation::OWNED);

        self.owned.push(Entity::from_raw_and_generation(
            entity.index,
            meta.generation,
        ));

        Some(loc)
    }

    /// Ensure at least `n` allocations can succeed without allocations or internal reservations.
    pub fn reserve(&mut self, additional: u32) {
        let shortfal = additional.saturating_sub(self.owned.len() as u32);
        if let Some(additional) = NonZero::new(shortfal) {
            self.extend_owned(additional);
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
        self.owned.clear();
        self.wild_pending_chunks.clear();
        self.coordinator.close();
        self.coordinator = Arc::new(AtomicEntityReservations::new());
        self.reserver = EntityReserver::new(self.coordinator.clone());
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
            let len = self.coordinator.meta_len.load(Ordering::Relaxed);
            (index < len).then_some(Entity::from_raw(index))
        }
    }

    /// Allocates space for entities previously reserved with [`reserve_entity`](Entities::reserve_entity),
    /// [`reserve_entities`](Entities::reserve_entities), or an associated [`EntityReserver`], then initializes each one using the supplied function.
    ///
    /// # Safety
    /// Flush _must_ set the entity location to the correct [`ArchetypeId`] for the given [`Entity`]
    /// each time init is called. This _can_ be [`ArchetypeId::INVALID`], provided the [`Entity`]
    /// has not been assigned to an [`Archetype`][crate::archetype::Archetype].
    ///
    /// Note: freshly-allocated entities (ones which don't come from the pending list) are guaranteed
    /// to be initialized with the invalid archetype.
    pub unsafe fn flush(&mut self, mut init: impl FnMut(Entity, &mut EntityLocation)) {
        // flush appended reservations
        self.coordinator.flush_appended(
            &mut self.meta,
            |meta, _index| {
                // We shouldn't flush owned entities.
                meta.location == EntityLocation::INVALID
            },
            |meta, index| {
                init(
                    Entity::from_raw_and_generation(index, meta.generation),
                    &mut meta.location,
                );
            },
        );

        // update `wild_pending_chunks`
        self.coordinator.get_new_pending_chunks(|new_chunks| {
            self.wild_pending_chunks.extend(new_chunks);
        });

        // flush and reuse `wild_pending_chunks`
        let hot_swap_arc = Arc::new(PendingEntitiesChunk::default());
        self.wild_pending_chunks.retain_mut(|item| {
            // flush the pending list
            item.flush(|reserved| {
                // SAFETY: The entity was pending, so it must have existed at some point, so the index is valid.
                let meta = unsafe { self.meta.get_unchecked_mut(reserved.index() as usize) };
                // We shouldn't flush owned entities or those already flushed.
                // For example, one may have been reserved, freed, and reserved again, potentially causing a double flush.
                if meta.location == EntityLocation::INVALID {
                    init(reserved, &mut meta.location);
                }
            });

            // see if we can get rid of it
            let pending = mem::replace(item, hot_swap_arc.clone());
            match Arc::try_unwrap(pending) {
                Ok(inner) => {
                    self.coordinator.reuse_pending(inner);
                    false
                }
                Err(still_pending) => {
                    let _hot_swapped = mem::replace(item, still_pending);
                    true
                }
            }
        });

        // balance owned entities
        if self.owned.len() > self.ideal_owned.get() as usize {
            let offload_range = self.ideal_owned.get() as usize..;
            for no_longer_owned in self.owned[offload_range.clone()].iter() {
                // SAFETY: Owned entities are known to have valid indices.
                unsafe {
                    self.meta
                        .get_unchecked_mut(no_longer_owned.index() as usize)
                }
                .location = EntityLocation::INVALID;
            }
            let drain = self.owned.drain(offload_range);
            self.coordinator.pending_mut(|pending| {
                pending.entities.extend(drain);
            });
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
        self.coordinator.flushed_meta_len.load(Ordering::Relaxed) as usize
    }

    /// The count of all entities in the [`World`] that are used,
    /// including both those allocated and those reserved, but not those freed.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn used_count(&self) -> usize {
        self.meta
            .iter()
            .filter(|meta| meta.location != EntityLocation::OWNED)
            .count()
    }

    /// The count of all entities in the [`World`] that have ever been allocated or reserved, including those that are freed.
    /// This is the value that [`Self::total_count()`] would return if [`Self::flush()`] were called right now.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn total_prospective_count(&self) -> usize {
        self.coordinator.meta_len.load(Ordering::Relaxed) as usize
    }

    /// The count of currently allocated entities.
    #[inline]
    pub fn len(&self) -> u32 {
        self.meta
            .iter()
            .filter(|meta| meta.location.archetype_id != ArchetypeId::INVALID)
            .count() as u32
    }

    /// Checks if any entity is currently active.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.meta
            .iter()
            .all(|meta| meta.location.archetype_id != ArchetypeId::INVALID)
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

    /// location for **owned entity**. See [`Entities::owned`].
    pub(crate) const OWNED: EntityLocation = EntityLocation {
        archetype_id: ArchetypeId::INVALID,
        archetype_row: ArchetypeRow::new(0),
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
        assert!(entities
            .free_current_and_future_generations(entity, 1)
            .is_some());
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
