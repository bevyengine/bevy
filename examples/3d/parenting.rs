//! Illustrates how to create parent-child relationships between entities and how parent transforms
//! are propagated to their descendants.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotator_system)
        .run();
}

/// this component indicates what entities should rotate
#[derive(Component)]
struct Rotator;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotator>>) {
    for mut transform in &mut query {
        transform.rotate_x(3.0 * time.delta_secs());
    }
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.6),
        ..default()
    });

    // parent cube
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material_handle.clone()),
        Transform::from_xyz(0.0, 0.0, 1.0),
        Rotator,
        children![(
            // child cube
            Mesh3d(cube_handle),
            MeshMaterial3d(cube_material_handle),
            Transform::from_xyz(0.0, 0.0, 3.0),
        )],
    ));
    // light
    commands.spawn((PointLight::default(), Transform::from_xyz(4.0, 5.0, -4.0)));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
