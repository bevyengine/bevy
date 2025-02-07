//! Provides indexing support for the ECS.
//!
//! # Background
//!
//! The most common way of querying for data within the [`World`] is with [`Query`] as a system parameter.
//! This requires specifying all the parameters of your query up-front in the type-signature of system.
//! This is problematic when you don't want to query for _all_ entities with a particular set of components,
//! and instead want entities who have particular _values_ for a given component.
//!
//! Consider a `Planet` component that marks which planet an entity is on.
//! We _could_ create a unique marking component for each planet:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! #[derive(Component)]
//! struct Earth;
//!
//! #[derive(Component)]
//! struct Mars;
//!
//! // ...
//! ```
//!
//! But what if the list of planets isn't knowable at compile-time and is instead controlled at runtime?
//! This would require something like:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! #[derive(Component, PartialEq, Eq)]
//! struct Planet(&'static str);
//! ```
//!
//! This lets us create planets at runtime (maybe the player is the one creating them!).
//! But how do we query for this runtime-compatible `Planet`?
//! The naive approach would be to query for the `Planet` component and `filter` for a particular value.
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # #[derive(Component, PartialEq, Eq)]
//! # struct Planet(&'static str);
//! fn get_earthlings(mut query: Query<(Entity, &Planet)>) {
//!     let earthlings = query.iter().filter(|(_, planet)| **planet == Planet("Earth"));
//!
//!     for earthling in earthlings {
//!         // ...
//!     }
//! }
//! ```
//!
//! The problem here is that our `get_earthlings` system reserves access to and iterates through _every_
//! entity on _every_ planet!
//! If you have a lot of planets and a lot of entities, that's a massive bottleneck.
//!
//! _There must be a better way!_
//!
//! # Query By Index
//!
//! Instead of filtering by value in the body of a system, we can instead use [`QueryByIndex`] and treat
//! our `Planet` as an indexable component.
//!
//! First, we need to modify `Planet` to include implementations for `Clone` and `Hash`, and to mark it as
//! an immutable component:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! #[derive(Component, PartialEq, Eq, Hash, Clone)]
//! #[component(immutable)]
//! struct Planet(&'static str);
//! ```
//!
//! Next, we need to inform the world that we want `Planet` to be indexed:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # #[derive(Component, PartialEq, Eq, Hash, Clone)]
//! # #[component(immutable)]
//! # struct Planet(&'static str);
//! # let mut world = World::new();
//! world.add_index(IndexOptions::<Planet>::default());
//! ```
//!
//! This sets up the necessary mechanisms behind the scenes to track `Planet` components and make
//! querying by value as performant as possible.
//!
//! Now we can use [`QueryByIndex`] instead of [`Query`] in our `get_earthlings` system:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # #[derive(Component, PartialEq, Eq, Hash, Clone)]
//! # #[component(immutable)]
//! # struct Planet(&'static str);
//! fn get_earthlings(mut query: QueryByIndex<Planet, Entity>) {
//!     let mut earthlings = query.at(&Planet("Earth"));
//!
//!     for earthling in &earthlings.query() {
//!         // ...
//!     }
//! }
//! ```
//!
//! While this may look similar, the way this information is loaded from the ECS is completely different.
//! Instead of loading archetypes, then the entities, and comparing to our value, we first check an
//! index for our value and only load the archetypes with that value.
//! This gives us the same iteration performance as if we had created all those planet
//! marker components at compile time.
//!
//! # Drawbacks
//!
//! Indexing by a component value isn't free unfortunately. If it was, it would be enabled by default!
//!
//! ## Fragmentation
//!
//! To provide the maximum iteration speed, the indexable component is fragmented, meaning each unique
//! value is stored in its own archetype.
//! Archetypes are reused when values are no longer in use;
//! and so the cost paid scales with the maximum number of unique values alive _simultaneously_.
//! This makes iterating through a subset of the total archetypes faster, but decreases the performance
//! of iterating all archetypes by a small amount.
//!
//! This also has the potential to multiply the number of unused [`Archetypes`](crate::archetype::Archetype).
//! Since Bevy does not currently have a mechanism for cleaning up unused [`Archetypes`](crate::archetype::Archetype),
//! this can present itself like a memory leak.
//! If you find your application consuming substantially more memory when using indexing, please
//! [open an issue on GitHub](https://github.com/bevyengine/bevy/issues/new/choose) to help us
//! improve memory performance in real-world applications.
//!
//! ## Mutation Overhead
//!
//! The index is maintained continuously to ensure it is always valid.
//! This is great for usability, but means all mutations of indexed components will carry a small but
//! existent overhead.

mod query_by_index;
mod storage;

pub use query_by_index::*;
pub use storage::*;

use crate::{
    self as bevy_ecs,
    component::{Component, ComponentDescriptor, ComponentId, Immutable, StorageType},
    entity::Entity,
    prelude::Trigger,
    system::{Commands, Query, ResMut},
    world::{OnInsert, OnReplace, World},
};
use alloc::{boxed::Box, format, vec::Vec};
use bevy_ecs_macros::Resource;
use bevy_platform_support::{collections::HashMap, hash::FixedHasher, sync::Arc};
use bevy_ptr::OwningPtr;
use core::{alloc::Layout, hash::Hash, marker::PhantomData, ptr::NonNull};
use thiserror::Error;

