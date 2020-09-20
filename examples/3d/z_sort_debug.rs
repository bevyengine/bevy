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
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .add_system(camera_order_color_system.system())
        .run();
}

struct Rotator;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<(&Rotator, &mut Transform)>) {
    for (_rotator, mut transform) in &mut query.iter() {
        let rotation = transform.rotation() * Quat::from_rotation_x(3.0 * time.delta_seconds);
        transform.set_rotation(rotation);
    }
}

fn camera_order_color_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera_query: Query<(&Camera, &VisibleEntities)>,
    material_query: Query<&Handle<StandardMaterial>>,
) {
    for (_camera, visible_entities) in &mut camera_query.iter() {
        for visible_entity in visible_entities.iter() {
            if let Ok(material_handle) =
                material_query.get::<Handle<StandardMaterial>>(visible_entity.entity)
            {
                let material = materials.get_mut(&material_handle).unwrap();
                let value = 1.0 - (visible_entity.order.0 - 10.0) / 7.0;
                material.albedo = Color::rgb(value, value, value);
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    commands
        // parent cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: materials.add(StandardMaterial {
                shaded: false,
                ..Default::default()
            }),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|parent| {
            // child cubes
            parent
                .spawn(PbrComponents {
                    mesh: cube_handle,
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    transform: Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)),
                    ..Default::default()
                })
                .spawn(PbrComponents {
                    mesh: cube_handle,
                    material: materials.add(StandardMaterial {
                        shaded: false,
                        ..Default::default()
                    }),
                    transform: Transform::from_translation(Vec3::new(0.0, -3.0, 0.0)),
                    ..Default::default()
                });
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
