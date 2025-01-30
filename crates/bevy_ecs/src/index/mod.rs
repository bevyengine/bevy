//! Provides indexing support for the ECS.
//!
//! # Background
//!
//! The most common way of querying for data within the [`World`] is with [`Query`] as a [system parameter](`SystemParam`).
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
    self as bevy_ecs, component::{Component, ComponentDescriptor, ComponentId, Immutable, StorageType}, entity::Entity, prelude::Trigger, query::{QueryBuilder, QueryData, QueryFilter}, system::{Commands, Query, ResMut, SystemParam}, world::{OnInsert, OnReplace, World}
};
use alloc::{format, vec::Vec};
use bevy_ecs_macros::Resource;
use bevy_platform_support::collections::HashMap;
use bevy_ptr::OwningPtr;
use core::{alloc::Layout, hash::Hash, ptr::NonNull};

/// Extension methods for [`World`] to assist with indexing components.
pub trait WorldIndexExtension {
    /// Create and track an index for `C`.
    /// This is required to use the [`QueryByIndex`] system parameter.
    fn add_index<C: IndexableComponent>(&mut self) -> &mut Self;
}

impl WorldIndexExtension for World {
    fn add_index<C: IndexableComponent>(&mut self) -> &mut Self {
        if self.get_resource::<Index<C>>().is_none() {
            let mut index = Index::<C>::default();

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

/// A marker trait describing the requirements for a [`Component`] to be indexed and accessed via [`QueryByIndex`].
///
/// See the module docs for more information.
pub trait IndexableComponent: Component<Mutability = Immutable> + Eq + Hash + Clone {}

impl<C: Component<Mutability = Immutable> + Eq + Hash + Clone> IndexableComponent for C {}

/// This [`Resource`] is responsible for managing a value-to-[`ComponentId`] mapping, allowing
/// [`QueryByIndex`] to simply filter by [`ComponentId`] on a standard [`Query`].
#[derive(Resource)]
struct Index<C: IndexableComponent> {
    /// Maps `C` values to a marking ZST component.
    mapping: HashMap<C, IndexState>,
    /// Previously registered but currently unused marking ZSTs.
    /// If a value _was_ tracked in [`mapping`](Index::mapping) but no entity
    /// has that value anymore, its marker is pushed here for reuse when a _new_
    /// value for `C` needs to be tracked.
    ///
    /// When exhausted, new markers must be registered from a [`World`].
    spare_markers: Vec<ComponentId>,
}

/// Internal state for a particular index value within the [`Index`] resource.
#[derive(Clone, Copy)]
struct IndexState {
    /// [`ComponentId`] of this marking ZST
    component_id: ComponentId,
    /// A count of how many entities are currently holding this component
    live_count: usize,
}

// Rust's derives assume that C must impl Default for this to hold,
// but that's not true.
impl<C: IndexableComponent> Default for Index<C> {
    fn default() -> Self {
        Self {
            mapping: Default::default(),
            spare_markers: Default::default(),
        }
    }
}

impl<C: IndexableComponent> Index<C> {
    fn track_entity(&mut self, world: &mut World, entity: Entity) {
        let Some(value) = world.get::<C>(entity) else {
            return;
        };

        // Need a marker component for this entity
        let component_id = match self.mapping.get_mut(value) {
            // This particular value already has an assigned marker, reuse it.
            Some(state) => {
                state.live_count += 1;
                state.component_id
            }
            None => {
                // Need to clone the index value for later lookups
                let value = value.clone();

                // Attempt to recycle an old marker.
                // Otherwise, allocate a new one.
                let component_id = self
                    .spare_markers
                    .pop()
                    .unwrap_or_else(|| Self::alloc_new_marker(world));

                self.mapping.insert(
                    value,
                    IndexState {
                        component_id,
                        live_count: 1,
                    },
                );
                component_id
            }
        };

        // SAFETY:
        // - component_id is from the same world
        // - NonNull::dangling() is appropriate for a ZST component
        unsafe {
            world
                .entity_mut(entity)
                .insert_by_id(component_id, OwningPtr::new(NonNull::dangling()));
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

        let state = index.mapping.get_mut(value).unwrap();

        state.live_count = state.live_count.saturating_sub(1);

        let component_id = state.component_id;

        // On removal, we check if this was the last usage of this marker.
        // If so, we can recycle it for a different value
        if state.live_count == 0 {
            index.mapping.remove(value);
            index.spare_markers.push(component_id);
        }

        commands.queue(move |world: &mut World| {
            // The old marker is no longer applicable since the value has changed/been removed.
            world.entity_mut(entity).remove_by_id(component_id);
        });
    }

    /// Creates a new marker component for this index.
    /// It represents a ZST and is not tied to a particular value.
    /// This allows moving entities into new archetypes based on the indexed value.
    fn alloc_new_marker(world: &mut World) -> ComponentId {
        // SAFETY:
        // - ZST is Send + Sync
        // - No drop function provided or required
        let descriptor = unsafe {
            ComponentDescriptor::new_with_layout(
                format!("Index Marker ({})", core::any::type_name::<Self>()),
                StorageType::Table,
                Layout::new::<()>(),
                None,
                false,
            )
        };

        world.register_component_with_descriptor(descriptor)
    }

    fn filter_query_for<D: QueryData, F: QueryFilter>(&self, builder: &mut QueryBuilder<D, F>, value: &C) {
        let Some(state) = self.mapping.get(value) else {
            // If there is no marker, create a no-op query by including With<C> and Without<C>
            builder.without::<C>();
            return;
        };

        // If there is a marker, restrict to it
        builder.with_id(state.component_id);
    }
}

/// Converts a [`usize`] into a [`bool`] array.
const fn usize_to_bool_array(value: usize) -> [bool; size_of::<usize>() * 8] {
    const LENGTH: usize = size_of::<usize>() * 8;
    let mut array = [false; LENGTH];

    let mut index = 0;
    while index < LENGTH {
        array[index] = value & (1 << index) > 0;
        index += 1;
    }

    array
}