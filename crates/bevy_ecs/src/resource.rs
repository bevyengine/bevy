//! Resources are unique, singleton-like data types that can be accessed from systems and stored in the [`World`](crate::world::World).

use core::ops::{Deref, DerefMut};
use log::warn;

use crate::{
    component::{Component, ComponentId, Mutable},
    entity::Entity,
    lifecycle::HookContext,
    storage::SparseSet,
    world::DeferredWorld,
};
#[cfg(feature = "bevy_reflect")]
use {crate::reflect::ReflectComponent, bevy_reflect::Reflect};
// The derive macro for the `Resource` trait
pub use bevy_ecs_macros::Resource;
use bevy_platform::cell::SyncUnsafeCell;

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
pub trait Resource: Component<Mutability = Mutable> {}

/// A cache that links each `ComponentId` from a resource to the corresponding entity.
#[derive(Default)]
pub struct ResourceEntities(SyncUnsafeCell<SparseSet<ComponentId, Entity>>);

impl Deref for ResourceEntities {
    type Target = SparseSet<ComponentId, Entity>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: There are no other mutable references to the map.
        // The underlying `SyncUnsafeCell` is never exposed outside this module,
        // so mutable references are only created by the resource hooks.
        // We only expose `&ResourceCache` to code with access to a resource (such as `&World`),
        // and that would conflict with the `DeferredWorld` passed to the resource hook.
        unsafe { &*self.0.get() }
    }
}

impl DerefMut for ResourceEntities {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_mut()
    }
}

/// A marker component for entities that have a Resource component.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Debug))]
#[derive(Component, Debug)]
#[component(on_insert, on_discard, on_despawn)]
pub struct IsResource(ComponentId);

impl IsResource {
    /// Creates a new instance with the given `component_id`
    pub fn new(component_id: ComponentId) -> Self {
        Self(component_id)
    }

    /// The [`ComponentId`] of the resource component (the _actual_ resource value component, not the [`IsResource`] component).
    pub fn resource_component_id(&self) -> ComponentId {
        self.0
    }

    pub(crate) fn on_insert(mut world: DeferredWorld, context: HookContext) {
        let resource_component_id = world
            .entity(context.entity)
            .get::<Self>()
            .unwrap()
            .resource_component_id();

        if let Some(&original_entity) = world.resource_entities.get(resource_component_id) {
            if !world.entities().contains(original_entity) {
                let name = world
                    .components()
                    .get_name(resource_component_id)
                    .expect("resource is registered");
                panic!(
                    "Resource entity {} of {} has been despawned, when it's not supposed to be.",
                    original_entity, name
                );
            }

            if original_entity != context.entity {
                // the resource already exists and the new one should be removed
                world
                    .commands()
                    .entity(context.entity)
                    .remove_by_id(resource_component_id);
                world
                    .commands()
                    .entity(context.entity)
                    .remove_by_id(context.component_id);
                let name = world
                    .components()
                    .get_name(resource_component_id)
                    .expect("resource is registered");
                warn!("Tried inserting the resource {} while one already exists.
                Resources are unique components stored on a single entity.
                Inserting on a different entity, when one already exists, causes the new value to be removed.", name);
            }
        } else {
            // SAFETY: We have exclusive world access (as long as we don't make structural changes).
            let cache = unsafe { world.as_unsafe_world_cell().resource_entities() };
            // SAFETY: There are no shared references to the map.
            // We only expose `&ResourceCache` to code with access to a resource (such as `&World`),
            // and that would conflict with the `DeferredWorld` passed to the resource hook.
            unsafe { &mut *cache.0.get() }.insert(resource_component_id, context.entity);
        }
    }

    pub(crate) fn on_discard(mut world: DeferredWorld, context: HookContext) {
        let resource_component_id = world
            .entity(context.entity)
            .get::<Self>()
            .unwrap()
            .resource_component_id();

        if let Some(resource_entity) = world.resource_entities.get(resource_component_id)
            && *resource_entity == context.entity
        {
            // SAFETY: We have exclusive world access (as long as we don't make structural changes).
            let cache = unsafe { world.as_unsafe_world_cell().resource_entities() };
            // SAFETY: There are no shared references to the map.
            // We only expose `&ResourceCache` to code with access to a resource (such as `&World`),
            // and that would conflict with the `DeferredWorld` passed to the resource hook.
            unsafe { &mut *cache.0.get() }.remove(resource_component_id);

            world
                .commands()
                .entity(context.entity)
                .remove_by_id(resource_component_id);
        }
    }

    pub(crate) fn on_despawn(_world: DeferredWorld, _context: HookContext) {
        warn!("Resource entities are not supposed to be despawned.");
    }
}

/// [`ComponentId`] of the [`IsResource`] component.
pub const IS_RESOURCE: ComponentId = ComponentId::new(crate::component::IS_RESOURCE);

#[cfg(test)]
mod tests {
    use crate::{
        change_detection::MaybeLocation,
        entity::Entity,
        ptr::OwningPtr,
        resource::{IsResource, Resource},
        world::World,
    };
    use alloc::vec::Vec;
    use bevy_platform::prelude::String;

