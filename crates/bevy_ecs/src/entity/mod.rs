//! This module contains all entity types and utilities for interacting with their ids.
//!
//! # What is an Entity?
//!
//! The ecs [docs](crate) give an overview of what entities are and generally how to use them.
//! These docs provide more detail into how they actually work.
//! In these docs [`Entity`] and "entity id" are synonymous and refer to the [`Entity`] type, which identifies an entity.
//! The term "entity" used on its own refers to the "thing"/"game object" that id references.
//!
//! # In this Module
//!
//! This module contains four main things:
//!
//!  - Core ECS types like [`Entity`], [`Entities`], and [`EntityAllocator`].
//!  - Utilities for [`Entity`] ids like [`MapEntities`], [`EntityHash`], and [`UniqueEntityVec`].
//!  - Helpers for entity tasks like [`EntityCloner`].
//!  - Entity-related error types like [`EntityNotSpawnedError`].
//!
//! # Entity Life Cycle
//!
//! Entities have life cycles.
//! They are created, used for a while, and eventually destroyed.
//! Let's start from the top:
//!
//! **Spawn:** An entity is created.
//! In bevy, this is called spawning.
//! Most commonly, this is done through [`World::spawn`](crate::world::World::spawn) or [`Commands::spawn`](crate::system::Commands::spawn).
//! This creates a fresh entity in the world and returns its [`Entity`] id, which can be used to interact with the entity it identifies.
//! These methods initialize the entity with a [`Bundle`], a group of [components](crate::component::Component) that it starts with.
//! It is also possible to use [`World::spawn_empty`](crate::world::World::spawn_empty) or [`Commands::spawn_empty`](crate::system::Commands::spawn_empty), which are similar but do not add any components to the entity.
//! In either case, the returned [`Entity`] id is used to further interact with the entity.
//!
//! **Update:** Once an entity is created, you will need its [`Entity`] id to perform further actions on it.
//! This can be done through [`World::entity_mut`](crate::world::World::entity_mut) and [`Commands::entity`](crate::system::Commands::entity).
//! Even if you don't store the id, you can still find the entity you spawned by searching for it in a [`Query`].
//! Queries are also the primary way of interacting with an entity's components.
//! You can use [`EntityWorldMut::remove`](crate::world::EntityWorldMut::remove) and [`EntityCommands::remove`](crate::system::EntityCommands::remove) to remove components,
//! and you can use [`EntityWorldMut::insert`](crate::world::EntityWorldMut::insert) and [`EntityCommands::insert`](crate::system::EntityCommands::insert) to insert more components.
//! Be aware that each entity can only have 0 or 1 values for each kind of component, so inserting a bundle may overwrite existing component values.
//! This can also be further configured based on the insert method.
//!
//! **Despawn:** Despawn an entity when it is no longer needed.
//! This destroys it and all its components.
//! The entity is no longer reachable through the [`World`], [`Commands`], or [`Query`]s.
//! Note that this means an [`Entity`] id may refer to an entity that has since been despawned!
//! Not all [`Entity`] ids refer to active entities.
//! If an [`Entity`] id is used when its entity has been despawned, an [`EntityNotSpawnedError`] is emitted.
//! Any [`System`](crate::system) could despawn any entity; even if you never share its id, it could still be despawned unexpectedly.
//! Your code should do its best to handle these errors gracefully.
//!
//! In short:
//!
//! - Entities are spawned through methods like [`World::spawn`](crate::world::World::spawn), which return an [`Entity`] id for the new entity.
//! - Once spawned, they can be accessed and modified through [`Query`]s and other apis.
//! - You can get the [`Entity`] id of an entity through [`Query`]s, so losing an [`Entity`] id is not a problem.
//! - Entities can have components inserted and removed via [`World::entity_mut`](crate::world::World::entity_mut) and [`Commands::entity`](crate::system::Commands::entity).
//! - Entities are eventually despawned, destroying the entity and causing its [`Entity`] id to no longer refer to an entity.
//! - Not all [`Entity`] ids point to actual entities, which makes many entity methods fallible.
//!
//! # Entity Ids
//!
//! As mentioned entities each have an [`Entity`] id, which is used to interact with that entity.
//! But what actually is this id?
//! This [`Entity`] id is the combination of two ideas: [`EntityRow`] and [`EntityGeneration`].
//! You can think of the [`Entity`] type as a `struct Entity { row: u32, generation: u32 }`.
//!
//! To understand these ids, picture the ECS [`World`] as a spreadsheet.
//! Each kind of component is represented by a column in the spreadsheet and each entity is a row.
//! That's what the `row` does in [`Entity`]; it identifies where in the spreadsheet to find component values.
//! If an entity doesn't have a component, picture leaving the cell at that entity row and component column blank or `None`.
//! To find the component values of an entity, Bevy searches through the spreadsheet at the [`EntityRow`] for the entity and the [`ComponentId`](crate::component::ComponentId) for the component.
//!
//! An [`EntityRow`] always references exactly 1 entity in the [`World`].
//! Think about it, even if the spreadsheet only *uses* rows 1, 2, and 12, it still *has* millions of rows.
//! In the spreadsheet analogy, you can think of each row as being in one of 3 states:
//!
//!  1. The row is not used.
//!     Think of this as graying out the row or otherwise hiding it.
//!     This row doesn't just have no components; it isn't even participating at this point.
//!  2. The row is empty.
//!     The row is being used; it's visible, discoverable, etc; it just happens to not have any component values.
//!  3. The row is full.
//!     This is the "normal" state of a row.
//!     It has some component values and is being used.
//!
//! [`EntityRow`] behaves much the same way as the spreadsheet row.
//! Each row has a [`Option<EntityLocation>`](EntityLocation) which defines that row/entity's state.
//! If this is `Some`, the row is considered spawned (think *used* in the spreadsheet), otherwise it is considered despawned (think *grayed out* in the spreadsheet).
//! Only spawned entities, entities with `Some` [`EntityLocation`], participate in the [`World`].
//! The [`EntityLocation`] further describes which components an entity has and where to find them; it determines which spreadsheet cells are blank and which ones have values.
//! Only spawned rows are discoverable through [`Query`]s, etc.
//!
//! With that spreadsheet intuition, lets get a bit more precise with some definitions:
//!
//! - An entity that is used, not grayed out in the spreadsheet, is considered *spawned*.
//! - An entity that is is grayed out in the spreadsheet, not used, is considered *despawned*.
//! - A spawned entity that has no components is considered *empty* or *void* though still *spawned*.
//!   This is different from despawned since these are still participating, discoverable through queries and interact-able through commands;
//!   they just happen to have no components.
//!
//! An [`EntityRow`] always references exactly 1 entity in the [`World`]; they always exist (even though they may still be despawned).
//! This differs from [`Entity`] which references 0 or 1 entities, depending on if the entity it refers to still exists.
//! The rows are represented with 32 bits, so there are always over 4 billion entities in the world.
//! However, not all these entities are usable or stored in memory; Bevy doesn't store information for rows that have never been spawned.
//!
//! Rows can be repeatedly spawned and despawned.
//! Each spawning and despawning corresponds to an [`EntityGeneration`].
//! The first time a row is spawned, it has a generation of 0, and when it is despawned, it gets a generation of 1.
//! This differentiates each spawning of that [`EntityRow`].
//! Again, all an [`Entity`] id is is an [`EntityRow`] (where to find the component values) and an [`EntityGeneration`] (which version of that row it references).
//! When an [`Entity`] id is invalid, it just means that that generation of its row has been despawned.
//! It could still be despawned or it could have been re-spawned after it was despawned.
//! Either way, that row-generation pair no longer exists.
//! This produces the [`InvalidEntityError`].
//!
//! As mentioned, once an [`EntityRow`] is despawned, it is not discoverable until it is spawned again.
//! To prevent these rows from being forgotten, bevy tracks them in an [`EntityAllocator`].
//! When a new entity is spawned, all bevy does is allocate a new [`Entity`] id from the allocator and [`World::spawn_at`](crate::world::World::spawn_at) it.
//! When it is despawned, all bevy does is [`World::despawn_no_free`](crate::world::World::despawn_no_free) it and return the [`Entity`] id (with the next [`EntityGeneration`] for that [`EntityRow`]) to the allocator.
//! It's that simple.
//!
//! Bevy exposes this functionality as well.
//! Spawning an entity requires full access to the [`World`], but using [`EntityAllocator::alloc`] can be done fully concurrently.
//! Of course, to make that entity usable, it will need to be passed to [`World::spawn_at`](crate::world::World::spawn_at).
//! Managing entity ids manually is advanced but can be very useful for concurrency, custom entity allocators, etc.
//! But there are risks when used improperly:
//! Losing a despawned entity row without returning it to bevy's allocator will cause that row to be unreachable, effectively a memory leak.
//! Further, spawning an arbitrary [`EntityRow`] can cause problems if that same row is queued for reuse in the allocator.
//! This is powerful, but use it with caution.
//!
//! Lots of information about the state of an [`EntityRow`] can be obtained through [`Entities`].
//! For example, this can be used to get the most recent [`Entity`] of an [`EntityRow`] in [`Entities::resolve_from_row`].
//! See its docs for more.
//!
//! In general, you won't need to interact with the [`Entities`] type directly, but you will still feel its impact, largely because of its error types.
//! The big three to be aware of are:
//!
//! - [`InvalidEntityError`]: when the [`EntityGeneration`] of an [`Entity`] unexpectedly does not match the most recent generation of its [`EntityRow`],
//! - [`EntityValidButNotSpawnedError`]: when an [`EntityRow`] unexpectedly is not spawned,
//! - [`EntityNotSpawnedError`]: the union of the other two; when [`Entity`]'s [`EntityGeneration`] doesn't match the most recent generation of its [`EntityRow`] or when that row is not spawned,
//!
//!
//! To summarize:
//!
//! - An [`Entity`] id is just a [`EntityRow`] and a [`EntityGeneration`] of that row.
//! - [`EntityRow`]s can be spawned and despawned repeatedly, where each spawning gets its own [`EntityGeneration`].
//! - Bevy exposes this functionality through [`EntityAllocator::alloc`], [`World::spawn_at`](crate::world::World::spawn_at), and [`World::despawn_no_free`](crate::world::World::despawn_no_free).
//! - While understanding these details help build an intuition for how bevy handles entities, using these apis directly is risky but powerful,
//!   and you can usually use the easier [`World::spawn`](crate::world::World::spawn`), and [`World::despawn`](crate::world::World::despawn).
//! - Lots of id information can be obtained from [`Entities`].
//! - Watch out for the errors [`InvalidEntityError`], [`EntityValidButNotSpawnedError`], and [`EntityNotSpawnedError`], explained above.
//!
//! # Storage
//!
//! As mentioned above, an ecs [`World`] can be imagined as a spreadsheet.
//! One way that spreadsheet could be implemented is a list of [`Entity`]s and a hashmap for each component that maps an [`EntityRow`] to a component value if that row has the entity.
//! Bevy's ECS is quite different from that implementation (and much, much faster).
//! For details on how component storage actually works, see [`storage`](crate::storage).
//!
//! Regardless, the spreadsheet also needs a special column that tracks metadata about an entity.
//! This column does not represents a component and is specific to the [`EntityRow`], not the [`Entity`].
//! For example, one thing Bevy stores in this metadata is the current [`EntityGeneration`] of the row.
//! It also stores more information like the [`Tick`] a row was last spawned or despawned, and the optional [`EntityLocation`] itself.
//! For more information about what's stored here, see [`Entities`], Bevy's implementation of this special column.
//!
//! Entity spawning is done in two stages:
//! First we create an entity id (alloc step), and then we spawn it (add components and other metadata).
//! The reason for this split is that we need to be able to assign entity ids concurrently,
//! while for spawning we need exclusive (non-concurrent) access to the world.
//! That leaves three states for any given [`EntityRow`], where those two spawning stages serve to transition between states.
//! First, a row could be unallocated; only the allocator has any knowledge of this [`Entity`].
//! Second, the row is allocated, making it a "null" entity; it exists, and other code can have knowledge of its existence, but it is not spawned.
//! Third, the row is spawned, adding any (or no) components to the entity; it is now discoverable through queries, etc.
//!
//! [`World`]: crate::world::World
//! [`Query`]: crate::system::Query
//! [`Bundle`]: crate::bundle::Bundle
//! [`Component`]: crate::component::Component
//! [`Commands`]: crate::system::Commands

