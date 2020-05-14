use bevy::{gltf, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    mut meshes: ResourceMut<Assets<Mesh>>,
    mut materials: ResourceMut<Assets<StandardMaterial>>,
) {
    // load the mesh
    let model_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/Monkey.gltf");
    let mesh = gltf::load_gltf(&model_path).unwrap().unwrap();
    let mesh_handle = meshes.add(mesh);

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
