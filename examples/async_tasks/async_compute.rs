//! This example shows how to use the ECS and the [`AsyncComputeTaskPool`]
//! to spawn, poll, and complete tasks across systems and system ticks.

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use rand::Rng;
use std::time::{Duration, Instant};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_env)
        .add_startup_system(add_assets)
        .add_startup_system(spawn_tasks)
        .add_system(handle_tasks)
        .run();
}

// Number of cubes to spawn across the x, y, and z axis
const NUM_CUBES: u32 = 6;

#[derive(Resource, Deref)]
struct BoxMeshHandle(Handle<Mesh>);

#[derive(Resource, Deref)]
struct BoxMaterialHandle(Handle<StandardMaterial>);

/// Startup system which runs only once and generates our Box Mesh
/// and Box Material assets, adds them to their respective Asset
/// Resources, and stores their handles as resources so we can access
/// them later when we're ready to render our Boxes
fn add_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let box_mesh_handle = meshes.add(Mesh::from(shape::Cube { size: 0.25 }));
    commands.insert_resource(BoxMeshHandle(box_mesh_handle));

    let box_material_handle = materials.add(Color::rgb(1.0, 0.2, 0.3).into());
    commands.insert_resource(BoxMaterialHandle(box_material_handle));
}

#[derive(Component)]
struct ComputeTransform(Task<Transform>);

/// This system generates tasks simulating computationally intensive
/// work that potentially spans multiple frames/ticks. A separate
/// system, `handle_tasks`, will poll the spawned tasks on subsequent
/// frames/ticks, and use the results to spawn cubes
fn spawn_tasks(mut commands: Commands) {
    let thread_pool = AsyncComputeTaskPool::get();
    for x in 0..NUM_CUBES {
        for y in 0..NUM_CUBES {
            for z in 0..NUM_CUBES {
                // Spawn new task on the AsyncComputeTaskPool
                let task = thread_pool.spawn(async move {
                    let mut rng = rand::thread_rng();
                    let start_time = Instant::now();
                    let duration = Duration::from_secs_f32(rng.gen_range(0.05..0.2));
                    while start_time.elapsed() < duration {
                        // Spinning for 'duration', simulating doing hard
                        // compute work generating translation coords!
                    }

                    // Such hard work, all done!
                    Transform::from_xyz(x as f32, y as f32, z as f32)
                });

                // Spawn new entity and add our new task as a component
                commands.spawn().insert(ComputeTransform(task));
            }
        }
    }
}

/// This system queries for entities that have our Task<Transform> component. It polls the
/// tasks to see if they're complete. If the task is complete it takes the result, adds a
/// new [`PbrBundle`] of components to the entity using the result from the task's work, and
/// removes the task component from the entity.
fn handle_tasks(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut ComputeTransform)>,
    box_mesh_handle: Res<BoxMeshHandle>,
    box_material_handle: Res<BoxMaterialHandle>,
) {
    for (entity, mut task) in &mut transform_tasks {
        if let Some(transform) = future::block_on(future::poll_once(&mut task.0)) {
            // Add our new PbrBundle of components to our tagged entity
            commands.entity(entity).insert_bundle(PbrBundle {
                mesh: box_mesh_handle.clone(),
                material: box_material_handle.clone(),
                transform,
                ..default()
            });

            // Task is complete, so remove task component from entity
            commands.entity(entity).remove::<ComputeTransform>();
        }
    }
}

/// This system is only used to setup light and camera for the environment
fn setup_env(mut commands: Commands) {
    // Used to center camera on spawned cubes
    let offset = if NUM_CUBES % 2 == 0 {
        (NUM_CUBES / 2) as f32 - 0.5
    } else {
        (NUM_CUBES / 2) as f32
    };

    // lights
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 12.0, 15.0),
        ..default()
    });

    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(offset, offset, 15.0)
            .looking_at(Vec3::new(offset, offset, 0.0), Vec3::Y),
        ..default()
    });
}
