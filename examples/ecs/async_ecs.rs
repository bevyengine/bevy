//! A minimal example showing how to perform asynchronous work in Bevy
//! using [`AsyncComputeTaskPool`] for parallel task execution and a crossbeam channel
//! to communicate between async tasks and the main ECS thread.
//!
//! This example demonstrates how to spawn detached async tasks, send completion messages via channels,
//! and dynamically spawn ECS entities (cubes) as results from these tasks. The system processes
//! async task results in the main game loop, all without blocking or polling the main thread.

use bevy::{
    math::ops::{cos, sin},
    prelude::*,
    tasks::AsyncComputeTaskPool,
};
use bevy_ecs::schedule::r#async::TaskIdentifier;
use futures_timer::Delay;
use rand::Rng;
use std::time::Duration;

const NUM_CUBES: i32 = 16;
const LIGHT_RADIUS: f32 = 8.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (setup_env, setup_assets, spawn_tasks.after(setup_assets)),
        )
        .add_systems(Update, rotate_light)
        .run();
}

/// Spawns async tasks on the compute task pool to simulate delayed cube creation.
///
/// Each task is executed on a separate thread and sends the result (cube position)
/// back through the `CubeChannel` once completed. The tasks are detached to
/// run asynchronously without blocking the main thread.
///
/// In this example, we don't implement task tracking or proper error handling.
fn spawn_tasks(world_id: WorldId) {
    let pool = AsyncComputeTaskPool::get();
    let task_id = TaskIdentifier::new(world_id);
    for x in -NUM_CUBES..NUM_CUBES {
        for z in -NUM_CUBES..NUM_CUBES {
            // Spawn a task on the async compute pool
            pool.spawn(async move {
                let delay = Duration::from_secs_f32(rand::rng().random_range(2.0..8.0));
                // Simulate a delay before task completion
                println!("delaying for {:?}", delay);
                Delay::new(delay).await;
                if let Err(e) = async_access::<
                    (
                        Local<u32>,
                        Commands,
                        Res<BoxMeshHandle>,
                        Res<BoxMaterialHandle>,
                    ),
                    _,
                    _,
                >(
                    task_id,
                    Update,
                    |(mut local, mut commands, box_mesh, box_material)| {
                        *local += 1;
                        println!("spawning {}", *local);
                        commands.spawn((
                            Mesh3d(box_mesh.clone()),
                            MeshMaterial3d(box_material.clone()),
                            Transform::from_xyz(x as f32, 0.5, z as f32),
                        ));
                    },
                )
                .await
                {
                    println!("got error: {}", e);
                }
            })
            .detach();
        }
    }
}

/// Resource holding the mesh handle for the box (used for spawning cubes)
#[derive(Resource, Deref)]
struct BoxMeshHandle(Handle<Mesh>);

/// Resource holding the material handle for the box (used for spawning cubes)
#[derive(Resource, Deref)]
struct BoxMaterialHandle(Handle<StandardMaterial>);

/// Sets up the shared mesh and material for the cubes.
fn setup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create and store a cube mesh
    let box_mesh_handle = meshes.add(Cuboid::new(0.4, 0.4, 0.4));
    commands.insert_resource(BoxMeshHandle(box_mesh_handle));

    // Create and store a red material
    let box_material_handle = materials.add(Color::srgb(1.0, 0.2, 0.3));
    commands.insert_resource(BoxMaterialHandle(box_material_handle));
}

/// Sets up the environment by spawning the ground, light, and camera.
fn setup_env(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a circular ground plane
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(1.618 * NUM_CUBES as f32))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // Spawn a point light with shadows enabled
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, LIGHT_RADIUS, 4.0),
    ));

    // Spawn a camera looking at the origin
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.5, 5.5, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Rotates the point light around the origin (0, 0, 0)
fn rotate_light(mut query: Query<&mut Transform, With<PointLight>>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        let angle = 1.618 * time.elapsed_secs();
        let x = LIGHT_RADIUS * cos(angle);
        let z = LIGHT_RADIUS * sin(angle);

        // Update the light's position to rotate around the origin
        transform.translation = Vec3::new(x, LIGHT_RADIUS, z);
    }
}