    #[test]
    fn unique_resource_entities() {
        #[derive(Default, Resource)]
        struct TestResource1;

        #[derive(Resource)]
        #[expect(dead_code, reason = "field needed for testing")]
        struct TestResource2(String);

        #[derive(Resource)]
        #[expect(dead_code, reason = "field needed for testing")]
        struct TestResource3(u8);

        let mut world = World::new();
        let start = world.entities().count_spawned();
        world.init_resource::<TestResource1>();
        assert_eq!(world.entities().count_spawned(), start + 1);
        world.insert_resource(TestResource2(String::from("Foo")));
        assert_eq!(world.entities().count_spawned(), start + 2);
        // like component registration, which just makes it known to the world that a component exists,
        // registering a resource should not spawn an entity.
        let id = world.register_resource::<TestResource3>();
        assert_eq!(world.entities().count_spawned(), start + 2);
        OwningPtr::make(20_u8, |ptr| {
            // SAFETY: id was just initialized and corresponds to a resource.
            unsafe {
                world.insert_resource_by_id(id, ptr, MaybeLocation::caller());
            }
        });
        assert_eq!(world.entities().count_spawned(), start + 3);
        assert!(world.remove_resource_by_id(id));
        // the entity is stable: removing the resource should only remove the component from the entity, not despawn the entity
        assert_eq!(world.entities().count_spawned(), start + 3);
        // again, the entity is stable: see previous explanation
        world.remove_resource::<TestResource1>();
        assert_eq!(world.entities().count_spawned(), start + 3);
        // make sure that trying to add a resource twice results, doesn't change the entity count
        world.insert_resource(TestResource2(String::from("Bar")));
        assert_eq!(world.entities().count_spawned(), start + 3);
    }

    #[test]
    fn is_resource_presence() {
        #[derive(Default, Resource)]
        struct TestResource;

        let mut world = World::new();
        let id = world.init_resource::<TestResource>();

        assert!(world.get_resource::<TestResource>().is_some());

        let mut query = world.query::<(Entity, &TestResource, &IsResource)>();
        let first_entity = {
            let resources = query.iter(&world).collect::<Vec<_>>();
            assert_eq!(resources.len(), 1);
            let (entity, _test_resource, is_resource) = resources[0];
            assert_eq!(is_resource.resource_component_id(), id);
            entity
        };

        // Removing IsResource should invalidate the current TestResource entity
        // This uses commands because IsResource's despawn-on-removal invalidates the EntityWorldMut and panics
        world.entity_mut(first_entity).remove::<IsResource>();
        assert!(world.get_resource::<TestResource>().is_none());

        assert!(
            !world.entity(first_entity).contains::<TestResource>(),
            "Removing IsResource should also remove the Resource component it corresponds to"
        );

        world.init_resource::<TestResource>();
        let second_entity = {
            let resources = query.iter(&world).collect::<Vec<_>>();
            assert_eq!(resources.len(), 1);
            let (entity, _test_resource, is_resource) = resources[0];
            assert_eq!(is_resource.resource_component_id(), id);
            entity
        };

        assert_ne!(
            first_entity, second_entity,
            "The first resource entity was invalidated, so the second initialization should be new"
        );

        let id = world.spawn(TestResource).id();
        // This spawned resource conflicts with the canonical resource, so it was cleaned up.
        assert!(world.entity(id).get::<TestResource>().is_none());
        assert!(world.entity(id).get::<IsResource>().is_none());
        assert!(world.entity(second_entity).get::<TestResource>().is_some());
        assert!(world.entity(second_entity).get::<IsResource>().is_some());
    }
}
