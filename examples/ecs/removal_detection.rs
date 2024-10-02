//! This example shows how you can know when a [`Component`] has been removed, so you can react to it.
//!
//! When a [`Component`] is removed from an [`Entity`], all [`Observer`] with an [`OnRemove`] trigger for
//! that [`Component`] will be notified. These observers will be called immediately after the
//! [`Component`] is removed. For more info on observers, see the
//! [observers example](https://github.com/bevyengine/bevy/blob/main/examples/ecs/observers.rs).
//!
//! Advanced users may also consider using a lifecycle hook
//! instead of an observer, as it incurs less overhead for a case like this.
//! See the [component hooks example](https://github.com/bevyengine/bevy/blob/main/examples/ecs/component_hooks.rs).
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // This system will remove a component after two seconds.
        .add_systems(Update, remove_component)
        // This observer will react to the removal of the component.
        .observe(react_on_removal)
        .run();
}

/// This `struct` is just used for convenience in this example. This is the [`Component`] we'll be
/// giving to the `Entity` so we have a [`Component`] to remove in `remove_component()`.
#[derive(Component)]
struct MyComponent;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("branding/icon.png"),
            ..default()
        },
        // Add the `Component`.
        MyComponent,
    ));
}

fn remove_component(
    time: Res<Time>,
    mut commands: Commands,
    query: Query<Entity, With<MyComponent>>,
) {
    // After two seconds have passed the `Component` is removed.
    if time.elapsed_seconds() > 2.0 {
        if let Some(entity) = query.iter().next() {
            commands.entity(entity).remove::<MyComponent>();
        }
    }
}

fn react_on_removal(trigger: Trigger<OnRemove, MyComponent>, mut query: Query<&mut Sprite>) {
    // The `OnRemove` trigger was automatically called on the `Entity` that had its `MyComponent` removed.
    let entity = trigger.entity();
    if let Ok(mut sprite) = query.get_mut(entity) {
        sprite.color = Color::srgb(0.5, 1., 1.);
    }
}
