use bevy::prelude::*;
use bevy_asset::AssetServer;

/// Hot reloading allows you to modify assets on disk and they will be "live reloaded" while your game is running.
/// This lets you immediately see the results of your changes without restarting the game.
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    mut asset_server: ResMut<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Load an asset folder:
    asset_server.load_asset_folder("assets").unwrap();

    // Tell the asset server to watch for asset changes on disk:
    asset_server.watch_for_changes().unwrap();

    // Get a handle for our mesh:
    let mesh_handle = asset_server
        .get_handle("assets/models/monkey/Monkey.gltf")
        .unwrap();

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
        .add_entity(MeshEntity {
            mesh: mesh_handle,
            material: material_handle,
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // camera
        .add_entity(CameraEntity {
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(2.0, -6.0, 2.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
