#![allow(unsafe_code)]

//! This example show how you can create immutable components dynamically.

use core::alloc::Layout;

use bevy::{
    ecs::component::{ComponentDescriptor, ComponentId, StorageType},
    prelude::*,
    ptr::OwningPtr,
};

fn setup(world: &mut World) {
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
                ComponentDescriptor::new_immutable_with_layout(
                    name.to_string(),
                    StorageType::Table,
                    Layout::array::<u8>(size).unwrap(),
                    None,
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
    App::new().add_systems(Startup, setup).run();
}
