//! A minimal example showing how to perform asynchronous work in Bevy
//! using [`AsyncComputeTaskPool`] to run detached tasks, combined with
//! `async_access` to safely access ECS data from async contexts.
//!
//! Instead of using channels to send results back to the main thread,
//! this example performs ECS world mutations directly *inside* async tasks
//! by scheduling closures to run on a chosen schedule (e.g., `Update`).

use bevy::{
    math::ops::{cos, sin},
    prelude::*,
    tasks::AsyncComputeTaskPool,
};
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

/// Spawns a grid of async tasks to simulate delayed cube creation.
///
/// Each task sleeps for a random duration, then uses `async_access`
/// to enqueue a closure that runs on the ECS main thread, allowing
/// mutation of ECS data (e.g., spawning entities and modifying `Local` state).
///
/// No polling, task handles, or channels are needed — async work is detached,
/// and ECS access happens only inside scheduled closures.
fn spawn_tasks(world_id: WorldId) {
    let pool = AsyncComputeTaskPool::get();
    let task_id = EcsTask::new(world_id);
    for x in -NUM_CUBES..NUM_CUBES {
        for z in -NUM_CUBES..NUM_CUBES {
            let task_id = task_id.clone();
            // Spawn a task on the async compute pool
            pool.spawn(async move {
                let delay = Duration::from_secs_f32(rand::rng().random_range(2.0..8.0));
                // Simulate a delay before task completion
                println!("delaying for {:?}", delay);
                Delay::new(delay).await;
                let value = async_access::<
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
                        *local
                    },
                )
                .await;
                if value as i32 == (NUM_CUBES * 2) * (NUM_CUBES * 2) {
                    println!("DONE");
                }
                // Showcasing how you can mutably access variables from outside the closure
                let mut my_thing = String::new();
                async_access::<(), _, _>(world_id, PreUpdate, |()| {
                    my_thing.push('h');
                })
                .await;
                my_thing.push('i');
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
