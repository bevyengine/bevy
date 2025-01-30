//! Provides indexing support for the ECS.

use crate::{
    self as bevy_ecs,
    component::{Component, ComponentDescriptor, ComponentId, Immutable, StorageType, Tick},
    prelude::Trigger,
    query::{QueryBuilder, QueryData, QueryFilter, QueryState, With},
    system::{Commands, Query, Res, SystemMeta, SystemParam},
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
            self.add_observer(Index::<C>::on_insert);
            self.add_observer(Index::<C>::on_replace);
        }

        self.init_resource::<Index<C>>();

        self
    }
}

/// Marker describing the requirements for a [`Component`] to be suitable for indexing.
pub trait IndexableComponent: Component<Mutability = Immutable> + Eq + Hash + Clone {}

impl<C: Component<Mutability = Immutable> + Eq + Hash + Clone> IndexableComponent for C {}

/// This system parameter allows querying by an indexed component value.
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

            match self.index.mapping.get(value).copied() {
                // If there is a marker, restrict to it
                Some(component_id) => builder.with_id(component_id),
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

#[derive(Resource)]
struct Index<C: IndexableComponent> {
    mapping: HashMap<C, ComponentId>,
    spare_markers: Vec<ComponentId>,
}

impl<C: IndexableComponent> Default for Index<C> {
    fn default() -> Self {
        Self {
            mapping: Default::default(),
            spare_markers: Default::default(),
        }
    }
}

impl<C: IndexableComponent> Index<C> {
    fn on_insert(trigger: Trigger<OnInsert, C>, mut commands: Commands) {
        let entity = trigger.target();

        commands.queue(move |world: &mut World| {
            world.resource_scope::<Self, _>(|world, mut index| {
                let Some(value) = world.get::<C>(entity) else {
                    return;
                };

                // Need a marker component for this entity
                let component_id = match index.mapping.get(value).copied() {
                    // This particular value already has an assigned marker, reuse it.
                    Some(component_id) => component_id,
                    None => {
                        // Need to clone the index value for later lookups
                        let value = value.clone();

                        // Attempt to recycle an old marker.
                        // Otherwise, allocate a new one.
                        let component_id = index
                            .spare_markers
                            .pop()
                            .unwrap_or_else(|| Self::alloc_new_marker(world));

                        index.mapping.insert(value, component_id);
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
            });
        });
    }

    fn on_replace(
        trigger: Trigger<OnReplace, C>,
        query: Query<&C>,
        index: Res<Index<C>>,
        mut commands: Commands,
    ) {
        let entity = trigger.target();

        let value = query
            .get(entity)
            .expect("observer should see a component in on_replace");

        let component_id = index
            .mapping
            .get(value)
            .copied()
            .expect("somehow didn't track this value");

        commands.queue(move |world: &mut World| {
            world.resource_scope::<Self, _>(|world, mut index| {
                // The old marker is no longer applicable since the value has changed/been removed.
                world.entity_mut(entity).remove_by_id(component_id);

                // On removal, we check if this was the last usage of this marker.
                // If so, we can recycle it for a different value
                let Self {
                    ref mut mapping,
                    ref mut spare_markers,
                    ..
                } = index.as_mut();

                // TODO: It may be more performant to make a clone of the old value and lookup directly
                // rather than iterating the whole map.
                mapping.retain(|_key, &mut component_id_other| {
                    if component_id_other != component_id {
                        return true;
                    }

                    let mut builder = QueryBuilder::<(), With<C>>::new(world);
                    builder.with_id(component_id);
                    let mut query = builder.build();

                    // is_empty
                    if query.iter(world).next().is_none() {
                        spare_markers.push(component_id);
                        false
                    } else {
                        true
                    }
                });
            });
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
