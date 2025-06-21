//! This module contains all entity types and utilities for interacting with their ids.
//!
//! # What is an Entity?
//!
//! An entity is a thing that exists in a [`World`].
//! Entities have zero or more [`Component`]s, which are just data associated with the entity.
//! Entities serve the same purpose as things like game objects from Unity or nodes from Godot:
//! They can be created, have their data accessed and changed, and are eventually deleted.
//! In bevy, an entity can represent anything from a player character to the game's window itself (and everything in-between!)
//! However, unlike other engines, entities are not represented as large class objects.
//! This makes bevy much faster in principal, but it also leads to some perhaps unintuitive differences that need to be considered.
//!
//! Because entities are not traditional, object-oriented, garbage collected, class instances, more effort is needed to interact with them.
//! The biggest difference is that the [`Entity`] type does *not* represent a conceptual entity.
//! In other words, an entity's data, its components, are not stored within the [`Entity`] type.
//! Instead, the [`Entity`] acts as an id, and it's components are stored separate from its id in the [`World`].
//! In fact, one way to think about entities and their data is to imagine each world as a list of entity ids
//! and a hashmap for each component which maps [`Entity`] values to component values if the entity has that component.
//! Of course, the [`World`] is really quite different from this and much more efficient, but thinking about it this way will be helpful to understand how entities work.
//! Put another way, the world can be thought of as a big spreadsheet, where each component type has a column and each entity has a row.
//! In order to get an entity's components, bevy finds the values by looking up the [`Entity`]'s [`EntityRow`] and the [`Component`](crate::component::Component)'s [`ComponentId`](crate::component::ComponentId).
//! Interacting with an entity can be done through three main interfaces:
//! Use the [`World`] with methods like [`World::entity`](crate::world::World::entity) for complete and immediate access to an entity and its components.
//! Use [`Query`]s for very fast access to component values.
//! Use [`Commands`] with methods like [`Commands::entity`](crate::system::Commands::entity) for delayed but unrestricted access to entities.
//!
//! In short:
//!
//! - An entity is a thing in the world, similar to game objects from Unity or nodes from Godot.
//! - Entities can represent anything! (players, items, and the app window itself)
//! - Entities have data attached to them called [`Component`]s. (health on the player, damage on the item, and resolution on the window)
//! - The [`Entity`] type is an id for its entity; it is not the entity itself and does not store component data directly.
//! - To access an entity's data, use [`World`], [`Query`], or [`Commands`] apis.
//!
//! # Entity Life Cycle
//!
//! Entities have life cycles.
//! They are created, used for a while, and eventually destroyed.
//! Let's start from the top:
//!
//! **Spawn:** An entity is crated.
//! In bevy, this is called spawning.
//! Most commonly, this is done through [`World::spawn`](crate::world::World::spawn) or [`Commands::spawn`](crate::system::Commands::spawn).
//! This creates a fresh entity in the world and returns its [`Entity`] id, which can be used to interact with the entity.
//! These methods initialize the entity with a [`Bundle`], which is a group of [components](crate::component::Component) that it starts with.
//! It is also possible to use [`World::spawn_empty`](crate::world::World::spawn_empty) or [`Commands::spawn_empty`](crate::system::Commands::spawn_empty), which are similar but do not add any components to the entity.
//! In either case, the returned [`Entity`] id is used to further interact with the entity.
//! Once an entity is created, you will need its [`Entity`] id to progress the entity through its life cycle.
//! This can be done through [`World::entity_mut`](crate::world::World::entity_mut) and [`Commands::entity`](crate::system::Commands::entity).
//! Even if you don't store the id, you can still find the entity you spawned by searching for it in a [`Query`].
//!
//! **Insert:** Once an entity has been created, additional [`Bundle`]s can be inserted onto the entity.
//! There are lots of ways to do this and lots of ways to configure what to do when a component in the bundle is already present on the entity.
//! Each entity can only have 0 or 1 values for a component.
//! See [`EntityWorldMut::insert`](crate::world::EntityWorldMut::insert) and [`EntityCommands::insert`](crate::system::EntityCommands::insert) for a start on how to do this.
//!
//! **Remove:** Components on an entity can be removed as well.
//! See [`EntityWorldMut::remove`](crate::world::EntityWorldMut::remove) and [`EntityCommands::remove`](crate::system::EntityCommands::remove) for a start on how to do this.
//!
//! **Despawn:** Despawn an entity when it is no longer needed.
//! This destroys it and all its components.
//! The entity is no longer reachable through the [`World`], [`Commands`], or [`Query`]s.
//! Note that this means an [`Entity`] id may refer to an entity that has since been despawned!
//! Not all [`Entity`] ids refer to active entities.
//! If an [`Entity`] id is used when its entity no longer exists, an [`EntityDoesNotExistError`] is emitted.
//! Any [`System`](crate::system) could despawn entities; even if you never share an entity's id, it could still be despawned unexpectedly.
//! Handle these errors gracefully.
//!
//! In short:
//!
//! - Entities are spawned through methods like [`World::spawn`](crate::world::World::spawn), which return an [`Entity`] id for the new entity.
//! - Once spawned, they can be accessed and modified through [`Query`]s and other apis.
//! - You can get the [`Entity`] id of an entity through [`Query`]s, so loosing an [`Entity`] id is not a problem.
//! - Entities can have components inserted and removed via [`World::entity_mut`](crate::world::World::entity_mut) and [`Commands::entity`](crate::system::Commands::entity).
//! - Entities are eventually despawned, destroying the entity and causing its [`Entity`] id to no longer refer to an entity.
//! - Not all [`Entity`] ids point to actual entities, which makes many entity methods fallible.
//!
//! # Entity Ids
//!
//! As mentioned entities each have an [`Entity`] id, which is used to interact with that entity.
//! But what actually is this id?
//! This [`Entity`] id is the combination of two ideas: [`EntityRow`] and [`EntityGeneration`].
//!
//! An [`EntityRow`] always references exactly 1 entity in the [`World`]; they are always valid.
//! This differs from [`Entity`] which references 0 or 1 entities, depending on if the entity it refers to still exists.
//! Each [`EntityRow`] refers to an entity, and each entity has an [`EntityRow`].
//! The rows are represented with 32 bits, so there are always over 4 billion entities in the world.
//! However, not all these entities are usable or stored in memory.
//! To understand why, let's look at the states an entity row can be in:
//!
//! Each [`EntityRow`] has a [`EntityIdLocation`] which defines that row/entity's state.
//! The [`EntityIdLocation`] is an `Option` of [`EntityLocation`].
//! If this is `Some`, the row is considered constructed, otherwise it is considered destructed.
//! Only constructed entities, entities with `Some` [`EntityLocation`], participate in the [`World`].
//! The [`EntityLocation`] further describes which components an entity has and where to find them.
//! That means each entity row can be in three states: 1) It has some components, 2) It has no components *empty*, 3) It has no location *null*.
//! Only non-null entities are discoverable through [`Query`]s, etc.
//!
//! Rows can be repeatedly constructed and destructed.
//! Each construction and destruction corresponds to a [`EntityGeneration`].
//! The first time a row is constructed, it has a generation of 0, and when it is destructed, it gets a generation of 1.
//! This differentiates each construction of that [`EntityRow`].
//! All an [`Entity`] id is is a [`EntityRow`] (which entity it is) and a [`EntityGeneration`] (which version of that row it references).
//! When an [`Entity`] id is invalid, it just means that that generation of its row has been destructed.
//!
//! As mentioned, once an [`EntityRow`] is destructed, it is not discoverable until it is constructed again.
//! To prevent these rows from being forgotten, bevy tracks them in an [`EntitiesAllocator`].
//! When a new entity is spawned, all bevy does is allocate a new [`Entity`] id from the allocator and [`World::construct`](crate::world::World::construct) it.
//! When it is despawned, all bevy does is [`World::destruct`](crate::world::World::destruct) it and return the [`Entity`] id (with the next [`EntityGeneration`] for that [`EntityRow`]) to the allocator.
//! It's that simple.
//!
//! Bevy exposes this functionality as well.
//! Spawning an entity requires full access to the [`World`], but using [`World::spawn_null`](crate::world::World::spawn_null) can be done fully concurrently.
//! Of course, to make that entity usable, it will need to be passed to [`World::construct`](crate::world::World::construct).
//! Managing entity ids manually is advanced but can be very useful for concurrency, custom entity allocators, etc.
//! But there are risks when used improperly:
//! Loosing a destructed entity row without returning it to bevy's allocator will cause that row to be unreachable, effectively a memory leak.
//! Further, constructing an arbitrary [`EntityRow`] can cause problems if that same row is queued for reuse in the allocator.
//! Use this powerfully but with caution.
//!
//! Lots of information about the state of an [`EntityRow`] can be obtained through [`Entities`].
//! For example, this can be used to get the most recent [`Entity`] of an [`EntityRow`] in [`Entities::resolve_from_row`].
//! See its docs for more.
//!
//! In short:
//!
//! - An [`Entity`] id is just a [`EntityRow`] and a [`EntityGeneration`] of that row.
//! - [`EntityRow`]s can be constructed and destructed repeatedly, where each construction gets its own [`EntityGeneration`].
//! - Bevy exposes this functionality through [`World::spawn_null`](crate::world::World::spawn_null), [`World::construct`](crate::world::World::construct), and [`World::destruct`](crate::world::World::destruct).
//! - While understanding these details help build an intuition for how bevy handles entities, using these apis directly is risky but powerful.
//! - Lots of id information can be obtained from [`Entities`].
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
    change_detection::MaybeLocation,
    component::{CheckChangeTicks, Tick},
    storage::{SparseSetIndex, TableId, TableRow},
};
use alloc::vec::Vec;
use bevy_platform::sync::atomic::{AtomicU32, Ordering};
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

