//! Resources are unique, singleton-like data types that can be accessed from systems and stored in the [`World`](crate::world::World).

use crate::entity_disabling::Internal;
use crate::prelude::Component;
use crate::prelude::ReflectComponent;
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use core::marker::PhantomData;
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
#[require(Internal, IsResource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Default))]
pub struct ResourceEntity<R: Resource>(#[reflect(ignore)] PhantomData<R>);

impl<R: Resource> Default for ResourceEntity<R> {
    fn default() -> Self {
        ResourceEntity(PhantomData)
    }
}

/// A marker component for entities which store resources.
///
/// By contrast, the [`ResourceEntity<R>`] component is used to find the entity that stores a particular resource.
/// This component is required by the [`ResourceEntity<R>`] component, and will automatically be added.
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug)
)]
#[derive(Component, Default, Debug)]
pub struct IsResource;

/// Used in conjunction with [`ResourceEntity<R>`], when no type information is available.
/// This is used by [`World::insert_resource_by_id`](crate::world::World).
#[derive(Resource)]
pub(crate) struct TypeErasedResource;

#[cfg(test)]
mod tests {
    use crate::change_detection::MaybeLocation;
    use crate::ptr::OwningPtr;
    use crate::resource::Resource;
    use crate::world::World;
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
        let start = world.entities().len();
        world.init_resource::<TestResource1>();
        assert_eq!(world.entities().len(), start + 1);
        world.insert_resource(TestResource2(String::from("Foo")));
        assert_eq!(world.entities().len(), start + 2);
        // like component registration, which just makes it known to the world that a component exists,
        // registering a resource should not spawn an entity.
        let id = world.register_resource::<TestResource3>();
        assert_eq!(world.entities().len(), start + 2);
        OwningPtr::make(20_u8, |ptr| {
            // SAFETY: id was just initialized and corresponds to a resource.
            unsafe {
                world.insert_resource_by_id(id, ptr, MaybeLocation::caller());
            }
        });
        assert_eq!(world.entities().len(), start + 3);
        assert!(world.remove_resource_by_id(id).is_some());
        assert_eq!(world.entities().len(), start + 2);
        world.remove_resource::<TestResource1>();
        assert_eq!(world.entities().len(), start + 1);
        // make sure that trying to add a resource twice results, doesn't change the entity count
        world.insert_resource(TestResource2(String::from("Bar")));
        assert_eq!(world.entities().len(), start + 1);
    }
}
