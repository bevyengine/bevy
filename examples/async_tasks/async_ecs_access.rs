//! This example demonstrates how to use Bevy's ECS and the [`AsyncComputeTaskPool`]
//! to offload computationally intensive tasks to a background thread pool and process them
//! asynchronously.
//!
//! Unlike the channel-based approach (where tasks send results directly via a communication
//! channel) or the direct approach in async_compute, this example uses the ecs <-> async bridge.

use bevy::async_bridge::prelude::{drive_async_bridge, AsyncBridge};
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use rand::RngExt;

struct MySyncPoint;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (drive_async_bridge::<MySyncPoint>, rotate_light))
        .run();
}

// Number of cubes to spawn across the x, y, and z axis
const NUM_CUBES: i32 = 6;

const LIGHT_RADIUS: f32 = 8.0;

/// This system generates tasks simulating computationally intensive
/// work that potentially spans multiple frames/ticks. A separate
/// system, [`handle_tasks`], will track the spawned tasks on subsequent
/// frames/ticks, and use the results to spawn cubes.
///
/// The task is offloaded to the `AsyncComputeTaskPool`, allowing heavy computation
/// to be handled asynchronously, without blocking the main game thread.
fn setup(
    mut commands: Commands,
    bridge: Res<AsyncBridge>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(1.618 * NUM_CUBES as f32))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // Spawn a point light with shadows enabled
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, LIGHT_RADIUS, 4.0),
    ));

    // Spawn a camera looking at the origin
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.5, 5.5, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let pool = bevy::tasks::AsyncComputeTaskPool::get();

    // Reuse tasks so you don't have to pay the system init cost every time it runs.
    let task = bridge.new::<(
        Commands,
        Local<Option<Handle<Mesh>>>,
        Local<Option<Handle<StandardMaterial>>>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
    )>();
    for x in -NUM_CUBES..NUM_CUBES {
        for z in -NUM_CUBES..NUM_CUBES {
            // Spawn a task on the async compute pool
            let task = task.clone();
            pool.spawn(async move {
                let delay = std::time::Duration::from_secs_f32(rand::rng().random_range(2.0..8.0));
                // Simulate a delay before task completion
                futures_timer::Delay::new(delay).await;
                task.access(
                    MySyncPoint,
                    |(mut commands, mut box_mesh, mut box_material, mut meshes, mut materials)| {
                        if box_mesh.is_none() {
                            box_mesh.replace(meshes.add(Cuboid::new(0.25, 0.25, 0.25)));
                        }
                        if box_material.is_none() {
                            box_material.replace(materials.add(Color::srgb(1.0, 0.2, 0.3)));
                        }
                        let (box_mesh, box_material) =
                            (box_mesh.as_ref().unwrap(), box_material.as_ref().unwrap());
                        commands.spawn((
                            Mesh3d(box_mesh.clone()),
                            MeshMaterial3d(box_material.clone()),
                            Transform::from_xyz(x as f32, 0.5, z as f32),
                        ));
                    },
                )
                .await
                .unwrap();
            })
            .detach();
        }
    }
}

/// Rotates the point light around the origin (0, 0, 0)
fn rotate_light(mut query: Query<&mut Transform, With<PointLight>>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        let angle = 1.618 * time.elapsed_secs();
        let x = LIGHT_RADIUS * ops::cos(angle);
        let z = LIGHT_RADIUS * ops::sin(angle);

        // Update the light's position to rotate around the origin
        transform.translation = Vec3::new(x, LIGHT_RADIUS, z);
    }
}
