//! This example shows how to use the ECS and the [`AsyncComputeTaskPool`]
//! to spawn, poll, and complete tasks across systems and system ticks.

use bevy::{
    ecs::system::SystemState,
    ecs::world::CommandQueue,
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use rand::Rng;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_env, add_assets, spawn_tasks))
        .add_systems(Update, handle_tasks)
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
    let box_mesh_handle = meshes.add(Cuboid::new(0.25, 0.25, 0.25));
    commands.insert_resource(BoxMeshHandle(box_mesh_handle));

    let box_material_handle = materials.add(Color::srgb(1.0, 0.2, 0.3));
    commands.insert_resource(BoxMaterialHandle(box_material_handle));
}

#[derive(Component)]
struct ComputeTransform(Task<CommandQueue>);

/// This system generates tasks simulating computationally intensive
/// work that potentially spans multiple frames/ticks. A separate
/// system, [`handle_tasks`], will poll the spawned tasks on subsequent
/// frames/ticks, and use the results to spawn cubes
fn spawn_tasks(mut commands: Commands) {
    let thread_pool = AsyncComputeTaskPool::get();
    for x in 0..NUM_CUBES {
        for y in 0..NUM_CUBES {
            for z in 0..NUM_CUBES {
                // Spawn new task on the AsyncComputeTaskPool; the task will be
                // executed in the background, and the Task future returned by
                // spawn() can be used to poll for the result
                let entity = commands.spawn_empty().id();
                let task = thread_pool.spawn(async move {
                    let duration = Duration::from_secs_f32(rand::thread_rng().gen_range(0.05..5.0));

                    // Pretend this is a time-intensive function. :)
                    async_std::task::sleep(duration).await;

                    // Such hard work, all done!
                    let transform = Transform::from_xyz(x as f32, y as f32, z as f32);
                    let mut command_queue = CommandQueue::default();

                    // we use a raw command queue to pass a FnOne(&mut World) back to be
                    // applied in a deferred manner.
                    command_queue.push(move |world: &mut World| {
                        let (box_mesh_handle, box_material_handle) = {
                            let mut system_state = SystemState::<(
                                Res<BoxMeshHandle>,
                                Res<BoxMaterialHandle>,
                            )>::new(world);
                            let (box_mesh_handle, box_material_handle) =
                                system_state.get_mut(world);

                            (box_mesh_handle.clone(), box_material_handle.clone())
                        };

                        world
                            .entity_mut(entity)
                            // Add our new PbrBundle of components to our tagged entity
                            .insert(PbrBundle {
                                mesh: box_mesh_handle,
                                material: box_material_handle,
                                transform,
                                ..default()
                            })
                            // Task is complete, so remove task component from entity
                            .remove::<ComputeTransform>();
                    });

                    command_queue
                });

                // Spawn new entity and add our new task as a component
                commands.entity(entity).insert(ComputeTransform(task));
            }
        }
    }
}

/// This system queries for entities that have our Task<Transform> component. It polls the
/// tasks to see if they're complete. If the task is complete it takes the result, adds a
/// new [`PbrBundle`] of components to the entity using the result from the task's work, and
/// removes the task component from the entity.
fn handle_tasks(mut commands: Commands, mut transform_tasks: Query<&mut ComputeTransform>) {
    for mut task in &mut transform_tasks {
        if let Some(mut commands_queue) = block_on(future::poll_once(&mut task.0)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);
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
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 12.0, 15.0),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(offset, offset, 15.0)
            .looking_at(Vec3::new(offset, offset, 0.0), Vec3::Y),
        ..default()
    });
}
