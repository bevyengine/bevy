//! Provides indexing support for the ECS.

use crate::{
    self as bevy_ecs,
    component::{Component, ComponentDescriptor, ComponentId, Immutable, StorageType},
    prelude::Trigger,
    query::{QueryBuilder, QueryData, QueryFilter, QueryState, With},
    system::{Commands, Query, Res},
    world::{OnInsert, OnReplace, World},
};
use alloc::{format, vec::Vec};
use bevy_ecs_macros::Resource;
use bevy_platform_support::collections::HashMap;
use bevy_ptr::OwningPtr;
use core::{alloc::Layout, hash::Hash, ptr::NonNull};

/// Extension methods for [`World`] to assist with indexing components.
pub trait WorldIndexExtension {
    /// Create and track an index for `C`.
    fn index_component<C: IndexComponent>(&mut self) -> &mut Self;

    /// Query by an indexed component's value.
    fn query_by_index<C: IndexComponent, D: QueryData, F: QueryFilter>(
        &mut self,
        value: &C,
    ) -> QueryState<D, (F, With<C>)>;
}

impl WorldIndexExtension for World {
    fn index_component<C: IndexComponent>(&mut self) -> &mut Self {
        if self.get_resource::<Index<C>>().is_none() {
            self.add_observer(Index::<C>::on_insert);
            self.add_observer(Index::<C>::on_replace);
        }

        self.init_resource::<Index<C>>();

        self
    }

    fn query_by_index<C: IndexComponent, D: QueryData, F: QueryFilter>(
        &mut self,
        value: &C,
    ) -> QueryState<D, (F, With<C>)> {
        self.resource_scope::<Index<C>, _>(|world, index| {
            let mut builder = QueryBuilder::<D, (F, With<C>)>::new(world);

            match index.mapping.get(value).copied() {
                // If there is a marker, restrict to it
                Some(component_id) => builder.with_id(component_id),
                // Otherwise, create a no-op query by including With<C> and Without<C>
                None => builder.without::<C>(),
            };

            let query = builder.build();

            query
        })
    }
}

/// Marker describing the requirements for a [`Component`] to be suitable for indexing.
pub trait IndexComponent: Component<Mutability = Immutable> + Eq + Hash + Clone {}

impl<C: Component<Mutability = Immutable> + Eq + Hash + Clone> IndexComponent for C {}

#[derive(Resource)]
struct Index<C: IndexComponent> {
    mapping: HashMap<C, ComponentId>,
    spare_markers: Vec<ComponentId>,
}

impl<C: IndexComponent> Default for Index<C> {
    fn default() -> Self {
        Self {
            mapping: Default::default(),
            spare_markers: Default::default(),
        }
    }
}

impl<C: IndexComponent> Index<C> {
    fn on_insert(trigger: Trigger<OnInsert, C>, mut commands: Commands) {
        let entity = trigger.target();

        commands.queue(move |world: &mut World| {
            world.resource_scope::<Self, _>(|world, mut index| {
                let Some(value) = world.get::<C>(entity) else {
                    return;
                };

                let component_id = match index.mapping.get(value).copied() {
                    Some(component_id) => component_id,
                    None => {
                        let value = value.clone();
                        let component_id = index
                            .spare_markers
                            .pop()
                            .unwrap_or_else(|| Self::alloc_new_marker(world));
                        index.mapping.insert(value, component_id);
                        component_id
                    }
                };

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
                world.entity_mut(entity).remove_by_id(component_id);

                let Self {
                    ref mut mapping,
                    ref mut spare_markers,
                    ..
                } = index.as_mut();

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
        world.register_component_with_descriptor(unsafe {
            ComponentDescriptor::new_with_layout(
                format!("Index Marker ({})", core::any::type_name::<Self>()),
                StorageType::Table,
                Layout::new::<()>(),
                None,
                false,
            )
        })
    }
}
