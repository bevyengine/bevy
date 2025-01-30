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

use crate::{
    self as bevy_ecs,
    component::{Component, ComponentDescriptor, ComponentId, Immutable, StorageType, Tick},
    entity::Entity,
    prelude::Trigger,
    query::{QueryBuilder, QueryData, QueryFilter, QueryState, With},
    system::{Commands, Query, Res, ResMut, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, OnInsert, OnReplace, World},
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
                .iter(&self)
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

/// This system parameter allows querying by an [indexable component](`IndexableComponent`) value.
///
/// # Examples
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::new();
/// #[derive(Component, PartialEq, Eq, Hash, Clone)]
/// #[component(immutable)]
/// struct Player(u8);
///
/// // Indexing is opt-in through `World::add_index`
/// world.add_index::<Player>();
/// # world.spawn(Player(0));
/// # world.spawn(Player(0));
/// # world.spawn(Player(1));
/// # world.spawn(Player(1));
/// # world.spawn(Player(1));
/// # world.spawn(Player(2));
/// # world.spawn(Player(2));
/// # world.spawn(Player(2));
/// # world.spawn(Player(2));
/// # world.flush();
///
/// fn find_all_player_one_entities(mut query: QueryByIndex<Player, Entity>) {
///     for entity in query.at(&Player(0)).iter() {
///         println!("{entity:?} belongs to Player 1!");
///     }
/// #   assert_eq!(query.at(&Player(0)).iter().count(), 2);
/// #   assert_eq!(query.at(&Player(1)).iter().count(), 3);
/// #   assert_eq!(query.at(&Player(2)).iter().count(), 4);
/// }
/// # world.run_system_cached(find_all_player_one_entities);
/// ```
pub struct QueryByIndex<'world, C: IndexableComponent, D: QueryData, F: QueryFilter = ()> {
    world: UnsafeWorldCell<'world>,
    state: Option<QueryState<D, (F, With<C>)>>,
    last_run: Tick,
    this_run: Tick,
    index: Res<'world, Index<C>>,
}

impl<C: IndexableComponent, D: QueryData, F: QueryFilter> QueryByIndex<'_, C, D, F> {
    /// Return a [`Query`] only returning entities with a component `C` of the provided value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// #[derive(Component, PartialEq, Eq, Hash, Clone)]
    /// #[component(immutable)]
    /// enum FavoriteColor {
    ///     Red,
    ///     Green,
    ///     Blue,
    /// }
    ///
    /// world.add_index::<FavoriteColor>();
    ///
    /// fn find_red_fans(mut query: QueryByIndex<FavoriteColor, Entity>) {
    ///     for entity in query.at(&FavoriteColor::Red).iter() {
    ///         println!("{entity:?} likes the color Red!");
    ///     }
    /// }
    /// ```
    pub fn at(&mut self, value: &C) -> Query<'_, '_, D, (F, With<C>)> {
        self.state = {
            // SAFETY: Mutable references do not alias and will be dropped after this block
            let mut builder = unsafe { QueryBuilder::new(self.world.world_mut()) };

            match self.index.mapping.get(value) {
                // If there is a marker, restrict to it
                Some(state) => builder.with_id(state.component_id),
                // Otherwise, create a no-op query by including With<C> and Without<C>
                None => builder.without::<C>(),
            };

            Some(builder.build())
        };

        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        unsafe {
            Query::new(
                self.world,
                self.state.as_mut().unwrap(),
                self.last_run,
                self.this_run,
            )
        }
    }
}

// SAFETY: We rely on the known-safe implementations of `SystemParam` for `Res` and `Query`.
unsafe impl<C: IndexableComponent, D: QueryData + 'static, F: QueryFilter + 'static> SystemParam
    for QueryByIndex<'_, C, D, F>
{
    type State = (QueryState<D, (F, With<C>)>, ComponentId);
    type Item<'w, 's> = QueryByIndex<'w, C, D, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let query_state = <Query<D, (F, With<C>)> as SystemParam>::init_state(world, system_meta);
        let res_state = <Res<Index<C>> as SystemParam>::init_state(world, system_meta);

        (query_state, res_state)
    }

    #[inline]
    unsafe fn validate_param(
        (query_state, res_state): &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        let query_valid = <Query<D, (F, With<C>)> as SystemParam>::validate_param(
            query_state,
            system_meta,
            world,
        );
        let res_valid =
            <Res<Index<C>> as SystemParam>::validate_param(res_state, system_meta, world);

        query_valid && res_valid
    }

    unsafe fn get_param<'world, 'state>(
        (query_state, res_state): &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        query_state.validate_world(world.id());

        let index =
            <Res<Index<C>> as SystemParam>::get_param(res_state, system_meta, world, change_tick);

        QueryByIndex {
            world,
            state: None,
            last_run: system_meta.last_run,
            this_run: change_tick,
            index,
        }
    }
}

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
}