/// This uniquely identifies an entity in a [`World`].
/// Note that this is just an id, not the entity itself.
/// Further, the entity this id refers to may no longer exist in the [`World`].
/// For more information about entities, their ids, and how to use them, see the module docs.
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
/// If you must do otherwise, do not assume the [`Entity`] corresponds to the same conceptual entity it originally did.
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

/// Allocates [`Entity`] ids uniquely.
/// This is used in [`World::construct`](crate::world::World::construct) and [`World::despawn`](crate::world::World::despawn) to track entity ids no longer in use.
/// Allocating is fully concurrent and can be done from multiple threads.
///
/// Conceptually, this is a collection of [`Entity`] ids who's [`EntityRow`] is destructed and who's [`EntityGeneration`] is the most recent.
/// See the module docs for how these ids and this allocator participate in the life cycle of an entity.
#[derive(Default, Debug)]
pub struct EntitiesAllocator {
    /// All the entities to reuse.
    /// This is a buffer, which contains an array of [`Entity`] ids to hand out.
    /// The next id to hand out is tracked by `free_len`.
    free: Vec<Entity>,
    /// This is continually subtracted from.
    /// If it wraps to a very large number, it will be outside the bounds of `free`,
    /// and a new row will be needed.
    free_len: AtomicU32,
    /// This is the next "fresh" row to hand out.
    /// If there are no rows to reuse, this row, which has a generation of 0, is the next to return.
    next_row: AtomicU32,
}