mod clone_entities;
mod entity_set;
mod map_entities;
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
    change_detection::{CheckChangeTicks, MaybeLocation, Tick},
    storage::{SparseSetIndex, TableId, TableRow},
};
use alloc::vec::Vec;
use bevy_platform::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use core::{fmt, hash::Hash, mem, num::NonZero, panic::Location};
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

    /// Equivalent to [`new`](Self::new) except that it takes a `u32` instead of a `NonMaxU32`.
    ///
    /// Returns `None` if the index is `u32::MAX`.
    pub const fn from_raw_u32(index: u32) -> Option<Self> {
        match NonMaxU32::new(index) {
            Some(index) => Some(Self(index)),
            None => None,
        }
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
/// This should be treated as a opaque identifier, and its internal representation may be subject to change.
///
/// # Aliasing
///
/// Internally [`EntityGeneration`] wraps a `u32`, so it can't represent *every* possible generation.
/// Eventually, generations can (and do) wrap or alias.
/// This can cause [`Entity`] and [`EntityGeneration`] values to be equal while still referring to different conceptual entities.
/// This can cause some surprising behavior:
///
/// ```
/// # use bevy_ecs::entity::EntityGeneration;
/// let (aliased, did_alias) = EntityGeneration::FIRST.after_versions(1u32 << 31).after_versions_and_could_alias(1u32 << 31);
/// assert!(did_alias);
/// assert!(EntityGeneration::FIRST == aliased);
/// ```
///
/// This can cause some unintended side effects.
/// See [`Entity`] docs for practical concerns and how to minimize any risks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Display)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(opaque))]
#[cfg_attr(feature = "bevy_reflect", reflect(Hash, PartialEq, Debug, Clone))]
#[repr(transparent)]
pub struct EntityGeneration(u32);

impl EntityGeneration {
    /// Represents the first generation of an [`EntityRow`].
    pub const FIRST: Self = Self(0);

    /// Non-wrapping difference between two generations after which a signed interpretation becomes negative.
    const DIFF_MAX: u32 = 1u32 << 31;

    /// Gets some bits that represent this value.
    /// The bits are opaque and should not be regarded as meaningful.
    #[inline(always)]
    pub const fn to_bits(self) -> u32 {
        self.0
    }

    /// Reconstruct an [`EntityGeneration`] previously destructured with [`EntityGeneration::to_bits`].
    ///
    /// Only useful when applied to results from `to_bits` in the same instance of an application.
    #[inline]
    pub const fn from_bits(bits: u32) -> Self {
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

    /// Compares two generations.
    ///
    /// Generations that are later will be [`Greater`](core::cmp::Ordering::Greater) than earlier ones.
    ///
    /// ```
    /// # use bevy_ecs::entity::EntityGeneration;
    /// # use core::cmp::Ordering;
    /// let later_generation = EntityGeneration::FIRST.after_versions(400);
    /// assert_eq!(EntityGeneration::FIRST.cmp_approx(&later_generation), Ordering::Less);
    ///
    /// let (aliased, did_alias) = EntityGeneration::FIRST.after_versions(400).after_versions_and_could_alias(u32::MAX);
    /// assert!(did_alias);
    /// assert_eq!(EntityGeneration::FIRST.cmp_approx(&aliased), Ordering::Less);
    /// ```
    ///
    /// Ordering will be incorrect and [non-transitive](https://en.wikipedia.org/wiki/Transitive_relation)
    /// for distant generations:
    ///
    /// ```should_panic
    /// # use bevy_ecs::entity::EntityGeneration;
    /// # use core::cmp::Ordering;
    /// let later_generation = EntityGeneration::FIRST.after_versions(3u32 << 31);
    /// let much_later_generation = later_generation.after_versions(3u32 << 31);
    ///
    /// // while these orderings are correct and pass assertions...
    /// assert_eq!(EntityGeneration::FIRST.cmp_approx(&later_generation), Ordering::Less);
    /// assert_eq!(later_generation.cmp_approx(&much_later_generation), Ordering::Less);
    ///
    /// // ... this ordering is not and the assertion fails!
    /// assert_eq!(EntityGeneration::FIRST.cmp_approx(&much_later_generation), Ordering::Less);
    /// ```
    ///
    /// Because of this, `EntityGeneration` does not implement `Ord`/`PartialOrd`.
    #[inline]
    pub const fn cmp_approx(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        match self.0.wrapping_sub(other.0) {
            0 => Ordering::Equal,
            1..Self::DIFF_MAX => Ordering::Greater,
            _ => Ordering::Less,
        }
    }
}

/// Unique identifier for an entity in a [`World`].
/// Note that this is just an id, not the entity itself.
/// Further, the entity this id refers to may no longer exist in the [`World`].
/// For more information about entities, their ids, and how to use them, see the module [docs](crate::entity).
///
/// # Aliasing
///
/// Once an entity is despawned, it ceases to exist.
/// However, its [`Entity`] id is still present, and may still be contained in some data.
/// This becomes problematic because it is possible for a later entity to be spawned at the exact same id!
/// If this happens, which is rare but very possible, it will be logged.
///
/// Aliasing can happen without warning.
/// Holding onto a [`Entity`] id corresponding to an entity well after that entity was despawned can cause un-intuitive behavior for both ordering, and comparing in general.
/// To prevent these bugs, it is generally best practice to stop holding an [`Entity`] or [`EntityGeneration`] value as soon as you know it has been despawned.
/// If you must do otherwise, do not assume the [`Entity`] id corresponds to the same entity it originally did.
/// See [`EntityGeneration`]'s docs for more information about aliasing and why it occurs.
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
    /// Creates a new instance with the given index and generation.
    #[inline(always)]
    pub const fn from_row_and_generation(row: EntityRow, generation: EntityGeneration) -> Entity {
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
    pub const PLACEHOLDER: Self = Self::from_row(EntityRow::PLACEHOLDER);

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
    pub const fn from_row(row: EntityRow) -> Entity {
        Self::from_row_and_generation(row, EntityGeneration::FIRST)
    }

    /// This is equivalent to [`from_row`](Self::from_row) except that it takes a `u32` instead of an [`EntityRow`].
    ///
    /// Returns `None` if the row is `u32::MAX`.
    #[inline(always)]
    pub const fn from_raw_u32(row: u32) -> Option<Entity> {
        match NonMaxU32::new(row) {
            Some(row) => Some(Self::from_row(EntityRow::new(row))),
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

/// Outputs the short entity identifier, including the index and generation.
///
/// This takes the format: `{index}v{generation}`.
///
/// For [`Entity::PLACEHOLDER`], this outputs `PLACEHOLDER`.
///
/// For a unique [`u64`] representation, use [`Entity::to_bits`].
impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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
        Entity::from_row(EntityRow::get_sparse_set_index(value))
    }
}

/// Allocates [`Entity`] ids uniquely.
/// This is used in [`World::spawn_at`](crate::world::World::spawn_at) and [`World::despawn_no_free`](crate::world::World::despawn_no_free) to track entity ids no longer in use.
/// Allocating is fully concurrent and can be done from multiple threads.
///
/// Conceptually, this is a collection of [`Entity`] ids who's [`EntityRow`] is despawned and who's [`EntityGeneration`] is the most recent.
/// See the module docs for how these ids and this allocator participate in the life cycle of an entity.
#[derive(Default, Debug)]
pub struct EntityAllocator {
    /// All the entities to reuse.
    /// This is a buffer, which contains an array of [`Entity`] ids to hand out.
    /// The next id to hand out is tracked by `free_len`.
    free: Vec<Entity>,
    /// This is continually subtracted from.
    /// If it wraps to a very large number, it will be outside the bounds of `free`,
    /// and a new row will be needed.
    free_len: AtomicUsize,
    /// This is the next "fresh" row to hand out.
    /// If there are no rows to reuse, this row, which has a generation of 0, is the next to return.
    next_row: AtomicU32,
}

impl EntityAllocator {
    /// Restarts the allocator.
    pub(crate) fn restart(&mut self) {
        self.free.clear();
        *self.free_len.get_mut() = 0;
        *self.next_row.get_mut() = 0;
    }

    /// This allows `freed` to be retrieved from [`alloc`](Self::alloc), etc.
    /// Freeing an [`Entity`] such that one [`EntityRow`] is in the allocator in multiple places can cause panics when spawning the allocated entity.
    /// Additionally, to differentiate versions of an [`Entity`], updating the [`EntityGeneration`] before freeing is a good idea
    /// (but not strictly necessary if you don't mind [`Entity`] id aliasing.)
    pub fn free(&mut self, freed: Entity) {
        let expected_len = *self.free_len.get_mut();
        if expected_len > self.free.len() {
            self.free.clear();
        } else {
            self.free.truncate(expected_len);
        }
        self.free.push(freed);
        *self.free_len.get_mut() = self.free.len();
    }

    /// Allocates some [`Entity`].
    /// The result could have come from a [`free`](Self::free) or be a brand new [`EntityRow`].
    ///
    /// The returned entity is valid and unique, but it is not yet spawned.
    /// Using the id as if it were spawned may produce errors.
    /// It can not be queried, and it has no [`EntityLocation`].
    /// See module [docs](crate::entity) for more information about entity validity vs spawning.
    ///
    /// This is different from empty entities, which are spawned and
    /// just happen to have no components.
    ///
    /// These ids must be used; otherwise, they will be forgotten.
    /// For example, the result must be eventually used to either spawn an entity or be [`free`](Self::free)d.
    ///
    /// # Panics
    ///
    /// If there are no more entities available, this panics.
    ///
    ///
    /// # Example
    ///
    /// This is particularly useful when spawning entities in special ways.
    /// For example, [`Commands`](crate::system::Commands) uses this to allocate an entity and [`spawn_at`](crate::world::World::spawn_at) it later.
    /// But remember, since this entity is not queryable and is not discoverable, losing the returned [`Entity`] effectively leaks it, never to be used again!
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*};
    /// let mut world = World::new();
    /// let entity = world.entities_allocator().alloc();
    /// // wait as long as you like
    /// let entity_access = world.spawn_empty_at(entity).unwrap(); // or spawn_at(entity, my_bundle)
    /// // treat it as a normal entity
    /// entity_access.despawn();
    /// ```
    ///
    /// More generally, manually spawning and [`despawn_no_free`](crate::world::World::despawn_no_free)ing entities allows you to skip Bevy's default entity allocator.
    /// This is useful if you want to enforce properties about the [`EntityRow`]s of a group of entities, make a custom allocator, etc.
    pub fn alloc(&self) -> Entity {
        let index = self
            .free_len
            .fetch_sub(1, Ordering::Relaxed)
            .wrapping_sub(1);
        self.free.get(index).copied().unwrap_or_else(|| {
            let row = self.next_row.fetch_add(1, Ordering::Relaxed);
            let row = NonMaxU32::new(row).expect("too many entities");
            Entity::from_row(EntityRow::new(row))
        })
    }

    /// A more efficient way of calling [`alloc`](Self::alloc) repeatedly `count` times.
    /// See [`alloc`](Self::alloc) for details.
    ///
    /// Like [`alloc`](Self::alloc), these entities must be used, otherwise they will be forgotten.
    /// If the iterator is not exhausted, its remaining entities are forgotten.
    /// See [`AllocEntitiesIterator`] docs for more.
    pub fn alloc_many(&self, count: u32) -> AllocEntitiesIterator<'_> {
        let current_len = self.free_len.fetch_sub(count as usize, Ordering::Relaxed);
        let current_len = if current_len < self.free.len() {
            current_len
        } else {
            0
        };
        let start = current_len.saturating_sub(count as usize);
        let reuse = start..current_len;
        let still_need = (count as usize - reuse.len()) as u32;
        let new = if still_need > 0 {
            let start_new = self.next_row.fetch_add(still_need, Ordering::Relaxed);
            let end_new = start_new
                .checked_add(still_need)
                .expect("too many entities");
            start_new..end_new
        } else {
            0..0
        };
        AllocEntitiesIterator {
            reuse: self.free[reuse].iter(),
            new,
        }
    }
}

/// An [`Iterator`] returning a sequence of unique [`Entity`] values from [`Entities`].
/// Dropping this will still retain the entities as allocated; this is effectively a leak.
/// To prevent this, ensure the iterator is exhausted before dropping it.
pub struct AllocEntitiesIterator<'a> {
    reuse: core::slice::Iter<'a, Entity>,
    new: core::ops::Range<u32>,
}

