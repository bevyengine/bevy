use bevy::prelude::*;

/// Hot reloading allows you to modify assets on disk and they will be "live reloaded" while your game is running.
/// This lets you immediately see the results of your changes without restarting the game.
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    command_buffer: &mut CommandBuffer,
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
    command_buffer
        .build()
        // mesh
        .entity_with(MeshComponents {
            mesh: mesh_handle,
            material: material_handle,
            ..Default::default()
        })
        // light
        .entity_with(LightComponents {
            translation: Translation::new(4.0, 5.0, 4.0),
            ..Default::default()
        })
        // camera
        .entity_with(PerspectiveCameraComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(2.0, 2.0, 6.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
