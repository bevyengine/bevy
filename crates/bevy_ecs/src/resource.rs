//! Resources are unique, singleton-like data types that can be accessed from systems and stored in the [`World`](crate::world::World).
//!
//! Under the hood, each resource of type `R` is stored in a dedicated entity in the world,
//! with the data of type `R` stored as a component on that entity.
//! These entities are marked with the [`ResourceEntity<R>`] component and the [`IsResource`] component.
//! This strategy allows Bevy to reuse the existing ECS tools for working with resources:
//! storage, querying, hooks, observers, relationships and more.
//!
//! While resources are components, not all resources are components!
//! The [`Resource`] trait is used to mark components which can be used as such,
//! and must be derived for any type that is to be used as a resource.
//! The various methods for inserting and accessing resources require this trait bound (when working with Rust types),
//! and the simplest, clearest way to access resource data in systems is to use the [`Res`] and [`ResMut`] system parameters.
//!
//! Because resources are *also* components, queries will find the component on the entity which stores the resource
//! by default, and operate on it like any other entity. If this behavior is not desired, filter out
//! entities with the [`IsResource`] component.
//!
//! [`Res`]: crate::system::Res
//! [`ResMut`]: crate::system::ResMut

use crate as bevy_ecs;
use crate::component::ComponentId;
use crate::entity::Entity;
use crate::prelude::{require, Component};
use crate::query::With;
use crate::world::DeferredWorld;
use alloc::vec::Vec;
use core::marker::PhantomData;

// The derive macro for the `Resource` trait
pub use bevy_ecs_macros::Resource;

/// A type that can be inserted into a [`World`] as a singleton.
///
/// You can access resource data in systems using the [`Res`] and [`ResMut`] system parameters
///
/// Only one resource of each type can be stored in a [`World`] at any given time.
///
/// # Deriving this trait
///
/// This trait can be derived! The derive macro also implements the [`Component`] trait for the type,
/// and any attributes that are valid for the [`Component`] derive are also applied.
///
/// # Examples
///
/// ```
/// # let mut world = World::default();
/// # let mut schedule = Schedule::default();
/// # use bevy_ecs::prelude::*;
/// #[derive(Resource)]
/// struct MyResource { value: u32 }
///
/// world.insert_resource(MyResource { value: 42 });
///
/// fn read_resource_system(resource: Res<MyResource>) {
///     assert_eq!(resource.value, 42);
/// }
///
/// fn write_resource_system(mut resource: ResMut<MyResource>) {
///     assert_eq!(resource.value, 42);
///     resource.value = 0;
///     assert_eq!(resource.value, 0);
/// }
/// # schedule.add_systems((read_resource_system, write_resource_system).chain());
/// # schedule.run(&mut world);
/// ```
///
/// # `!Sync` Resources
/// A `!Sync` type cannot implement `Resource`. However, it is possible to wrap a `Send` but not `Sync`
/// type in [`SyncCell`] or the currently unstable [`Exclusive`] to make it `Sync`. This forces only
/// having mutable access (`&mut T` only, never `&T`), but makes it safe to reference across multiple
/// threads.
///
/// This will fail to compile since `RefCell` is `!Sync`.
/// ```compile_fail
/// # use std::cell::RefCell;
/// # use bevy_ecs::resource::Resource;
///
/// #[derive(Resource)]
/// struct NotSync {
///    counter: RefCell<usize>,
/// }
/// ```
///
/// This will compile since the `RefCell` is wrapped with `SyncCell`.
/// ```
/// # use std::cell::RefCell;
/// # use bevy_ecs::resource::Resource;
/// use bevy_utils::synccell::SyncCell;
///
/// #[derive(Resource)]
/// struct ActuallySync {
///    counter: SyncCell<RefCell<usize>>,
/// }
/// ```
///
/// [`Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html
/// [`World`]: crate::world::World
/// [`Res`]: crate::system::Res
/// [`ResMut`]: crate::system::ResMut
/// [`SyncCell`]: bevy_utils::synccell::SyncCell
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Resource`",
    label = "invalid `Resource`",
    note = "consider annotating `{Self}` with `#[derive(Resource)]`"
)]
pub trait Resource: Component {}

/// A marker component for the entity that stores the resource of type `T`.
///
/// This component is automatically inserted when a resource of type `T` is inserted into the world,
/// and can be used to find the entity that stores a particular resource.
///
/// By contrast, the [`IsResource`] component is used to find all entities that store resources,
/// regardless of the type of resource they store.
///
/// This component comes with a hook that ensures that at most one entity has this component for any given `R`:
/// adding this component to an entity (or spawning an entity with this component) will despawn any other entity with this component.
#[derive(Component, Debug)]
#[require(IsResource)]
#[component(on_insert = at_most_one_hook::<R>)]
pub struct ResourceEntity<R: Resource>(PhantomData<R>);

impl<R: Resource> Default for ResourceEntity<R> {
    fn default() -> Self {
        ResourceEntity(PhantomData)
    }
}

fn at_most_one_hook<R: Resource>(
    mut deferred_world: DeferredWorld,
    entity: Entity,
    _component_id: ComponentId,
) {
    let mut query = deferred_world
        .try_query_filtered::<Entity, With<ResourceEntity<R>>>()
        // The component is guaranteed to have been added to the world,
        // since that's why this hook is running!
        .unwrap();

    let mut offending_entities = Vec::new();

    for detected_entity in query.iter(&deferred_world) {
        if detected_entity != entity {
            offending_entities.push(detected_entity);
        }
    }

    let mut commands = deferred_world.commands();
    for offending_entity in offending_entities {
        commands.entity(offending_entity).despawn();
    }
}

/// A marker component for entities which store resources.
///
/// By contrast, the [`ResourceEntity<R>`] component is used to find the entity that stores a particular resource.
/// This component is required by the [`ResourceEntity<R>`] component, and will automatically be added.
#[derive(Component, Default, Debug)]
pub struct IsResource;

#[cfg(test)]
mod tests {
    use super::ResourceEntity;
    use crate as bevy_ecs;
    use crate::prelude::*;

    #[test]
    fn resource_with_component_attributes() {
        #[derive(Resource, Default)]
        struct RA;

        #[derive(Resource)]
        #[require(RA)]
        struct RB;
    }

    #[test]
    fn at_most_one_resource_entity_exists() {
        #[derive(Resource, Default)]
        struct R;

        let mut world = World::default();
        world.init_resource::<R>();

        let mut resource_query = world.query::<&ResourceEntity<R>>();
        assert_eq!(resource_query.iter(&world).count(), 1);

        world.insert_resource(R);
        let mut resource_query = world.query::<&ResourceEntity<R>>();
        assert_eq!(resource_query.iter(&world).count(), 1);
    }
}
