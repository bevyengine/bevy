use bevy::{
    prelude::*,
    render::{
        camera::{Camera, VisibleEntities},
        mesh::shape,
    },
};

/// This example visualizes camera z-ordering by setting the material of rotating cubes to their distance from the camera
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .add_system(camera_order_color_system.system())
        .run();
}

struct Rotator;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotator>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(3.0 * time.delta_seconds());
    }
}

fn camera_order_color_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&VisibleEntities, With<Camera>>,
    material_query: Query<&Handle<StandardMaterial>>,
) {
    for visible_entities in camera_query.iter() {
        for visible_entity in visible_entities.iter() {
            if let Ok(material_handle) = material_query.get(visible_entity.entity) {
                let material = materials.get_mut(&*material_handle).unwrap();
                let value = 1.0 - (visible_entity.order.0 - 10.0) / 7.0;
                material.albedo = Color::rgb(value, value, value);
            }
        }
    }
}

fn setup(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 2.0 }));
    commands
        // parent cube
        .spawn(PbrBundle {
            mesh: cube_handle.clone(),
            material: materials.add(StandardMaterial {
                shaded: false,
                ..Default::default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|parent| {
            // child cubes
            parent
                .spawn(PbrBundle {
                    mesh: cube_handle.clone(),
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    transform: Transform::from_xyz(0.0, 3.0, 0.0),
                    ..Default::default()
                })
                .spawn(PbrBundle {
                    mesh: cube_handle,
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    transform: Transform::from_xyz(0.0, -3.0, 0.0),
                    ..Default::default()
                });
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(5.0, 10.0, 10.0)
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
