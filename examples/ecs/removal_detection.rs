// This example shows how you can know when a `Component` has been removed, so you can react to it.

use bevy::prelude::*;

fn main() {
    // Information regarding removed `Component`s is discarded at the end of each frame, so you need
    // to react to the removal before the frame is over.
    //
    // Also, `Components` are removed via a `Command`. `Command`s are applied after a stage has
    // finished executing. So you need to react to the removal at some stage after the
    // `Component` is removed.
    //
    // With these constraints in mind we make sure to place the system that removes a `Component` on
    // the `CoreStage::Update' stage, and the system that reacts on the removal on the
    // `CoreStage::PostUpdate` stage.
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_to_stage(CoreStage::Update, remove_component)
        .add_system_to_stage(CoreStage::PostUpdate, react_on_removal)
        .run();
}

// This `Struct` is just used for convenience in this example. This is the `Component` we'll be
// giving to the `Entity` so we have a `Component` to remove in `remove_component()`.
#[derive(Component)]
struct MyComponent;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("branding/icon.png"),
            ..Default::default()
        })
        .insert(MyComponent); // Add the `Component`.
}

fn remove_component(
    time: Res<Time>,
    mut commands: Commands,
    query: Query<Entity, With<MyComponent>>,
) {
    // After two seconds have passed the `Component` is removed.
    if time.seconds_since_startup() > 2.0 {
        if let Some(entity) = query.iter().next() {
            commands.entity(entity).remove::<MyComponent>();
        }
    }
}

fn react_on_removal(removed: RemovedComponents<MyComponent>, mut query: Query<&mut Sprite>) {
    // `RemovedComponents<T>::iter()` returns an interator with the `Entity`s that had their
    // `Component` `T` (in this case `MyComponent`) removed at some point earlier during the frame.
    for entity in removed.iter() {
        if let Ok(mut sprite) = query.get_mut(entity) {
            sprite.color.set_r(0.0);
        }
    }
}