impl<'a> Iterator for AllocEntitiesIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.reuse.next().copied().or_else(|| {
            self.new.next().map(|index| {
                // SAFETY: This came from an exclusive range so the max can't be hit.
                let row = unsafe { EntityRow::new(NonMaxU32::new_unchecked(index)) };
                Entity::from_row(row)
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.reuse.len() + self.new.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for AllocEntitiesIterator<'a> {}

impl<'a> core::iter::FusedIterator for AllocEntitiesIterator<'a> {}

// SAFETY: Newly allocated entity values are unique.
unsafe impl EntitySetIterator for AllocEntitiesIterator<'_> {}

/// [`Entities`] tracks all known [`EntityRow`]s and their metadata.
/// This is like a base table of information all entities have.
#[derive(Debug, Clone)]
pub struct Entities {
    meta: Vec<EntityMeta>,
}

impl Entities {
    pub(crate) const fn new() -> Self {
        Self { meta: Vec::new() }
    }

    /// Clears all entity information
    pub fn clear(&mut self) {
        self.meta.clear();
    }

    /// Returns the [`EntityLocation`] of an [`Entity`] if it is valid and spawned.
    /// This can error if the [`EntityGeneration`] of this entity has passed or if the [`EntityRow`] is not spawned.
    ///
    /// See the module [docs](crate::entity) for a full explanation of these ids, entity life cycles, and the meaning of this result.
    #[inline]
    pub fn get_spawned(&self, entity: Entity) -> Result<EntityLocation, EntityNotSpawnedError> {
        let meta = self.meta.get(entity.index() as usize);
        let meta = meta.unwrap_or(&EntityMeta::FRESH);
        if entity.generation() != meta.generation {
            return Err(EntityNotSpawnedError::Invalid(InvalidEntityError {
                entity,
                current_generation: meta.generation,
            }));
        };
        meta.location.ok_or(EntityNotSpawnedError::RowNotSpawned(
            EntityValidButNotSpawnedError {
                entity,
                location: meta.spawned_or_despawned.by,
            },
        ))
    }

    /// Returns the [`EntityLocation`] of an [`Entity`] if it is valid.
    /// The location may be `None` if the entity is not spawned.
    /// If you expect the entity to be spawned, use [`get_spawned`](Self::get_spawned).
    ///
    /// This can fail if the id's [`EntityGeneration`] has passed.
    ///
    /// See the module [docs](crate::entity) for a full explanation of these ids, entity life cycles, and the meaning of this result.
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<Option<EntityLocation>, InvalidEntityError> {
        match self.get_spawned(entity) {
            Ok(location) => Ok(Some(location)),
            Err(EntityNotSpawnedError::RowNotSpawned { .. }) => Ok(None),
            Err(EntityNotSpawnedError::Invalid(err)) => Err(err),
        }
    }

    /// Get the [`Entity`] for the given [`EntityRow`].
    /// Note that this entity may not be spawned yet.
    ///
    /// See the module [docs](crate::entity) for a full explanation of these ids, entity life cycles, and the meaning of this result.
    #[inline]
    pub fn resolve_from_row(&self, row: EntityRow) -> Entity {
        self.meta
            .get(row.index() as usize)
            .map(|meta| Entity::from_row_and_generation(row, meta.generation))
            .unwrap_or(Entity::from_row(row))
    }

    /// Returns whether the entity at this `row` is spawned or not.
    ///
    /// See the module [docs](crate::entity) for a full explanation of these ids, entity life cycles, and the meaning of this result.
    #[inline]
    pub fn is_row_spawned(&self, row: EntityRow) -> bool {
        self.meta
            .get(row.index() as usize)
            .is_some_and(|meta| meta.location.is_some())
    }

    /// Returns true if the entity is valid.
    /// This will return true for entities that are valid but have not been spawned.
    ///
    /// See the module [docs](crate::entity) for a full explanation of these ids, entity life cycles, and the meaning of this result.
    pub fn contains(&self, entity: Entity) -> bool {
        self.resolve_from_row(entity.row()).generation() == entity.generation()
    }

    /// Returns true if the entity is valid and is spawned.
    ///
    /// See the module [docs](crate::entity) for a full explanation of these ids, entity life cycles, and the meaning of this result.
    pub fn contains_spawned(&self, entity: Entity) -> bool {
        self.get_spawned(entity).is_ok()
    }

    /// Provides information regarding if `entity` may be safely spawned.
    /// This can error if the entity is invalid or is already spawned.
    ///
    /// See the module [docs](crate::entity) for a full explanation of these ids, entity life cycles, and the meaning of this result.
    #[inline]
    pub fn check_can_spawn_at(&self, entity: Entity) -> Result<(), SpawnError> {
        match self.get(entity) {
            Ok(Some(_)) => Err(SpawnError::AlreadySpawned),
            Ok(None) => Ok(()),
            Err(err) => Err(SpawnError::Invalid(err)),
        }
    }

    /// Updates the location of an [`EntityRow`].
    /// This must be called when moving the components of the existing entity around in storage.
    /// Returns the previous location of the row.
    ///
    /// # Safety
    ///  - The current location of the `row` must already be set. If not, use [`set_location`](Self::set_location).
    ///  - `location` must be valid for the entity at `row` or immediately made valid afterwards
    ///    before handing control to unknown code.
    #[inline]
    pub(crate) unsafe fn update_existing_location(
        &mut self,
        row: EntityRow,
        location: Option<EntityLocation>,
    ) -> Option<EntityLocation> {
        // SAFETY: Caller guarantees that `row` already had a location, so `declare` must have made the index valid already.
        let meta = unsafe { self.meta.get_unchecked_mut(row.index() as usize) };
        mem::replace(&mut meta.location, location)
    }

    /// Declares the location of an [`EntityRow`].
    /// This must be called when spawning entities, but when possible, prefer [`update_existing_location`](Self::update_existing_location).
    /// Returns the previous location of the row.
    ///
    /// # Safety
    ///  - `location` must be valid for the entity at `row` or immediately made valid afterwards
    ///    before handing control to unknown code.
    #[inline]
    pub(crate) unsafe fn set_location(
        &mut self,
        row: EntityRow,
        location: Option<EntityLocation>,
    ) -> Option<EntityLocation> {
        self.ensure_row_index_is_valid(row);
        // SAFETY: We just did `ensure_row`
        self.update_existing_location(row, location)
    }

    /// Ensures the row is within the bounds of [`Self::meta`], expanding it if necessary.
    #[inline]
    fn ensure_row_index_is_valid(&mut self, row: EntityRow) {
        #[cold] // to help with branch prediction
        fn expand(meta: &mut Vec<EntityMeta>, len: usize) {
            meta.resize(len, EntityMeta::FRESH);
            // Set these up too while we're here.
            meta.resize(meta.capacity(), EntityMeta::FRESH);
        }

        let index = row.index() as usize;
        if self.meta.len() <= index {
            // TODO: hint unlikely once stable.
            expand(&mut self.meta, index + 1);
        }
    }

    /// Marks the `row` as free, returning the [`Entity`] to reuse that [`EntityRow`].
    ///
    /// # Safety
    ///
    /// - `row` must be despawned (have no location) already.
    pub(crate) unsafe fn mark_free(&mut self, row: EntityRow, generations: u32) -> Entity {
        // We need to do this in case an entity is being freed that was never spawned.
        self.ensure_row_index_is_valid(row);
        // SAFETY: We just did `ensure_row`
        let meta = unsafe { self.meta.get_unchecked_mut(row.index() as usize) };

        let (new_generation, aliased) = meta.generation.after_versions_and_could_alias(generations);
        meta.generation = new_generation;
        if aliased {
            warn!("EntityRow({row}) generation wrapped on Entities::free, aliasing may occur",);
        }

        Entity::from_row_and_generation(row, meta.generation)
    }

    /// Mark an [`EntityRow`] as spawned or despawned in the given tick.
    ///
    /// # Safety
    ///  - `row` must have been spawned at least once, ensuring its row is valid.
    #[inline]
    pub(crate) unsafe fn mark_spawned_or_despawned(
        &mut self,
        row: EntityRow,
        by: MaybeLocation,
        tick: Tick,
    ) {
        // SAFETY: Caller guarantees that `row` already had a location, so `declare` must have made the index valid already.
        let meta = unsafe { self.meta.get_unchecked_mut(row.index() as usize) };
        meta.spawned_or_despawned = SpawnedOrDespawned { by, tick };
    }

    /// Try to get the source code location from which this entity has last been spawned or despawned.
    ///
    /// Returns `None` if the entity does not exist or has never been construced/despawned.
    pub fn entity_get_spawned_or_despawned_by(
        &self,
        entity: Entity,
    ) -> MaybeLocation<Option<&'static Location<'static>>> {
        MaybeLocation::new_with_flattened(|| {
            self.entity_get_spawned_or_despawned(entity)
                .map(|spawned_or_despawned| spawned_or_despawned.by)
        })
    }

    /// Try to get the [`Tick`] at which this entity has last been spawned or despawned.
    ///
    /// Returns `None` if the entity does not exist or has never been construced/despawned.
    pub fn entity_get_spawn_or_despawn_tick(&self, entity: Entity) -> Option<Tick> {
        self.entity_get_spawned_or_despawned(entity)
            .map(|spawned_or_despawned| spawned_or_despawned.tick)
    }

    /// Try to get the [`SpawnedOrDespawned`] related to the entity's last spawning or despawning.
    ///
    /// Returns `None` if the entity does not exist or has never been construced/despawned.
    #[inline]
    fn entity_get_spawned_or_despawned(&self, entity: Entity) -> Option<SpawnedOrDespawned> {
        self.meta
            .get(entity.index() as usize)
            .filter(|meta|
            // Generation is incremented immediately upon despawn
            (meta.generation == entity.generation)
            || (meta.location.is_none() && meta.generation == entity.generation.after_versions(1)))
            .map(|meta| meta.spawned_or_despawned)
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
        (meta.spawned_or_despawned.by, meta.spawned_or_despawned.tick)
    }

    #[inline]
    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        for meta in &mut self.meta {
            meta.spawned_or_despawned.tick.check_tick(check);
        }
    }

    /// The count of currently allocated entity rows.
    /// For information on active entities, see [`Self::count_spawned`].
    #[inline]
    pub fn len(&self) -> u32 {
        self.meta.len() as u32
    }

    /// Checks if any entity has been declared.
    /// For information on active entities, see [`Self::any_spawned`].
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Counts the number of entity rows currently spawned.
    /// See the module docs for a more precise explanation of what spawning means.
    /// Be aware that this is O(n) and is intended only to be used as a diagnostic for tests.
    pub fn count_spawned(&self) -> u32 {
        self.meta
            .iter()
            .filter(|meta| meta.location.is_some())
            .count() as u32
    }

    /// Returns true if there are any entity rows currently spawned.
    /// See the module docs for a more precise explanation of what spawning means.
    pub fn any_spawned(&self) -> bool {
        self.meta.iter().any(|meta| meta.location.is_some())
    }
}

/// An error that occurs when a specified [`Entity`] can not be spawned.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnError {
    /// The [`Entity`] to spawn was invalid.
    /// It probably had the wrong generation or was created erroneously.
    #[error("Invalid id: {0}")]
    Invalid(InvalidEntityError),
    /// The [`Entity`] to spawn was already spawned.
    #[error("The entity can not be spawned as it already has a location.")]
    AlreadySpawned,
}