impl EntitiesAllocator {
    /// Restarts the allocator.
    pub(crate) fn restart(&mut self) {
        self.free.clear();
        *self.free_len.get_mut() = 0;
        *self.next_row.get_mut() = 0;
    }

    pub(crate) fn free(&mut self, freed: Entity) {
        let expected_len = *self.free_len.get_mut() as usize;
        if expected_len > self.free.len() {
            self.free.clear();
        } else {
            self.free.truncate(expected_len);
        }
        self.free.push(freed);
        *self.free_len.get_mut() = self.free.len() as u32;
    }

    pub(crate) fn alloc(&self) -> Entity {
        let index = self
            .free_len
            .fetch_sub(1, Ordering::Relaxed)
            .wrapping_sub(1);
        self.free.get(index as usize).copied().unwrap_or_else(|| {
            let row = self.next_row.fetch_add(1, Ordering::Relaxed);
            let row = NonMaxU32::new(row).expect("too many entities");
            Entity::from_raw(EntityRow::new(row))
        })
    }

    pub(crate) fn alloc_many(&self, count: u32) -> AllocEntitiesIterator<'_> {
        let current_len = self
            .free_len
            .fetch_sub(count, Ordering::Relaxed)
            .min(self.free.len() as u32);
        let start = current_len.saturating_sub(count);
        let reuse = (start as usize)..(current_len as usize);
        let still_need = count - reuse.len() as u32;
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

/// An [`Iterator`] returning a sequence of [`Entity`] values from [`Entities`].
/// Dropping this will still retain the entities as allocated; this is effectively a leak.
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
                Entity::from_raw(row)
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

/// [`Entities`] tracks all know [`EntityRow`]s and their metadata.
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

