use bevy::{prelude::*, render::mesh::shape};

struct Rotator;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .run();
}

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<(&Rotator, &mut Rotation)>) {
    for (_rotator, mut rotation) in &mut query.iter() {
        rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
    }
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    commands
        // parent cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: cube_material_handle,
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|parent| {
            // child cube
            parent.spawn(PbrComponents {
                mesh: cube_handle,
                material: cube_material_handle,
                translation: Translation::new(0.0, 0.0, 3.0),
                ..Default::default()
            });
        })
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, 5.0, -4.0),
            ..Default::default()
        })
        // camera
        .spawn(PerspectiveCameraComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
