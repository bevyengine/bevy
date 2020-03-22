use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

fn main() {
    App::build()
        .add_defaults()
        .add_system(build_move_system())
        .add_system(bevy::diagnostics::build_fps_printer_system())
        .setup_world(setup)
        .run();
}

fn build_move_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("Move")
        .read_resource::<Time>()
        .write_resource::<AssetStorage<StandardMaterial>>()
        .with_query(<(Write<Translation>, Read<Handle<StandardMaterial>>)>::query())
        .build(move |_, world, (time, material_storage), person_query| {
            for (mut translation, material_handle) in person_query.iter_mut(world) {
                let material = material_storage.get_mut(&material_handle).unwrap();
                translation.0 += math::vec3(1.0, 0.0, 0.0) * time.delta_seconds;
                material.albedo = material.albedo
                    + Color::rgb(-time.delta_seconds, -time.delta_seconds, time.delta_seconds);
            }
        })
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));
    let plane_handle = mesh_storage.add(Mesh::load(MeshType::Plane { size: 10.0 }));
    let cube_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });
    let plane_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.1, 0.2, 0.1),
        ..Default::default()
    });

    let mut builder = world
        .build()
        // plane
        .add_entity(MeshEntity {
            mesh: plane_handle,
            material: plane_material_handle,
            ..Default::default()
        })
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: cube_material_handle,
            translation: Translation::new(0.0, 0.0, 1.0),
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
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });

    let mut rng = StdRng::from_entropy();
    for _ in 0..500 {
        let spawned_material_handle = material_storage.add(StandardMaterial {
            albedo: Color::rgb(
                rng.gen_range(0.0, 1.0),
                rng.gen_range(0.0, 1.0),
                rng.gen_range(0.0, 1.0),
            ),
            ..Default::default()
        });
        builder = builder.add_entity(MeshEntity {
            mesh: cube_handle,
            material: spawned_material_handle,
            translation: Translation::new(
                rng.gen_range(-50.0, 50.0),
                rng.gen_range(-50.0, 50.0),
                0.0,
            ),
            renderable: Renderable {
                is_instanced: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    builder.build();
}