    /// Returns the [`EntityLocation`] of an [`Entity`] if it exists and is constructed.
    /// This can error if the [`EntityGeneration`] of this id has passed or if the [`EntityRow`] is not constructed.
    /// See the module docs for a full explanation of these ids, entity life cycles, and the meaning of this result.
    #[inline]
    pub fn get_constructed(
        &self,
        entity: Entity,
    ) -> Result<EntityLocation, ConstructedEntityDoesNotExistError> {
        match self.meta.get(entity.index() as usize) {
            Some(meta) => {
                if meta.generation != entity.generation {
                    Err(ConstructedEntityDoesNotExistError::DidNotExist(
                        EntityDoesNotExistError {
                            entity,
                            current_generation: EntityGeneration::FIRST,
                        },
                    ))
                } else {
                    match meta.location {
                        Some(location) => Ok(location),
                        None => Err(ConstructedEntityDoesNotExistError::WasNotConstructed(
                            EntityNotConstructedError {
                                entity,
                                location: meta.spawned_or_despawned.by.map(Some),
                            },
                        )),
                    }
                }
            }
            None => {
                if entity.generation() == EntityGeneration::FIRST {
                    Err(ConstructedEntityDoesNotExistError::WasNotConstructed(
                        EntityNotConstructedError {
                            entity,
                            location: MaybeLocation::new(None),
                        },
                    ))
                } else {
                    Err(ConstructedEntityDoesNotExistError::DidNotExist(
                        EntityDoesNotExistError {
                            entity,
                            current_generation: EntityGeneration::FIRST,
                        },
                    ))
                }
            }
        }
    }

    /// Returns the [`EntityIdLocation`] of an [`Entity`] if it exists.
    /// This can fail if the id's [`EntityGeneration`] has passed.
    /// See the module docs for a full explanation of these ids, entity life cycles, and the meaning of this result.
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<EntityIdLocation, EntityDoesNotExistError> {
        match self.meta.get(entity.index() as usize) {
            Some(meta) => {
                if meta.generation == entity.generation {
                    Ok(meta.location)
                } else {
                    Err(EntityDoesNotExistError {
                        entity,
                        current_generation: meta.generation,
                    })
                }
            }
            None => {
                if entity.generation() == EntityGeneration::FIRST {
                    Ok(None)
                } else {
                    Err(EntityDoesNotExistError {
                        entity,
                        current_generation: EntityGeneration::FIRST,
                    })
                }
            }
        }
    }

