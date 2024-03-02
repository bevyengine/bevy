//! Illustrates how to scale an object in each direction.

use std::f32::consts::PI;

use bevy::prelude::*;

// Define a component to keep information for the scaled object.
#[derive(Component)]
struct Scaling {
    scale_direction: Vec3,
    scale_speed: f32,
    max_element_size: f32,
    min_element_size: f32,
}

// Implement a simple initialization.
impl Scaling {
    fn new() -> Self {
        Scaling {
            scale_direction: Vec3::X,
            scale_speed: 2.0,
            max_element_size: 5.0,
            min_element_size: 1.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (change_scale_direction, scale_cube))
        .run();
}

// Startup system to setup the scene and spawn all relevant entities.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a cube to scale.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: materials.add(Color::WHITE),
            transform: Transform::from_rotation(Quat::from_rotation_y(PI / 4.0)),
            ..default()
        },
        Scaling::new(),
    ));

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add a light source for better 3d visibility.
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

// This system will check if a scaled entity went above or below the entities scaling bounds
// and change the direction of the scaling vector.
fn change_scale_direction(mut cubes: Query<(&mut Transform, &mut Scaling)>) {
    for (mut transform, mut cube) in &mut cubes {
        // If an entity scaled beyond the maximum of its size in any dimension
        // the scaling vector is flipped so the scaling is gradually reverted.
        // Additionally, to ensure the condition does not trigger again we floor the elements to
        // their next full value, which should be max_element_size at max.
        if transform.scale.max_element() > cube.max_element_size {
            cube.scale_direction *= -1.0;
            transform.scale = transform.scale.floor();
        }
        // If an entity scaled beyond the minimum of its size in any dimension
        // the scaling vector is also flipped.
        // Additionally the Values are ceiled to be min_element_size at least
        // and the scale direction is flipped.
        // This way the entity will change the dimension in which it is scaled any time it
        // reaches its min_element_size.
        if transform.scale.min_element() < cube.min_element_size {
            cube.scale_direction *= -1.0;
            transform.scale = transform.scale.ceil();
            cube.scale_direction = cube.scale_direction.zxy();
        }
    }
}

// This system will scale any entity with assigned Scaling in each direction
// by cycling through the directions to scale.
fn scale_cube(mut cubes: Query<(&mut Transform, &Scaling)>, timer: Res<Time>) {
    for (mut transform, cube) in &mut cubes {
        transform.scale += cube.scale_direction * cube.scale_speed * timer.delta_seconds();
    }
}