/// This [`Resource`] is responsible for managing a value-to-[`ComponentId`] mapping, allowing
/// [`QueryByIndex`] to simply filter by [`ComponentId`] on a standard [`Query`].
#[derive(Resource)]
struct Index<C: Component<Mutability = Immutable>> {
    /// Maps `C` values to an index within [`slots`](Index::slots).
    ///
    /// We use a `Box<dyn IndexStorage<C>>` instead of a concrete type parameter to ensure two indexes
    /// for the same component `C` never exist.
    mapping: Box<dyn IndexStorage<C>>,
    /// A collection of ZST dynamic [`Component`]s which (in combination) uniquely address a _value_
    /// of `C` within the [`World`].
    ///
    /// We use an [`Arc`] to allow for cheap cloning by [`QueryByIndex`].
    markers: Arc<[ComponentId]>,
    /// A list of liveness counts.
    /// Once a value hits zero, it is free for reuse.
    /// If no values are zero, you must append to the end of the list.
    slots: Vec<IndexState>,
    /// Slots with an active count of zero should be put here for later reuse.
    spare_slots: Vec<usize>,
}

/// Internal state for a [slot](Index::slots) within an [`Index`].
struct IndexState {
    /// A count of how many living [entities](Entity) exist with the world.
    ///
    /// Once this value reaches zero, this slot can be re-allocated for a different
    /// value.
    active: usize,
}

/// Errors returned by [`track_entity`](Index::track_entity).
#[derive(Error, Debug)]
enum TrackEntityError {
    /// The total address space allocated for this index has been exhausted.
    #[error("address space exhausted")]
    AddressSpaceExhausted,
    /// An entity set to be tracked did not contain a suitable value.
    #[error("entity was expected to have the indexable component but it was not found")]
    EntityMissingValue,
}

impl<C: Component<Mutability = Immutable>> Index<C> {
    fn new<S: IndexStorage<C>>(world: &mut World, options: IndexOptions<C, S>) -> Self {
        let bits = options
            .address_space
            .min(size_of::<u32>().saturating_mul(8) as u8) as u16;

        let markers = (0..bits)
            .map(|bit| Self::alloc_new_marker(world, bit, options.marker_storage))
            .collect();

        Self {
            mapping: Box::new(options.index_storage),
            markers,
            slots: Vec::new(),
            spare_slots: Vec::new(),
        }
    }

    fn track_entity(&mut self, world: &mut World, entity: Entity) -> Result<(), TrackEntityError> {
        let Some(value) = world.get::<C>(entity) else {
            return Err(TrackEntityError::EntityMissingValue);
        };

        let slot_index = match self.mapping.get(value) {
            Some(index) => {
                self.slots[index].active += 1;
                index
            }
            None => {
                let spare_slot = self.spare_slots.pop();

                match spare_slot {
                    Some(index) => {
                        self.slots[index].active += 1;
                        self.mapping.insert(value, index);
                        index
                    }
                    None => {
                        if self.slots.len() >= 1 << self.markers.len() {
                            return Err(TrackEntityError::AddressSpaceExhausted);
                        }

                        let index = self.slots.len();
                        self.mapping.insert(value, index);
                        self.slots.push(IndexState { active: 1 });

                        index
                    }
                }
            }
        };

        let ids = self.ids_for(slot_index);

        let zsts = core::iter::repeat_with(|| {
            // SAFETY:
            // - NonNull::dangling() is appropriate for a ZST
            unsafe { OwningPtr::new(NonNull::dangling()) }
        })
        .take(ids.len());

        // SAFETY:
        // - ids are from the same world
        // - OwningPtr is valid for the entire lifetime of the application
        unsafe {
            world.entity_mut(entity).insert_by_ids(&ids, zsts);
        }

        Ok(())
    }

    /// Observer for [`OnInsert`] events for the indexed [`Component`] `C`.
    fn on_insert(trigger: Trigger<OnInsert, C>, mut commands: Commands) {
        let entity = trigger.target();

        commands.queue(move |world: &mut World| {
            world.resource_scope::<Self, _>(|world, mut index| {
                if let Err(error) = index.track_entity(world, entity) {
                    match error {
                        TrackEntityError::AddressSpaceExhausted => {
                            log::error!(
                                "Entity {:?} could not be indexed by component {} as the total addressable space ({} bits) has been exhausted. Consider increasing the address space using `IndexOptions::address_space`.",
                                entity,
                                disqualified::ShortName::of::<C>(),
                                index.markers.len(),
                            );
                        },
                        TrackEntityError::EntityMissingValue => {
                            // Swallow error.
                            // This was likely caused by the component `C` being removed
                            // before deferred commands were applied in response to the insertion.
                        },
                    }
                }
            });
        });
    }