/// An error that occurs when a specified [`Entity`] does not exist in the entity id space.
/// See [module](crate::entity) docs for more about entity validity.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("The entity with ID {entity} is invalid; its row now has generation {current_generation}.")]
pub struct InvalidEntityError {
    /// The entity's ID.
    pub entity: Entity,
    /// The generation of the [`EntityRow`], which did not match the requested entity.
    pub current_generation: EntityGeneration,
}

/// An error that occurs when a specified [`Entity`] is certain to be valid and is expected to be spawned but is spawned.
/// This includes when an [`EntityRow`] is requested but is not spawned, since each row always corresponds to exactly one valid entity.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityValidButNotSpawnedError {
    /// The entity's ID.
    pub entity: Entity,
    /// The location of what last despawned the entity.
    pub location: MaybeLocation<&'static Location<'static>>,
}

impl fmt::Display for EntityValidButNotSpawnedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entity = self.entity;
        match self.location.into_option() {
            Some(location) => write!(f, "The entity with ID {entity} is not spawned; its row was last despawned by {location}."),
            None => write!(
                f,
                "The entity with ID {entity} is not spawned; enable `track_location` feature for more details."
            ),
        }
    }
}

/// An error that occurs when a specified [`Entity`] is expected to be valid and spawned but is not.
/// Represents an error of either [`InvalidEntityError`] (when the entity is invalid) or [`EntityValidButNotSpawnedError`] (when the [`EntityGeneration`] is correct but the [`EntityRow`] is not spawned).
#[derive(thiserror::Error, Copy, Clone, Debug, Eq, PartialEq)]
pub enum EntityNotSpawnedError {
    /// The entity was invalid.
    #[error("{0}")]
    Invalid(#[from] InvalidEntityError),
    /// The entity was valid but was not spawned.
    #[error("{0}")]
    RowNotSpawned(#[from] EntityValidButNotSpawnedError),
}

impl EntityNotSpawnedError {
    /// The entity that did not exist or was not spawned.
    pub fn entity(&self) -> Entity {
        match self {
            EntityNotSpawnedError::Invalid(err) => err.entity,
            EntityNotSpawnedError::RowNotSpawned(err) => err.entity,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct EntityMeta {
    /// The current [`EntityGeneration`] of the [`EntityRow`].
    generation: EntityGeneration,
    /// The current location of the [`EntityRow`].
    location: Option<EntityLocation>,
    /// Location and tick of the last spawn/despawn
    spawned_or_despawned: SpawnedOrDespawned,
}

#[derive(Copy, Clone, Debug)]
struct SpawnedOrDespawned {
    by: MaybeLocation,
    tick: Tick,
}

impl EntityMeta {
    /// The metadata for a fresh entity: Never spawned/despawned, no location, etc.
    const FRESH: EntityMeta = EntityMeta {
        generation: EntityGeneration::FIRST,
        location: None,
        spawned_or_despawned: SpawnedOrDespawned {
            by: MaybeLocation::caller(),
            tick: Tick::new(0),
        },
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
        let r = EntityRow::from_raw_u32(0xDEADBEEF).unwrap();
        assert_eq!(EntityRow::from_bits(r.to_bits()), r);

        let e = Entity::from_row_and_generation(
            EntityRow::from_raw_u32(0xDEADBEEF).unwrap(),
            EntityGeneration::from_bits(0x5AADF00D),
        );
        assert_eq!(Entity::from_bits(e.to_bits()), e);
    }

    #[test]
    fn entity_const() {
        const C1: Entity = Entity::from_row(EntityRow::from_raw_u32(42).unwrap());
        assert_eq!(42, C1.index());
        assert_eq!(0, C1.generation().to_bits());

        const C2: Entity = Entity::from_bits(0x0000_00ff_0000_00cc);
        assert_eq!(!0x0000_00cc, C2.index());
        assert_eq!(0x0000_00ff, C2.generation().to_bits());

        const C3: u32 = Entity::from_row(EntityRow::from_raw_u32(33).unwrap()).index();
        assert_eq!(33, C3);

        const C4: u32 = Entity::from_bits(0x00dd_00ff_1111_1111)
            .generation()
            .to_bits();
        assert_eq!(0x00dd_00ff, C4);
    }

    #[test]
    #[expect(
        clippy::nonminimal_bool,
        reason = "This intentionally tests all possible comparison operators as separate functions; thus, we don't want to rewrite these comparisons to use different operators."
    )]
    fn entity_comparison() {
        assert_eq!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ),
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            )
        );
        assert_ne!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(789)
            ),
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            )
        );
        assert_ne!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ),
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(789)
            )
        );
        assert_ne!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ),
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(456).unwrap(),
                EntityGeneration::from_bits(123)
            )
        );

        // ordering is by generation then by index

        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ) >= Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            )
        );
        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ) <= Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            )
        );
        assert!(
            !(Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ) < Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ))
        );
        assert!(
            !(Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ) > Entity::from_row_and_generation(
                EntityRow::from_raw_u32(123).unwrap(),
                EntityGeneration::from_bits(456)
            ))
        );

        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(9).unwrap(),
                EntityGeneration::from_bits(1)
            ) < Entity::from_row_and_generation(
                EntityRow::from_raw_u32(1).unwrap(),
                EntityGeneration::from_bits(9)
            )
        );
        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(1).unwrap(),
                EntityGeneration::from_bits(9)
            ) > Entity::from_row_and_generation(
                EntityRow::from_raw_u32(9).unwrap(),
                EntityGeneration::from_bits(1)
            )
        );

        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(1).unwrap(),
                EntityGeneration::from_bits(1)
            ) > Entity::from_row_and_generation(
                EntityRow::from_raw_u32(2).unwrap(),
                EntityGeneration::from_bits(1)
            )
        );
        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(1).unwrap(),
                EntityGeneration::from_bits(1)
            ) >= Entity::from_row_and_generation(
                EntityRow::from_raw_u32(2).unwrap(),
                EntityGeneration::from_bits(1)
            )
        );
        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(2).unwrap(),
                EntityGeneration::from_bits(2)
            ) < Entity::from_row_and_generation(
                EntityRow::from_raw_u32(1).unwrap(),
                EntityGeneration::from_bits(2)
            )
        );
        assert!(
            Entity::from_row_and_generation(
                EntityRow::from_raw_u32(2).unwrap(),
                EntityGeneration::from_bits(2)
            ) <= Entity::from_row_and_generation(
                EntityRow::from_raw_u32(1).unwrap(),
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
        let first_hash =
            hash.hash_one(Entity::from_row(EntityRow::from_raw_u32(first_id).unwrap()));

        for i in 1..=255 {
            let id = first_id + i;
            let hash = hash.hash_one(Entity::from_row(EntityRow::from_raw_u32(id).unwrap()));
            assert_eq!(first_hash.wrapping_sub(hash) as u32, i);
        }
    }

    #[test]
    fn entity_hash_id_bitflip_affects_high_7_bits() {
        use core::hash::BuildHasher;

        let hash = EntityHash;

        let first_id = 0xC0FFEE;
        let first_hash =
            hash.hash_one(Entity::from_row(EntityRow::from_raw_u32(first_id).unwrap())) >> 57;

        for bit in 0..u32::BITS {
            let id = first_id ^ (1 << bit);
            let hash = hash.hash_one(Entity::from_row(EntityRow::from_raw_u32(id).unwrap())) >> 57;
            assert_ne!(hash, first_hash);
        }
    }

    #[test]
    fn entity_generation_is_approximately_ordered() {
        use core::cmp::Ordering;

        let old = EntityGeneration::FIRST;
        let middle = old.after_versions(1);
        let younger_before_ord_wrap = middle.after_versions(EntityGeneration::DIFF_MAX);
        let younger_after_ord_wrap = younger_before_ord_wrap.after_versions(1);

        assert_eq!(middle.cmp_approx(&old), Ordering::Greater);
        assert_eq!(middle.cmp_approx(&middle), Ordering::Equal);
        assert_eq!(middle.cmp_approx(&younger_before_ord_wrap), Ordering::Less);
        assert_eq!(
            middle.cmp_approx(&younger_after_ord_wrap),
            Ordering::Greater
        );
    }

    #[test]
    fn entity_debug() {
        let entity = Entity::from_row(EntityRow::from_raw_u32(42).unwrap());
        let string = format!("{entity:?}");
        assert_eq!(string, "42v0");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{entity:?}");
        assert_eq!(string, "PLACEHOLDER");
    }

    #[test]
    fn entity_display() {
        let entity = Entity::from_row(EntityRow::from_raw_u32(42).unwrap());
        let string = format!("{entity}");
        assert_eq!(string, "42v0");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{entity}");
        assert_eq!(string, "PLACEHOLDER");
    }

    #[test]
    fn allocator() {
        let mut allocator = EntityAllocator::default();
        let mut entities = allocator.alloc_many(2048).collect::<Vec<_>>();
        for _ in 0..2048 {
            entities.push(allocator.alloc());
        }

        let pre_len = entities.len();
        entities.sort();
        entities.dedup();
        assert_eq!(pre_len, entities.len());

        for e in entities.drain(..) {
            allocator.free(e);
        }

        entities.extend(allocator.alloc_many(5000));
        let pre_len = entities.len();
        entities.sort();
        entities.dedup();
        assert_eq!(pre_len, entities.len());
    }
}
