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
    // load an asset folder
    asset_server.load_asset_folder("assets").unwrap();

    // tell the asset server to watch for changes
    asset_server.watch_for_changes().unwrap();

    // load the mesh
    let mesh_handle = asset_server
        .get_handle("assets/models/monkey/Monkey.gltf")
        .unwrap();

    // now any changes to the mesh will be reloaded automatically!

    // create a material for the mesh
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    // add entities to the world
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
