use bevy::prelude::*;

use std::f32::consts::PI;

const FULL_TURN: f32 = 2.0 * PI;

// Define a component to designate a rotation speed to an entity.
#[derive(Component)]
struct Rotatable {
    speed: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotate_cube)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a cube to rotate.
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::WHITE.into()),
            transform: Transform::from_translation(Vec3::ZERO),
            ..Default::default()
        })
        .insert(Rotatable { speed: 0.3 });

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    // Add a light source for better 3d visibility.
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::ONE * 3.0),
        ..Default::default()
    });
}

// This system will rotate any entity in the scene with an assigned Rotatable around its z-axis.
fn rotate_cube(mut cubes: Query<(&mut Transform, &Rotatable)>, timer: Res<Time>) {
    for (mut transform, cube) in cubes.iter_mut() {
        // The speed is taken as a percentage of a full 360 degree turn.
        // The timers delta_seconds is used to smooth out the movement.
        let rotation_change = Quat::from_rotation_y(FULL_TURN * cube.speed * timer.delta_seconds());
        transform.rotate(rotation_change);
    }
}
