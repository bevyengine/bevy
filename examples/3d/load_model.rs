use bevy::{gltf, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .run();
}

fn setup(world: &mut World, resources: &mut Resources) {
    // load the mesh
    let model_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/Monkey.gltf");
    let mesh = gltf::load_gltf(&model_path).unwrap().unwrap();
    let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();
    let mesh_handle = meshes.add(mesh);

    // create a material for the mesh
    let mut materials = resources
        .get_mut::<Assets<StandardMaterial>>()
        .unwrap();
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    // add entities to the world
    world
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