    /// Get the [`Entity`] for the given [`EntityRow`].
    /// Note that this entity may not be constructed yet.
    /// See the module docs for a full explanation of these ids, entity life cycles, and what it means for a row to be constructed or not.
    #[inline]
    pub fn resolve_from_row(&self, row: EntityRow) -> Entity {
        self.meta
            .get(row.index() as usize)
            .map(|meta| Entity::from_raw_and_generation(row, meta.generation))
            .unwrap_or(Entity::from_raw(row))
    }

    /// Returns whether the entity at this `row` is constructed or not.
    /// See the module docs for what it means for a row to be constructed or not.
    #[inline]
    pub fn is_row_constructed(&self, row: EntityRow) -> bool {
        self.meta
            .get(row.index() as usize)
            .map(|meta| meta.location.is_some())
            .unwrap_or_default()
    }

    /// Returns true if the entity exists.
    /// This will return true for entities that exist but have not been constructed.
    /// See the module docs for a more precise explanation of which entities exist and what construction means.
    pub fn contains(&self, entity: Entity) -> bool {
        self.resolve_from_row(entity.row()).generation() == entity.generation()
    }

    /// Returns true if the entity exists and are constructed.
    /// See the module docs for a more precise explanation of which entities exist and what construction means.
    pub fn contains_constructed(&self, entity: Entity) -> bool {
        self.get_constructed(entity).is_ok()
    }

    /// Provides information regarding if `entity` may be safely constructed.
    /// This can error if the entity does not exist or if it is already constructed.
    /// See the module docs for a more precise explanation of which entities exist and what construction means.
    #[inline]
    pub fn validate_construction(&self, entity: Entity) -> Result<(), ConstructionError> {
        match self.get(entity) {
            Ok(Some(_)) => Err(ConstructionError::AlreadyConstructed),
            Ok(None) => Ok(()),
            Err(err) => Err(ConstructionError::InvalidId(err)),
        }
    }

    /// Updates the location of an [`EntityRow`].
    /// This must be called when moving the components of the existing entity around in storage.
    /// Returns the previous location of the row.
    ///
    /// # Safety
    ///  - The current location of the `row` must already be set. If not, try [`declare`](Self::declare).
    ///  - `location` must be valid for the entity at `row` or immediately made valid afterwards
    ///    before handing control to unknown code.
    #[inline]
    pub(crate) unsafe fn update(
        &mut self,
        row: EntityRow,
        location: EntityIdLocation,
    ) -> EntityIdLocation {
        // SAFETY: Caller guarantees that `row` already had a location, so `declare` must have made the index valid already.
        let meta = unsafe { self.meta.get_unchecked_mut(row.index() as usize) };
        mem::replace(&mut meta.location, location)
    }

    /// Declares the location of an [`EntityRow`].
    /// This must be called when constructing/spawning entities.
    /// Returns the previous location of the row.
    ///
    /// # Safety
    ///  - `location` must be valid for the entity at `index` or immediately made valid afterwards
    ///    before handing control to unknown code.
    #[inline]
    pub(crate) unsafe fn declare(
        &mut self,
        row: EntityRow,
        location: EntityIdLocation,
    ) -> EntityIdLocation {
        self.ensure_row_index_is_valid(row);
        // SAFETY: We just did `ensure_row`
        self.update(row, location)
    }

    /// Ensures row is valid as an index in [`Self::meta`].
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
    /// - `row` must be destructed (have no location) already.
    pub(crate) unsafe fn mark_free(&mut self, row: EntityRow, generations: u32) -> Entity {
        // We need to do this in case an entity is being freed that was never constructed.
        self.ensure_row_index_is_valid(row);
        // SAFETY: We just did `ensure_row`
        let meta = unsafe { self.meta.get_unchecked_mut(row.index() as usize) };

        let (new_generation, aliased) = meta.generation.after_versions_and_could_alias(generations);
        meta.generation = new_generation;
        if aliased {
            warn!("EntityRow({row}) generation wrapped on Entities::free, aliasing may occur",);
        }

        Entity::from_raw_and_generation(row, meta.generation)
    }

