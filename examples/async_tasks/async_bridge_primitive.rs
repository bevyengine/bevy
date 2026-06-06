//! This example demonstrates how to use Bevy's ECS and the [`AsyncComputeTaskPool`]
//! to offload computationally intensive tasks to a background thread pool and process them
//! asynchronously.
//!
//! Unlike the channel-based approach (where tasks send results directly via a communication
//! channel) or the direct approach in the `async_compute` example, this example uses the
//! ecs<->async bridge.
//!
//! This approach allows for arbitrary ECS mutations throughout the async task, as well as awaiting
//! changes (which allows for ECS mutations to happen concurrently with other async operations).
//! Both the channel-based approach and the direct approach involve bespoke systems to handle task
//! communication (polling the channel, or polling for task completion) and therefore cannot perform
//! arbitrary ECS mutations unless explicitly implemented. These options also cannot await ECS
//! operations.

use bevy::async_bridge::prelude::{async_world_sync_point, AsyncWorld};
use bevy::prelude::*;
use rand::RngExt;

struct MySyncPoint;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (spawn_cube_tasks, setup_scene))
        .add_systems(Update, async_world_sync_point::<MySyncPoint>)
        .run();
}

// Number of cubes to spawn across the x, y, and z axis
const NUM_CUBES: i32 = 6;

/// This system generates tasks simulating computationally intensive
/// work that potentially spans multiple frames/ticks.
///
/// The task is offloaded to the `AsyncComputeTaskPool`, allowing heavy computation
/// to be handled asynchronously, without blocking the main game thread.
fn spawn_cube_tasks(async_world: Res<AsyncWorld>) {
    let pool = bevy::tasks::AsyncComputeTaskPool::get();

    // Create a system state that is shared across all our tasks. SystemParams with local state
    // (e.g., `Local`, `Changed` QueryFilter) will be reused between async tasks that bridge with
    // this state. For example, if you have two tasks reusing a query with `Changed<Transform>`,
    // the second task that runs will only see changes the occurred between the two tasks.
    // Generally, you should reuse the system state for similar tasks. In this example, we only
    // spawn tasks in this system, so we don't need to cache this for reuse later.
    let system_state = async_world.system_state::<(
        Commands,
        Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
    )>();
    for x in -NUM_CUBES..NUM_CUBES {
        for z in -NUM_CUBES..NUM_CUBES {
            // Spawn a task on the async compute pool
            let system_state = system_state.clone();
            pool.spawn(async move {
                let delay = std::time::Duration::from_secs_f32(rand::rng().random_range(2.0..8.0));
                // Simulate a delay before task completion
                futures_timer::Delay::new(delay).await;
                system_state
                    .bridge(
                        MySyncPoint,
                        |(mut commands, mut box_handles, mut meshes, mut materials)| {
                            // The first time this bridge runs it will initialize the box mesh and box material, and then it will reuse them from then on.
                            if box_handles.is_none() {
                                box_handles.replace((
                                    meshes.add(Cuboid::new(0.25, 0.25, 0.25)),
                                    materials.add(Color::srgb(1.0, 0.2, 0.3)),
                                ));
                            }

                            let (box_mesh, box_material) = box_handles.clone().unwrap();

                            commands.spawn((
                                Mesh3d(box_mesh),
                                MeshMaterial3d(box_material),
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

/// Setup a generic scene for our cubes to spawn into.
fn setup_scene(
    mut commands: Commands,
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
        Transform::from_xyz(0.0, 8.0, 4.0),
    ));

    // Spawn a camera looking at the origin
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.5, 5.5, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
