//! Transform propagation on a compute pool with no worker threads, in a binary of its own
//! because the pool is process-wide.

use bevy_ecs::{
    hierarchy::ChildOf,
    schedule::{IntoScheduleConfigs, Schedule},
    world::World,
};
use bevy_math::Vec3;
use bevy_tasks::{ComputeTaskPool, TaskPoolBuilder};
use bevy_transform::{
    components::{GlobalTransform, Transform},
    systems::{
        mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms,
        StaticTransformOptimizations,
    },
};

#[test]
fn transforms_propagate_with_no_compute_worker_threads() {
    ComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(0).build());
    let mut world = World::default();
    world.insert_resource(StaticTransformOptimizations::default());
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            mark_dirty_trees,
            sync_simple_transforms,
            propagate_parent_transforms,
        )
            .chain(),
    );
    let parent = world.spawn(Transform::from_xyz(1.0, 0.0, 0.0)).id();
    let child = world
        .spawn((Transform::from_xyz(0.0, 2.0, 0.0), ChildOf(parent)))
        .id();

    schedule.run(&mut world);

    assert_eq!(
        world.get::<GlobalTransform>(child).unwrap().translation(),
        Vec3::new(1.0, 2.0, 0.0)
    );
}
