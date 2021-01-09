use bevy::prelude::*;

/// This example illustrates how to create parent->child relationships between entities how parent transforms
/// are propagated to their descendants
fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .run();
}

/// this component indicates what entities should rotate
struct Rotator;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotator>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(3.0 * time.delta_seconds());
    }
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 2.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.8, 0.7, 0.6),
        ..Default::default()
    });

    commands
        // parent cube
        .spawn(PbrBundle {
            mesh: cube_handle.clone(),
            material: cube_material_handle.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|parent| {
            // child cube
            parent.spawn(PbrBundle {
                mesh: cube_handle,
                material: cube_material_handle,
                transform: Transform::from_xyz(0.0, 0.0, 3.0),
                ..Default::default()
            });
        })
        // light
        .spawn(LightBundle {
            transform: Transform::from_xyz(4.0, 5.0, -4.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(5.0, 10.0, 10.0)
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
