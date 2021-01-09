use bevy::prelude::*;

/// Hot reloading allows you to modify assets on disk and they will be "live reloaded" while your game is running.
/// This lets you immediately see the results of your changes without restarting the game.
/// This example illustrates hot reloading mesh changes.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    // Load our mesh:
    let scene_handle = asset_server.load("models/monkey/Monkey.gltf#Scene0");

    // Tell the asset server to watch for asset changes on disk:
    asset_server.watch_for_changes().unwrap();

    // Any changes to the mesh will be reloaded automatically! Try making a change to Monkey.gltf.
    // You should see the changes immediately show up in your app.

    // Add entities to the world:
    commands
        // mesh
        .spawn_scene(scene_handle)
        // light
        .spawn(LightBundle {
            transform: Transform::from_xyz(4.0, 5.0, 4.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(2.0, 2.0, 6.0)
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
