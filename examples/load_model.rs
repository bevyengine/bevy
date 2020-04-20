use bevy::{gltf, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .add_system_init(bevy::input::system::exit_on_esc_system)
        .run();
}

fn setup(world: &mut World, resources: &mut Resources) {
    // load the mesh
    let mesh = gltf::load_gltf("examples/assets/Monkey.gltf")
        .unwrap()
        .unwrap();
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mesh_handle = mesh_storage.add(mesh);

    // create a material for the mesh
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();
    let material_handle = material_storage.add(StandardMaterial {
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
