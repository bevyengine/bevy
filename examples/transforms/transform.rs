//! Shows multiple transformations of objects.

use std::f32::consts::PI;

use bevy::prelude::*;

// A struct for additional data of for a moving cube.
#[derive(Component)]
struct CubeState {
    start_pos: Vec3,
    move_speed: f32,
    turn_speed: f32,
}

// A struct adding information to a scalable entity,
// that will be stationary at the center of the scene.
#[derive(Component)]
struct Center {
    max_size: f32,
    min_size: f32,
    scale_factor: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_cube,
                rotate_cube,
                scale_down_sphere_proportional_to_cube_travel_distance,
            )
                .chain(),
        )
        .run();
}

// Startup system to setup the scene and spawn all relevant entities.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Add an object (sphere) for visualizing scaling.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(
                Mesh::try_from(shape::Icosphere {
                    radius: 3.0,
                    subdivisions: 32,
                })
                .unwrap(),
            ),
            material: materials.add(Color::YELLOW.into()),
            transform: Transform::from_translation(Vec3::ZERO),
            ..default()
        },
        Center {
            max_size: 1.0,
            min_size: 0.1,
            scale_factor: 0.05,
        },
    ));

    // Add the cube to visualize rotation and translation.
    // This cube will circle around the center_sphere
    // by changing its rotation each frame and moving forward.
    // Define a start transform for an orbiting cube, that's away from our central object (sphere)
    // and rotate it so it will be able to move around the sphere and not towards it.
    let cube_spawn =
        Transform::from_translation(Vec3::Z * -10.0).with_rotation(Quat::from_rotation_y(PI / 2.));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::WHITE.into()),
            transform: cube_spawn,
            ..default()
        },
        CubeState {
            start_pos: cube_spawn.translation,
            move_speed: 2.0,
            turn_speed: 0.2,
        },
    ));

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add a light source for better 3d visibility.
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::ONE * 3.0),
        ..default()
    });
}

// This system will move the cube forward.
fn move_cube(mut cubes: Query<(&mut Transform, &mut CubeState)>, timer: Res<Time>) {
    for (mut transform, cube) in &mut cubes {
        // Move the cube forward smoothly at a given move_speed.
        let forward = transform.forward();
        transform.translation += forward * cube.move_speed * timer.delta_seconds();
    }
}

// This system will rotate the cube slightly towards the center_sphere.
// Due to the forward movement the resulting movement
// will be a circular motion around the center_sphere.
fn rotate_cube(
    mut cubes: Query<(&mut Transform, &mut CubeState), Without<Center>>,
    center_spheres: Query<&Transform, With<Center>>,
    timer: Res<Time>,
) {
    // Calculate the point to circle around. (The position of the center_sphere)
    let mut center: Vec3 = Vec3::ZERO;
    for sphere in &center_spheres {
        center += sphere.translation;
    }
    // Update the rotation of the cube(s).
    for (mut transform, cube) in &mut cubes {
        // Calculate the rotation of the cube if it would be looking at the sphere in the center.
        let look_at_sphere = transform.looking_at(center, transform.local_y());
        // Interpolate between the current rotation and the fully turned rotation
        // when looking a the sphere,  with a given turn speed to get a smooth motion.
        // With higher speed the curvature of the orbit would be smaller.
        let incremental_turn_weight = cube.turn_speed * timer.delta_seconds();
        let old_rotation = transform.rotation;
        transform.rotation = old_rotation.lerp(look_at_sphere.rotation, incremental_turn_weight);
    }
}

// This system will scale down the sphere in the center of the scene
// according to the traveling distance of the orbiting cube(s) from their start position(s).
fn scale_down_sphere_proportional_to_cube_travel_distance(
    cubes: Query<(&Transform, &CubeState), Without<Center>>,
    mut centers: Query<(&mut Transform, &Center)>,
) {
    // First we need to calculate the length of between
    // the current position of the orbiting cube and the spawn position.
    let mut distances = 0.0;
    for (cube_transform, cube_state) in &cubes {
        distances += (cube_state.start_pos - cube_transform.translation).length();
    }
    // Now we use the calculated value to scale the sphere in the center accordingly.
    for (mut transform, center) in &mut centers {
        // Calculate the new size from the calculated distances and the centers scale_factor.
        // Since we want to have the sphere at its max_size at the cubes spawn location we start by
        // using the max_size as start value and subtract the distances scaled by a scaling factor.
        let mut new_size: f32 = center.max_size - center.scale_factor * distances;

        // The new size should also not be smaller than the centers min_size.
        // Therefore the max value out of (new_size, center.min_size) is used.
        new_size = new_size.max(center.min_size);

        // Now scale the sphere uniformly in all directions using new_size.
        // Here Vec3:splat is used to create a vector with new_size in x, y and z direction.
        transform.scale = Vec3::splat(new_size);
    }
}
