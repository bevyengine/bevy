use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // load the mesh
    let mesh_handle = asset_server
        .load("assets/models/monkey/Monkey.gltf")
        .unwrap();

    // create a material for the mesh
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    // add entities to the world
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
                Vec3::new(-2.0, 2.0, 6.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
