//! In this example we will simulate a population of entities. In every tick we will:
//! 1. spawn a new entity with a certain possibility
//! 2. age all entities
//! 3. despawn entities with age > 2
//!
//! To demonstrate change detection, there are some console outputs based on changes in
//! the `EntityCounter` resource and updated Age components

use bevy_ecs::prelude::{async_sync_point, Schedule, World};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool, TaskPool, TaskPoolBuilder};

fn main() {
    // Create a world
    let mut world = World::new();
    // Create a schedule
    let mut schedule = Schedule::default();

    // Add systems to increase the counter and to print out the current value
    schedule.add_systems(
        (async_sync_point::<SyncPoint>, || {
            bevy_tasks::tick_global_task_pools_on_main_thread()
        })
            .chain(),
    );
    ComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(3).build());
    IoTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(3).build());
    let world_id = world.id();
    AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(3).build())
        .spawn(async move {
            world_id
                .ecs_task::<()>()
                .run_system(async_sync_point::<SyncPoint>, |_| {})
                .await.unwrap();
        })
        .detach();

    for iteration in 1..=10 {
        println!("Simulating frame {iteration}/10");
        schedule.run(&mut world);
    }
}

struct SyncPoint;
