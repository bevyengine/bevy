use bevy::prelude::*;

struct Rotator;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .add_system(camera_order_color_system.system())
        .run();
}

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, _rotator: ComMut<Rotator>, mut rotation: ComMut<Rotation>) {
    rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
}

fn camera_order_color_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: &mut SubWorld,
    camera_query: &mut Query<(Read<Camera>, Read<VisibleEntities>)>,
    _material_query: &mut Query<Read<Handle<StandardMaterial>>>,
) {
    for (_camera, visible_entities) in camera_query.iter(world) {
        for visible_entity in visible_entities.iter() {
            if let Some(material_handle) =
                world.get_component::<Handle<StandardMaterial>>(visible_entity.entity)
            {
                let material = materials.get_mut(&material_handle).unwrap();
                let value = 1.0 - (visible_entity.order.0 - 10.0) / 7.0;
                material.albedo = Color::rgb(value, value, value);
            }
        }
    }
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    command_buffer: &mut CommandBuffer,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    command_buffer
        .build()
        // parent cube
        .entity_with(MeshComponents {
            mesh: cube_handle,
            material: materials.add(StandardMaterial {
                shaded: false,
                ..Default::default()
            }),
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|builder| {
            // child cubes
            builder
                .entity_with(MeshComponents {
                    mesh: cube_handle,
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    translation: Translation::new(0.0, 3.0, 0.0),
                    ..Default::default()
                })
                .entity_with(MeshComponents {
                    mesh: cube_handle,
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    translation: Translation::new(0.0, -3.0, 0.0),
                    ..Default::default()
                })
        })
        // camera
        .entity_with(PerspectiveCameraComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