    /// Mark an [`EntityRow`] as constructed or destructed in the given tick.
    ///
    /// # Safety
    ///  - `row` must have been constructed at least once, ensuring its row is valid.
    #[inline]
    pub(crate) unsafe fn mark_construct_or_destruct(
        &mut self,
        row: EntityRow,
        by: MaybeLocation,
        at: Tick,
    ) {
        // SAFETY: Caller guarantees that `row` already had a location, so `declare` must have made the index valid already.
        let meta = unsafe { self.meta.get_unchecked_mut(row.index() as usize) };
        meta.spawned_or_despawned = SpawnedOrDespawned { by, at };
    }

    /// Try to get the source code location from which this entity has last been constructed or destructed.
    ///
    /// Returns `None` if the entity does not exist or has never been construced/destructed.
    pub fn entity_get_spawned_or_despawned_by(
        &self,
        entity: Entity,
    ) -> MaybeLocation<Option<&'static Location<'static>>> {
        MaybeLocation::new_with_flattened(|| {
            self.entity_get_spawned_or_despawned(entity)
                .map(|spawned_or_despawned| spawned_or_despawned.by)
        })
    }

    /// Try to get the [`Tick`] at which this entity has last been constructed or destructed.
    ///
    /// Returns `None` if the entity does not exist or has never been construced/destructed.
    pub fn entity_get_spawned_or_despawned_at(&self, entity: Entity) -> Option<Tick> {
        self.entity_get_spawned_or_despawned(entity)
            .map(|spawned_or_despawned| spawned_or_despawned.at)
    }

    /// Try to get the [`SpawnedOrDespawned`] related to the entity's last construction or destruction.
    ///
    /// Returns `None` if the entity does not exist or has never been construced/destructed.
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
        (meta.spawned_or_despawned.by, meta.spawned_or_despawned.at)
    }

    #[inline]
    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        for meta in &mut self.meta {
            meta.spawned_or_despawned.at.check_tick(check);
        }
    }

    /// The count of currently allocated entity rows.
    /// For information on active entities, see [`Self::count_constructed`].
    #[inline]
    pub fn len(&self) -> u32 {
        self.meta.len() as u32
    }

    /// Checks if any entity has been declared.
    /// For information on active entities, see [`Self::any_constructed`].
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Counts the number of entity rows currently constructed.
    /// See the module docs for a more precise explanation of what construction means.
    pub fn count_constructed(&self) -> u32 {
        self.meta
            .iter()
            .filter(|meta| meta.location.is_some())
            .count() as u32
    }

    /// Returns true if there are any entity rows currently constructed.
    /// See the module docs for a more precise explanation of what construction means.
    pub fn any_constructed(&self) -> bool {
        self.meta.iter().any(|meta| meta.location.is_some())
    }
}

/// An error that occurs when a specified [`Entity`] can not be constructed.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionError {
    /// The [`Entity`] to construct was invalid.
    /// It probably had the wrong generation or was created erroneously.
    #[error("Invalid id: {0}")]
    InvalidId(EntityDoesNotExistError),
    /// The [`Entity`] to construct was already constructed.
    #[error("The entity can not be constructed as it already has a location.")]
    AlreadyConstructed,
}

/// An error that occurs when a specified [`Entity`] does not exist.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error(
    "The entity with ID {entity} does not exist; its row now has generation {current_generation}."
)]
pub struct EntityDoesNotExistError {
    /// The entity's ID.
    pub entity: Entity,
    /// The generation of the [`EntityRow`], which did not match the requested entity.
    pub current_generation: EntityGeneration,
}

