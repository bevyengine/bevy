//! Resources are unique, singleton-like data types that can be accessed from systems and stored in the [`World`](crate::world::World).

use core::ops::Deref;

use crate::{
    entity_disabling::Internal,
    lifecycle::HookContext,
    prelude::{Component, ReflectComponent},
    world::DeferredWorld,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};

// The derive macro for the `Resource` trait
pub use bevy_ecs_macros::Resource;

/// A type that can be inserted into a [`World`] as a singleton.
///
/// You can access resource data in systems using the [`Res`] and [`ResMut`] system parameters
///
/// Only one resource of each type can be stored in a [`World`] at any given time.
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
/// use bevy_platform::cell::SyncCell;
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
/// [`SyncCell`]: bevy_platform::cell::SyncCell
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Resource`",
    label = "invalid `Resource`",
    note = "consider annotating `{Self}` with `#[derive(Resource)]`"
)]
pub trait Resource: Send + Sync + 'static {}

/// A component that contains the resource of type `T`.
///
/// When creating a resource, a [`ResourceComponent`] is inserted on a new entity in the world.
///
/// This component comes with a hook that ensures that at most one entity has this component for any given `R`:
/// adding this component to an entity (or spawning an entity with this component) will despawn any other entity with this component.
/// Moreover, this component requires both marker components [`IsResource`] and [`Internal`].
/// The former can be used to quickly iterate over all resources through a query,
/// while the latter marks the associated entity as internal, ensuring that it won't show up on broad queries such as
/// `world.query::<Entity>()`.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
#[derive(Component)]
#[require(IsResource, Internal)]
#[component(on_add = on_add_hook, on_remove = on_remove_hook)]
#[repr(transparent)]
pub struct ResourceComponent<R: Resource>(pub R);

pub(crate) fn on_add_hook(mut deferred_world: DeferredWorld, context: HookContext) {
    let world = deferred_world.deref();
    if world.resource_entities.contains(context.component_id) {
        // the resource already exists and we need to overwrite it
        let offending_entity = *world.resource_entities.get(context.component_id).unwrap();
        deferred_world.commands().entity(offending_entity).despawn();
    }
    // we update the cache
    // SAFETY: We only update a cache and don't perform any structural changes (component adds / removals)
    unsafe {
        deferred_world
            .as_unsafe_world_cell()
            .world_mut()
            .resource_entities
            .insert(context.component_id, context.entity);
    }
}

pub(crate) fn on_remove_hook(mut deferred_world: DeferredWorld, context: HookContext) {
    let world = deferred_world.deref();
    // If the resource is already linked to a new (different) entity, we don't remove it.
    if let Some(entity) = world.resource_entities.get(context.component_id)
        && *entity == context.entity
    {
        // SAFETY: We only update a cache and don't perform any structural changes (component adds / removals)
        unsafe {
            deferred_world
                .as_unsafe_world_cell()
                .world_mut()
                .resource_entities
                .remove(context.component_id);
        }
    }
}

/// A marker component for entities which store resources.
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug)
)]
#[derive(Component, Debug, Default)]
#[require(Internal)]
pub struct IsResource;
