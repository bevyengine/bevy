use bevy::prelude::*;

/// Hot reloading allows you to modify assets on disk and they will be "live reloaded" while your game is running.
/// This lets you immediately see the results of your changes without restarting the game.
/// This example illustrates hot reloading mesh changes.
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Load our mesh:
    let mesh_handle = asset_server
        .load("assets/models/monkey/Monkey.gltf")
        .unwrap();

    // Tell the asset server to watch for asset changes on disk:
    asset_server.watch_for_changes().unwrap();

    // Any changes to the mesh will be reloaded automatically! Try making a change to Monkey.gltf.
    // You should see the changes immediately show up in your app.

    // Create a material for the mesh:
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    // Add entities to the world:
    commands
        // mesh
        .spawn(PbrComponents {
            mesh: mesh_handle,
            material: material_handle,
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, 4.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(2.0, 2.0, 6.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
