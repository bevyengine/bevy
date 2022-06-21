//! Hot reloading allows you to modify assets files to be immediately reloaded while your game is
//! running. This lets you immediately see the results of your changes without restarting the game.
//! This example illustrates hot reloading mesh changes.

use bevy::{asset::AssetServerSettings, prelude::*};

fn main() {
    App::new()
        // Tell the asset server to watch for asset changes on disk:
        .insert_resource(AssetServerSettings {
            watch_for_changes: true,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load our mesh:
    let scene_handle = asset_server.load("models/monkey/Monkey.gltf#Scene0");

    // Any changes to the mesh will be reloaded automatically! Try making a change to Monkey.gltf.
    // You should see the changes immediately show up in your app.

    // mesh
    commands.spawn_bundle(SceneBundle {
        scene: scene_handle,
        ..default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(2.0, 2.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
