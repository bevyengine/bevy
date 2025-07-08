//! This example illustrates the different ways you can employ component lifecycle hooks.
//!
//! Whenever possible, prefer using Bevy's change detection or Events for reacting to component changes.
//! Events generally offer better performance and more flexible integration into Bevy's systems.
//! Hooks are useful to enforce correctness but have limitations (only one hook per component,
//! less ergonomic than events).
//!
//! Here are some cases where components hooks might be necessary:
//!
//! - Maintaining indexes: If you need to keep custom data structures (like a spatial index) in
//!   sync with the addition/removal of components.
//!
//! - Enforcing structural rules: When you have systems that depend on specific relationships
//!   between components (like hierarchies or parent-child links) and need to maintain correctness.

use bevy::{
    ecs::component::{Mutable, StorageType},
    ecs::lifecycle::{ComponentHook, HookContext},
    prelude::*,
};
use std::collections::HashMap;

#[derive(Debug)]
/// Hooks can also be registered during component initialization by
/// using [`Component`] derive macro:
/// ```no_run
/// #[derive(Component)]
/// #[component(on_add = ..., on_insert = ..., on_replace = ..., on_remove = ...)]
/// ```
struct MyComponent(KeyCode);

impl Component for MyComponent {
    const STORAGE_TYPE: StorageType = StorageType::Table;
    type Mutability = Mutable;

    /// Hooks can also be registered during component initialization by
    /// implementing the associated method
    fn on_add() -> Option<ComponentHook> {
        // We don't have an `on_add` hook so we'll just return None.
        // Note that this is the default behavior when not implementing a hook.
        None
    }
}

#[derive(Resource, Default, Debug, Deref, DerefMut)]
struct MyComponentIndex(HashMap<KeyCode, Entity>);

#[derive(Event, BufferedEvent)]
struct MyEvent;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, trigger_hooks)
        .init_resource::<MyComponentIndex>()
        .add_event::<MyEvent>()
        .run();
}

fn setup(world: &mut World) {
    // In order to register component hooks the component must:
    // - not be currently in use by any entities in the world
    // - not already have a hook of that kind registered
    // This is to prevent overriding hooks defined in plugins and other crates as well as keeping things fast
    world
        .register_component_hooks::<MyComponent>()
        // There are 4 component lifecycle hooks: `on_add`, `on_insert`, `on_replace` and `on_remove`
        // A hook has 2 arguments:
        // - a `DeferredWorld`, this allows access to resource and component data as well as `Commands`
        // - a `HookContext`, this provides access to the following contextual information:
        //   - the entity that triggered the hook
        //   - the component id of the triggering component, this is mostly used for dynamic components
        //   - the location of the code that caused the hook to trigger
        //
        // `on_add` will trigger when a component is inserted onto an entity without it
        .on_add(
            |mut world,
             HookContext {
                 entity,
                 component_id,
                 caller,
                 ..
             }| {
                // You can access component data from within the hook
                let value = world.get::<MyComponent>(entity).unwrap().0;
                println!(
                    "{component_id:?} added to {entity} with value {value:?}{}",
                    caller
                        .map(|location| format!("due to {location}"))
                        .unwrap_or_default()
                );
                // Or access resources
                world
                    .resource_mut::<MyComponentIndex>()
                    .insert(value, entity);
                // Or send events
                world.write_event(MyEvent);
            },
        )
        // `on_insert` will trigger when a component is inserted onto an entity,
        // regardless of whether or not it already had it and after `on_add` if it ran
        .on_insert(|world, _| {
            println!("Current Index: {:?}", world.resource::<MyComponentIndex>());
        })
        // `on_replace` will trigger when a component is inserted onto an entity that already had it,
        // and runs before the value is replaced.
        // Also triggers when a component is removed from an entity, and runs before `on_remove`
        .on_replace(|mut world, context| {
            let value = world.get::<MyComponent>(context.entity).unwrap().0;
            world.resource_mut::<MyComponentIndex>().remove(&value);
        })
        // `on_remove` will trigger when a component is removed from an entity,
        // since it runs before the component is removed you can still access the component data
        .on_remove(
            |mut world,
             HookContext {
                 entity,
                 component_id,
                 caller,
                 ..
             }| {
                let value = world.get::<MyComponent>(entity).unwrap().0;
                println!(
                    "{component_id:?} removed from {entity} with value {value:?}{}",
                    caller
                        .map(|location| format!("due to {location}"))
                        .unwrap_or_default()
                );
                // You can also issue commands through `.commands()`
                world.commands().entity(entity).despawn();
            },
        );
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
