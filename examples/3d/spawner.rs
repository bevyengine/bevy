use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system(move_system.system())
        .run();
}

fn move_system(
    time: Resource<Time>,
    mut materials: ResourceMut<Assets<StandardMaterial>>,
    mut translation: RefMut<Translation>,
    material_handle: Ref<Handle<StandardMaterial>>,
) {
    let material = materials.get_mut(&material_handle).unwrap();
    translation.0 += math::vec3(1.0, 0.0, 0.0) * time.delta_seconds;
    material.albedo += Color::rgb(-time.delta_seconds, -time.delta_seconds, time.delta_seconds);
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();
    let mut materials = resources
        .get_mut::<Assets<StandardMaterial>>()
        .unwrap();
    let cube_handle = meshes.add(Mesh::from(shape::Cube));
    let plane_handle = meshes.add(Mesh::from(shape::Plane { size: 10.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });
    let plane_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.1, 0.2, 0.1),
        ..Default::default()
    });

    let mut builder = world.build();
    builder
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
    for _ in 0..10000 {
        let spawned_material_handle = materials.add(StandardMaterial {
            albedo: Color::rgb(
                rng.gen_range(0.0, 1.0),
                rng.gen_range(0.0, 1.0),
                rng.gen_range(0.0, 1.0),
            ),
            ..Default::default()
        });
        builder.add_entity(MeshEntity {
            mesh: cube_handle,
            material: spawned_material_handle,
            translation: Translation::new(
                rng.gen_range(-50.0, 50.0),
                rng.gen_range(-50.0, 50.0),
                0.0,
            ),
            ..Default::default()
        });
    }
}