    /// Observer for [`OnReplace`] events for the indexed [`Component`] `C`.
    fn on_replace(
        trigger: Trigger<OnReplace, C>,
        query: Query<&C>,
        mut index: ResMut<Self>,
        mut commands: Commands,
    ) {
        let entity = trigger.target();

        let value = query.get(entity).unwrap();

        let slot_index = index.mapping.get(value).unwrap();

        let slot = &mut index.slots[slot_index];

        slot.active = slot.active.saturating_sub(1);

        // On removal, we check if this was the last usage of this marker.
        // If so, we can recycle it for a different value
        if slot.active == 0 {
            index.mapping.remove(value);
            index.spare_slots.push(slot_index);
        }

        let ids = index.ids_for(slot_index);

        commands.queue(move |world: &mut World| {
            let ids = ids;
            // The old marker is no longer applicable since the value has changed/been removed.
            world.entity_mut(entity).remove_by_ids(&ids);
        });
    }

    /// Creates a new marker component for this index.
    /// It represents a ZST and is not tied to a particular value.
    /// This allows moving entities into new archetypes based on the indexed value.
    fn alloc_new_marker(world: &mut World, bit: u16, storage_type: StorageType) -> ComponentId {
        // SAFETY:
        // - ZST is Send + Sync
        // - No drop function provided or required
        let descriptor = unsafe {
            ComponentDescriptor::new_with_layout(
                format!("{} Marker #{}", disqualified::ShortName::of::<Self>(), bit),
                storage_type,
                Layout::new::<()>(),
                None,
                false,
            )
        };

        world.register_component_with_descriptor(descriptor)
    }

    /// Gets the [`ComponentId`]s of all markers that _must_ be included on an [`Entity`] allocated
    /// to a particular slot index.
    fn ids_for(&self, index: usize) -> Vec<ComponentId> {
        self.markers
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| (index & (1 << i) > 0).then_some(id))
            .collect::<Vec<_>>()
    }
}

/// Options when configuring an index for a given indexable component `C`.
pub struct IndexOptions<
    C: Component<Mutability = Immutable>,
    S: IndexStorage<C> = HashMap<C, usize>,
> {
    /// Marker components will be added to indexed entities to allow for efficient lookups.
    /// This controls the [`StorageType`] that will be used with these markers.
    ///
    /// - [`Table`](StorageType::Table) is faster for querying
    /// - [`SpareSet`](StorageType::SparseSet) is more memory efficient
    ///
    /// Ensure you benchmark both options appropriately if you are experiencing performance issues.
    ///
    /// This defaults to [`SparseSet`](StorageType::SparseSet).
    pub marker_storage: StorageType,
    /// Marker components are combined into a unique address for each distinct value of the indexed
    /// component.
    /// This controls how many markers will be used to create that unique address.
    /// Note that a value greater than 32 will be reduced down to 32.
    /// Bevy's [`World`] only supports 2^32 entities alive at any one moment in time, so an address space
    /// of 32 is sufficient to uniquely refer to every individual [`Entity`] in the entire [`World`].
    ///
    /// Selecting a value lower than the default value may lead to a panic at runtime or entities
    /// missing from the index if the address space is exhausted.
    ///
    /// This defaults to [`size_of<C>`]
    pub address_space: u8,
    /// A storage backend for this index.
    /// For certain indexing strategies and [`Component`] types, you may be able to greatly
    /// optimize the utility and performance of an index by creating a custom backend.
    ///
    /// See [`IndexStorage`] for details around the implementation of a custom backend.
    ///
    /// This defaults to [`HashMap<C, usize>`].
    pub index_storage: S,
    #[doc(hidden)]
    pub _phantom: PhantomData<fn(&C)>,
}

impl<C: Component<Mutability = Immutable> + Eq + Hash + Clone> Default
    for IndexOptions<C, HashMap<C, usize>>
{
    fn default() -> Self {
        Self {
            marker_storage: StorageType::SparseSet,
            address_space: size_of::<C>() as u8,
            index_storage: HashMap::with_hasher(FixedHasher),
            _phantom: PhantomData,
        }
    }
}

impl<C: Component<Mutability = Immutable>, S: IndexStorage<C>> IndexOptions<C, S> {
    /// Performs initial setup for an index.
    // Note that this is placed here instead of inlined into `World` to allow most
    // most of the indexing internals to stay private.
    #[inline]
    pub(crate) fn setup_index(self, world: &mut World) {
        if world.get_resource::<Index<C>>().is_none() {
            let mut index = Index::<C>::new(world, self);

            world.query::<(Entity, &C)>()
                .iter(world)
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|entity| {
                    if let Err(error) = index.track_entity(world, entity) {
                        match error {
                            TrackEntityError::AddressSpaceExhausted => {
                                log::error!(
                                    "Entity {:?} could not be indexed by component {} as the total addressable space ({} bits) has been exhausted. Consider increasing the address space using `IndexOptions::address_space`.",
                                    entity,
                                    disqualified::ShortName::of::<C>(),
                                    index.markers.len(),
                                );
                            },
                            TrackEntityError::EntityMissingValue => {
                                unreachable!();
                            },
                        }
                    }
                });

            world.insert_resource(index);
            world.add_observer(Index::<C>::on_insert);
            world.add_observer(Index::<C>::on_replace);
        }
    }
}
