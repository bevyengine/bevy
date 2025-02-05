//! This example illustrates the way to register component hooks directly from the app.
//!
//! Whenever possible, prefer using Bevy's change detection or Events for reacting to component changes.
//! Events generally offer better performance and more flexible integration into Bevy's systems.
//! Hooks are useful to enforce correctness but have limitations (only one hook per component,
//! less ergonomic than events).
//!
//! Here are some cases where components hooks might be necessary:
//!
//! - Maintaining indexes: If you need to keep custom data structures (like a spatial index) in
//!     sync with the addition/removal of components.
//!
//! - Enforcing structural rules: When you have systems that depend on specific relationships
//!     between components (like hierarchies or parent-child links) and need to maintain correctness.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Component)]
struct MyComponent(KeyCode);

#[derive(Resource, Default, Debug, Deref, DerefMut)]
struct MyComponentIndex(HashMap<KeyCode, Entity>);

#[derive(Event)]
struct MyEvent;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, trigger_hooks)
        .with_component_hooks::<MyComponent>(|hooks| {
            hooks
                .on_add(|mut world, entity, component_id| {
                    // You can access component data from within the hook
                    let value = world.get::<MyComponent>(entity).unwrap().0;
                    println!(
                        "Component: {component_id:?} added to: {entity:?} with value {value:?}"
                    );
                    // Or access resources
                    world
                        .resource_mut::<MyComponentIndex>()
                        .insert(value, entity);
                    // Or send events
                    world.send_event(MyEvent);
                })
                // `on_insert` will trigger when a component is inserted onto an entity,
                // regardless of whether or not it already had it and after `on_add` if it ran
                .on_insert(|world, _, _| {
                    println!("Current Index: {:?}", world.resource::<MyComponentIndex>());
                })
                // `on_replace` will trigger when a component is inserted onto an entity that already had it,
                // and runs before the value is replaced.
                // Also triggers when a component is removed from an entity, and runs before `on_remove`
                .on_replace(|mut world, entity, _| {
                    let value = world.get::<MyComponent>(entity).unwrap().0;
                    world.resource_mut::<MyComponentIndex>().remove(&value);
                })
                // `on_remove` will trigger when a component is removed from an entity,
                // since it runs before the component is removed you can still access the component data
                .on_remove(|mut world, entity, component_id| {
                    let value = world.get::<MyComponent>(entity).unwrap().0;
                    println!(
                        "Component: {component_id:?} removed from: {entity:?} with value {value:?}"
                    );
                    // You can also issue commands through `.commands()`
                    world.commands().entity(entity).despawn();
                });
        })
        .init_resource::<MyComponentIndex>()
        .add_event::<MyEvent>()
        .run();
}

fn trigger_hooks(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    index: Res<MyComponentIndex>,
) {
    for (key, entity) in index.iter() {
        if !keys.pressed(*key) {
            commands.entity(*entity).remove::<MyComponent>();
        }
    }
    for key in keys.get_just_pressed() {
        commands.spawn(MyComponent(*key));
    }
}
