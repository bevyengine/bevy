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
    world: &mut SubWorld,
    camera_query: &mut Query<(Read<Camera>, Read<VisibleEntities>)>,
    _material_query: &mut Query<Write<StandardMaterial>>,
) {
    for (_camera, visible_entities) in camera_query.iter(world) {
        for visible_entity in visible_entities.value.iter() {
            println!("visible_entity: {:?}", visible_entity.order);
            // let mut material = world.get_component_mut::<StandardMaterial>(visible_entity.entity).unwrap();
            // println!("entity {:?}", visible_entity.order);
        }
    }
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(
    command_buffer: &mut CommandBuffer,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    command_buffer
        .build()
        // parent cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: materials.add(StandardMaterial {
                shaded: false,
                ..Default::default()
            }),
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .add(Rotator)
        .add_children(|builder| {
            // child cubes
            builder
                .add_entity(MeshEntity {
                    mesh: cube_handle,
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    translation: Translation::new(0.0, 0.0, 3.0),
                    ..Default::default()
                })
                .add_entity(MeshEntity {
                    mesh: cube_handle,
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    translation: Translation::new(0.0, 0.0, -3.0),
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
            transform: Transform::new_sync_disabled(Mat4::look_at_rh(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
