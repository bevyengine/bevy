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
//! our `Planet` as an [indexable component](`IndexableComponent`).
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
//! world.add_index::<Planet>();
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
//!     let earthlings = query.at(&Planet("Earth"));
//!
//!     for earthling in &earthlings {
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
//! This makes iterating through a subset of the total archetype faster, but decreases the performance
//! of iterating the whole archetype by a small amount.
//!
//! ## Component ID Exhaustion
//!
//! Fragmenting requires unique [component IDs](ComponentId), and they are finite.
//! For components with a small number of values in use at any one time, this is acceptable.
//! But for components with a _large_ number of unique values (e.g., components containing floating point numbers),
//! the pool of available component IDs will be quickly exhausted.
//!
//! ## Mutation Overhead
//!
//! The index is maintained continuously to ensure it is always valid.
//! This is great for usability, but means all mutations of indexed components will carry a small but
//! existent overhead.

mod query_by_index;

pub use query_by_index::*;

use crate::{
    self as bevy_ecs,
    component::{Component, ComponentDescriptor, ComponentId, Immutable, StorageType},
    entity::Entity,
    prelude::Trigger,
    system::{Commands, Query, ResMut},
    world::{FromWorld, OnInsert, OnReplace, World},
};
use alloc::{format, vec::Vec};
use bevy_ecs_macros::Resource;
use bevy_platform_support::{collections::HashMap, hash::FixedHasher};
use bevy_ptr::OwningPtr;
use core::{alloc::Layout, hash::Hash, ptr::NonNull};

/// A marker trait describing the requirements for a [`Component`] to be indexed and accessed via [`QueryByIndex`].
///
/// See the module docs for more information.
pub trait IndexableComponent: Component<Mutability = Immutable> + Eq + Hash + Clone {}

impl<C: Component<Mutability = Immutable> + Eq + Hash + Clone> IndexableComponent for C {}

/// This [`Resource`] is responsible for managing a value-to-[`ComponentId`] mapping, allowing
/// [`QueryByIndex`] to simply filter by [`ComponentId`] on a standard [`Query`].
#[derive(Resource)]
struct Index<C: IndexableComponent> {
    /// Maps `C` values to an index within [`slots`](Index::slots)
    mapping: HashMap<C, usize>,
    /// A collection of ZST dynamic [`Component`]s which (in combination) uniquely address a _value_
    /// of `C` within the [`World`].
    markers: Vec<ComponentId>,
    /// A list of liveness counts.
    /// Once a value hits zero, it is free for reuse.
    /// If no values are zero, you must append to the end of the list.
    slots: Vec<IndexState>,
}

struct IndexState {
    active: usize,
}

impl<C: IndexableComponent> FromWorld for Index<C> {
    fn from_world(world: &mut World) -> Self {
        let bits = 8 * size_of::<C>().min(size_of::<u32>());

        let markers = (0..bits)
            .map(|bit| Self::alloc_new_marker(world, bit, StorageType::Table))
            .take(bits)
            .collect::<Vec<_>>();

        Self {
            mapping: HashMap::with_hasher(FixedHasher),
            markers,
            slots: Vec::new(),
        }
    }
}

impl<C: IndexableComponent> Index<C> {
    fn track_entity(&mut self, world: &mut World, entity: Entity) {
        let Some(value) = world.get::<C>(entity) else {
            return;
        };

        let slot_index = match self.mapping.get(value) {
            Some(&index) => {
                self.slots[index].active += 1;
                index
            }
            None => {
                let spare_slot = (self.slots.len() > self.mapping.len())
                    .then(|| {
                        self.slots
                            .iter_mut()
                            .enumerate()
                            .find(|(_, slot)| slot.active == 0)
                    })
                    .flatten();

                match spare_slot {
                    Some((index, slot)) => {
                        slot.active += 1;
                        index
                    }
                    None => {
                        let index = self.slots.len();
                        self.mapping.insert(value.clone(), index);
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
    }

    fn on_insert(trigger: Trigger<OnInsert, C>, mut commands: Commands) {
        let entity = trigger.target();

        commands.queue(move |world: &mut World| {
            world.resource_scope::<Self, _>(|world, mut index| {
                index.track_entity(world, entity);
            });
        });
    }

    fn on_replace(
        trigger: Trigger<OnReplace, C>,
        query: Query<&C>,
        mut index: ResMut<Index<C>>,
        mut commands: Commands,
    ) {
        let entity = trigger.target();

        let value = query.get(entity).unwrap();

        let &slot_index = index.mapping.get(value).unwrap();

        let slot = &mut index.slots[slot_index];

        slot.active = slot.active.saturating_sub(1);

        // On removal, we check if this was the last usage of this marker.
        // If so, we can recycle it for a different value
        if slot.active == 0 {
            index.mapping.remove(value);
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
    fn alloc_new_marker(world: &mut World, bit: usize, storage_type: StorageType) -> ComponentId {
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

    fn ids_for(&self, index: usize) -> Vec<ComponentId> {
        self.markers
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| (index & (1 << i) > 0).then_some(id))
            .collect::<Vec<_>>()
    }
}

/// Extension methods for [`World`] to assist with indexing components.
pub trait WorldIndexExtension {
    /// Create and track an index for `C`.
    /// This is required to use the [`QueryByIndex`] system parameter.
    fn add_index<C: IndexableComponent>(&mut self) -> &mut Self;
}

impl WorldIndexExtension for World {
    fn add_index<C: IndexableComponent>(&mut self) -> &mut Self {
        if self.get_resource::<Index<C>>().is_none() {
            let mut index = Index::<C>::from_world(self);

            self.query::<(Entity, &C)>()
                .iter(self)
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|entity| {
                    index.track_entity(self, entity);
                });

            self.insert_resource(index);
            self.add_observer(Index::<C>::on_insert);
            self.add_observer(Index::<C>::on_replace);
        }

        self
    }
}
