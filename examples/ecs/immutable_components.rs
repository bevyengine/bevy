//! This example demonstrates immutable components.

use bevy::{
    ecs::{
        component::{ComponentCloneBehavior, ComponentDescriptor, ComponentId, StorageType},
        lifecycle::HookContext,
        world::DeferredWorld,
    },
    platform::collections::HashMap,
    prelude::*,
    ptr::OwningPtr,
};
use core::alloc::Layout;

/// This component is mutable, the default case. This is indicated by components
/// implementing [`Component`] where [`Component::Mutability`] is [`Mutable`](bevy::ecs::component::Mutable).
#[derive(Component)]
pub struct MyMutableComponent(bool);

/// This component is immutable. Once inserted into the ECS, it can only be viewed,
/// or removed. Replacement is also permitted, as this is equivalent to removal
/// and insertion.
///
/// Adding the `#[component(immutable)]` attribute prevents the implementation of [`Component<Mutability = Mutable>`]
/// in the derive macro.
#[derive(Component)]
#[component(immutable)]
pub struct MyImmutableComponent(bool);

fn demo_1(world: &mut World) {
    // Immutable components can be inserted just like mutable components.
    let mut entity = world.spawn((MyMutableComponent(false), MyImmutableComponent(false)));

    // But where mutable components can be mutated...
    let mut my_mutable_component = entity.get_mut::<MyMutableComponent>().unwrap();
    my_mutable_component.0 = true;

    // ...immutable ones cannot. The below fails to compile as `MyImmutableComponent`
    // is declared as immutable.
    // let mut my_immutable_component = entity.get_mut::<MyImmutableComponent>().unwrap();

    // Instead, you could take or replace the immutable component to update its value.
    let mut my_immutable_component = entity.take::<MyImmutableComponent>().unwrap();
    my_immutable_component.0 = true;
    entity.insert(my_immutable_component);
}

/// This is an example of a component like [`Name`](bevy::prelude::Name), but immutable.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component, Reflect)]
#[reflect(Hash, Component)]
#[component(
    immutable,
    // Since this component is immutable, we can fully capture all mutations through
    // these component hooks. This allows for keeping other parts of the ECS synced
    // to a component's value at all times.
    on_insert = on_insert_name,
    on_replace = on_replace_name,
)]
pub struct Name(pub &'static str);

/// This index allows for O(1) lookups of an [`Entity`] by its [`Name`].
#[derive(Resource, Default)]
struct NameIndex {
    name_to_entity: HashMap<Name, Entity>,
}

impl NameIndex {
    fn get_entity(&self, name: &'static str) -> Option<Entity> {
        self.name_to_entity.get(&Name(name)).copied()
    }
}

/// When a [`Name`] is inserted, we will add it to our [`NameIndex`].
///
/// Since all mutations to [`Name`] are captured by hooks, we know it is not currently
/// inserted in the index, and its value will not change without triggering a hook.
fn on_insert_name(mut world: DeferredWorld<'_>, HookContext { entity, .. }: HookContext) {
    let Some(&name) = world.entity(entity).get::<Name>() else {
        unreachable!("Insert hook guarantees `Name` is available on entity")
    };
    let Some(mut index) = world.get_resource_mut::<NameIndex>() else {
        return;
    };

    index.name_to_entity.insert(name, entity);
}

/// When a [`Name`] is removed or replaced, remove it from our [`NameIndex`].
///
/// Since all mutations to [`Name`] are captured by hooks, we know it is currently
/// inserted in the index.
fn on_replace_name(mut world: DeferredWorld<'_>, HookContext { entity, .. }: HookContext) {
    let Some(&name) = world.entity(entity).get::<Name>() else {
        unreachable!("Replace hook guarantees `Name` is available on entity")
    };
    let Some(mut index) = world.get_resource_mut::<NameIndex>() else {
        return;
    };

    index.name_to_entity.remove(&name);
}

fn demo_2(world: &mut World) {
    // Setup our name index
    world.init_resource::<NameIndex>();

    // Spawn some entities!
    let alyssa = world.spawn(Name("Alyssa")).id();
    let javier = world.spawn(Name("Javier")).id();

    // Check our index
    let index = world.resource::<NameIndex>();

    assert_eq!(index.get_entity("Alyssa"), Some(alyssa));
    assert_eq!(index.get_entity("Javier"), Some(javier));

    // Changing the name of an entity is also fully capture by our index
    world.entity_mut(javier).insert(Name("Steven"));

    // Javier changed their name to Steven
    let steven = javier;

    // Check our index
    let index = world.resource::<NameIndex>();

    assert_eq!(index.get_entity("Javier"), None);
    assert_eq!(index.get_entity("Steven"), Some(steven));
}

/// This example demonstrates how to work with _dynamic_ immutable components.
#[expect(
    unsafe_code,
    reason = "Unsafe code is needed to work with dynamic components"
)]
fn demo_3(world: &mut World) {
    // This is a list of dynamic components we will create.
    // The first item is the name of the component, and the second is the size
    // in bytes.
    let my_dynamic_components = [("Foo", 1), ("Bar", 2), ("Baz", 4)];

    // This pipeline takes our component descriptions, registers them, and gets
    // their ComponentId's.
    let my_registered_components = my_dynamic_components
        .into_iter()
        .map(|(name, size)| {
            // SAFETY:
            // - No drop command is required
            // - The component will store [u8; size], which is Send + Sync
            let descriptor = unsafe {
                ComponentDescriptor::new_with_layout(
                    name.to_string(),
                    StorageType::Table,
                    Layout::array::<u8>(size).unwrap(),
                    None,
                    false,
                    ComponentCloneBehavior::Default,
                )
            };

            (name, size, descriptor)
        })
        .map(|(name, size, descriptor)| {
            let component_id = world.register_component_with_descriptor(descriptor);

            (name, size, component_id)
        })
        .collect::<Vec<(&str, usize, ComponentId)>>();

    // Now that our components are registered, let's add them to an entity
    let mut entity = world.spawn_empty();

    for (_name, size, component_id) in &my_registered_components {
        // We're just storing some zeroes for the sake of demonstration.
        let data = core::iter::repeat_n(0, *size).collect::<Vec<u8>>();

        OwningPtr::make(data, |ptr| {
            // SAFETY:
            // - ComponentId has been taken from the same world
            // - Array is created to the layout specified in the world
            unsafe {
                entity.insert_by_id(*component_id, ptr);
            }
        });
    }

    for (_name, _size, component_id) in &my_registered_components {
        // With immutable components, we can read the values...
        assert!(entity.get_by_id(*component_id).is_ok());

        // ...but we cannot gain a mutable reference.
        assert!(entity.get_mut_by_id(*component_id).is_err());

        // Instead, you must either remove or replace the value.
    }
}

fn main() {
    App::new()
        .add_systems(Startup, demo_1)
        .add_systems(Startup, demo_2)
        .add_systems(Startup, demo_3)
        .run();
}
