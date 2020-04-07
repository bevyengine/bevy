use bevy::prelude::*;

struct Rotator;

fn main() {
    App::build()
        .add_default_plugins()
        .setup(setup)
        .add_system(build_rotator_system())
        .run();
}

/// rotates the parent, which will result in the child also rotating
fn build_rotator_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("rotator")
        .read_resource::<Time>()
        .with_query(<(Write<Rotator>, Write<Rotation>)>::query())
        .build(move |_, world, time, rotator_query| {
            for (_rotator, mut rotation) in rotator_query.iter_mut(world) {
                rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
            }
        })
}

/// set up a simple scene with a "parent" cube and a "child" cube 
fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();

    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));
    let cube_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    world
        .build()
        // parent cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: cube_material_handle,
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .add(Rotator)
        .add_children(|builder| {
            // child cube
            builder.add_entity(MeshEntity {
                mesh: cube_handle,
                material: cube_material_handle,
                translation: Translation::new(0.0, 0.0, 3.0),
                ..Default::default()
            })
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // camera
        .add_entity(CameraEntity {
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        })
        .build();
}
