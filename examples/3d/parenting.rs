use bevy::prelude::*;

struct Rotator;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .add_system(rotator_system.system())
        .run();
}

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Resource<Time>, _rotator: RefMut<Rotator>, mut rotation: RefMut<Rotation>) {
    rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(world: &mut World, resources: &mut Resources) {
    let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();
    let mut material_storage = resources
        .get_mut::<Assets<StandardMaterial>>()
        .unwrap();

    let cube_handle = meshes.add(Mesh::from(shape::Cube));
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
            });
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
        });
}