/// An error that occurs when a specified [`Entity`] exists but was not constructed when it was expected to be.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityNotConstructedError {
    /// The entity's ID.
    pub entity: Entity,
    /// The location of what last destructed the entity.
    pub location: MaybeLocation<Option<&'static Location<'static>>>,
}

impl fmt::Display for EntityNotConstructedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entity = self.entity;
        match self.location.into_option() {
            Some(Some(location)) => write!(f, "The entity with ID {entity} is not constructed; its row was last destructed by {location}."),
            Some(None) => write!(
                f,
                "The entity with ID {entity} is not constructed; its row has never been constructed."
            ),
            None => write!(
                f,
                "The entity with ID {entity} is not constructed; enable `track_location` feature for more details."
            ),
        }
    }
}

/// Represents an error of either [`EntityDoesNotExistError`] or [`EntityNotConstructedError`].
#[derive(thiserror::Error, Copy, Clone, Debug, Eq, PartialEq)]
pub enum ConstructedEntityDoesNotExistError {
    /// The entity did not exist.
    #[error("{0}")]
    DidNotExist(#[from] EntityDoesNotExistError),
    /// The entity did exist but was not constructed.
    #[error("{0}")]
    WasNotConstructed(#[from] EntityNotConstructedError),
}

impl ConstructedEntityDoesNotExistError {
    /// The entity that did not exist or was not constructed.
    pub fn entity(&self) -> Entity {
        match self {
            ConstructedEntityDoesNotExistError::DidNotExist(entity_does_not_exist_error) => {
                entity_does_not_exist_error.entity
            }
            ConstructedEntityDoesNotExistError::WasNotConstructed(entity_not_constructed_error) => {
                entity_not_constructed_error.entity
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct EntityMeta {
    /// The current [`EntityGeneration`] of the [`EntityRow`].
    generation: EntityGeneration,
    /// The current location of the [`EntityRow`].
    location: EntityIdLocation,
    /// Location and tick of the last construct/destruct
    spawned_or_despawned: SpawnedOrDespawned,
}

#[derive(Copy, Clone, Debug)]
struct SpawnedOrDespawned {
    by: MaybeLocation,
    at: Tick,
}

impl EntityMeta {
    /// The metadata for a fresh entity: Never constructed/destructed, no location, etc.
    const FRESH: EntityMeta = EntityMeta {
        generation: EntityGeneration::FIRST,
        location: None,
        spawned_or_despawned: SpawnedOrDespawned {
            by: MaybeLocation::caller(),
            at: Tick::new(0),
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

/// An [`Entity`] id may or may not correspond to a valid conceptual entity.
/// If it does, the conceptual entity may or may not have a location.
/// If it has no location, the [`EntityLocation`] will be `None`.
/// An location of `None` means the entity effectively does not exist; it has an id, but is not participating in the ECS.
/// This is different from a location in the empty archetype, which is participating (queryable, etc) but just happens to have no components.
///
/// Setting a location to `None` is often helpful when you want to destruct an entity or yank it from the ECS without allowing another system to reuse the id for something else.
/// It is also useful for reserving an id; commands will often allocate an `Entity` but not provide it a location until the command is applied.
pub type EntityIdLocation = Option<EntityLocation>;

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
        let entity = Entity::from_raw(EntityRow::new(NonMaxU32::new(42).unwrap()));
        let string = format!("{entity:?}");
        assert_eq!(string, "42v0#4294967253");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{entity:?}");
        assert_eq!(string, "PLACEHOLDER");
    }

    #[test]
    fn entity_display() {
        let entity = Entity::from_raw(EntityRow::new(NonMaxU32::new(42).unwrap()));
        let string = format!("{entity}");
        assert_eq!(string, "42v0");

        let entity = Entity::PLACEHOLDER;
        let string = format!("{entity}");
        assert_eq!(string, "PLACEHOLDER");
    }
}
