use bevy::prelude::*;

struct Rotator;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .run();
}

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, _rotator: ComMut<Rotator>, mut rotation: ComMut<Rotation>) {
    rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(
    command_buffer: &mut CommandBuffer,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    command_buffer
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
        .add_entity(PerspectiveCameraEntity {
            local_to_world: LocalToWorld::new_sync_disabled(Mat4::look_at_rh(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
