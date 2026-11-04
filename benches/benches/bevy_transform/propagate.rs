use core::{hint::black_box, time::Duration};

use benches::bench;
use bevy_ecs::prelude::*;
use bevy_math::ops::{cos, sin};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_transform::{
    components::{GlobalTransform, Transform},
    systems::{propagate_transforms, sync_simple_transforms, StaticTransformOptimizations},
};
use criterion::{criterion_group, Criterion};

criterion_group!(benches, transform_pipeline_bench);

#[derive(Component)]
struct MarkerA;

#[derive(Component)]
struct MarkerB;

#[derive(Component)]
struct MarkerC;

struct WorldBench {
    world: World,
    schedule: Schedule,
    movers: Vec<Entity>,
    frame: usize,
}

impl WorldBench {
    fn new(
        static_optimizations: StaticTransformOptimizations,
        spawn_entities: impl FnOnce(&mut World) -> Vec<Entity>,
    ) -> Self {
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut world = World::new();
        world.insert_resource(static_optimizations);

        let movers = spawn_entities(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems((propagate_transforms, sync_simple_transforms));

        // Warm steady-state caches / archetype paths once.
        schedule.run(&mut world);

        Self {
            world,
            schedule,
            movers,
            frame: 0,
        }
    }

    fn run_once(&mut self, batch_size: Option<usize>) {
        // Mutate the movers so that the transform propagation has work to do.
        let len = self.movers.len();
        let batch_size = batch_size.unwrap_or(len);
        let start = (self.frame * batch_size) % len;
        for offset in 0..batch_size {
            let idx = (start + offset) % len;
            let entity = self.movers[idx];
            if let Some(mut transform) = self.world.get_mut::<Transform>(entity) {
                let phase = (self.frame + offset) as f32 * 0.0005;
                transform.translation.x += sin(phase) * 0.015;
                transform.translation.y += cos(phase) * 0.010;
                transform.rotate_z(0.001 + ((idx % 17) as f32 * 0.000_05));
            }
        }

        // Run the schedule to benchmark.
        self.schedule.run(&mut self.world);

        // Update the frame number.
        self.frame = self.frame.wrapping_add(1);

        black_box(&mut self.frame);
        black_box(&mut self.world);
    }
}

fn spawn_for_each_fragmented(
    world: &mut World,
    archetype_size: usize,
    mut f: impl FnMut([bool; 3], usize, &mut EntityWorldMut<'_>),
) {
    for a in [true, false] {
        for b in [true, false] {
            for c in [true, false] {
                for i in 0..archetype_size {
                    let mut e = world.spawn(Transform::from_xyz(10.0, 0.0, 0.0));
                    if a {
                        e.insert(MarkerA);
                    }
                    if b {
                        e.insert(MarkerB);
                    }
                    if c {
                        e.insert(MarkerC);
                    }

                    f([a, b, c], i, &mut e);
                }
            }
        }
    }
}

fn spawn_flat_many_moves(world: &mut World) -> Vec<Entity> {
    let mut movers = Vec::new();

    spawn_for_each_fragmented(world, 1000, |_, _, e| {
        // Move all entities.
        movers.push(e.id());
    });

    movers
}

fn spawn_flat_few_moves(world: &mut World) -> Vec<Entity> {
    let mut movers = Vec::new();

    spawn_for_each_fragmented(world, 1000, |[a, _, _], i, e| {
        // Move only a few entities, and not in all archetypes.
        if a && i % 100 == 0 {
            movers.push(e.id());
        }
    });

    movers
}

fn spawn_wide_hierarchy_only_root(world: &mut World) -> Vec<Entity> {
    let root = world.spawn(Transform::from_xyz(10.0, 0.0, 0.0)).id();

    spawn_for_each_fragmented(world, 1000, |_, _, e| _ = e.insert(ChildOf(root)));

    // Only mutate the root
    vec![root]
}

fn spawn_wide_hierarchy_only_few_children(world: &mut World) -> Vec<Entity> {
    let root = world.spawn(Transform::from_xyz(10.0, 0.0, 0.0)).id();
    let mut movers = Vec::new();

    spawn_for_each_fragmented(world, 1000, |[a, _, _], i, e| {
        e.insert(ChildOf(root));

        // Move only a few entities, and not in all archetypes.
        if a && i % 100 == 0 {
            movers.push(e.id());
        }
    });

    movers
}

fn spawn_wide_hierarchy_mixed(world: &mut World) -> Vec<Entity> {
    let root = world.spawn(Transform::from_xyz(10.0, 0.0, 0.0)).id();
    let mut movers = Vec::new();

    // Spawn a lot of movers
    for _ in 0..10000 {
        let e = world.spawn((Transform::from_xyz(10.0, 0.0, 0.0), ChildOf(root), MarkerA));
        movers.push(e.id());
    }

    // Spawn a lot of static entities
    for _ in 0..10000 {
        world.spawn((Transform::from_xyz(10.0, 0.0, 0.0), ChildOf(root), MarkerB));
        world.spawn((Transform::from_xyz(10.0, 0.0, 0.0), ChildOf(root), MarkerC));
    }

    movers
}

struct ReparentBench {
    world: World,
    schedule: Schedule,
    children: Vec<Entity>,
    parent_a: Entity,
    parent_b: Entity,
    frame: usize,
}

impl ReparentBench {
    fn new() -> Self {
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut world = World::new();
        world.insert_resource(StaticTransformOptimizations::Enabled);

        let parent_a = world
            .spawn((
                Transform::from_xyz(-10.0, 0.0, 0.0),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let parent_b = world
            .spawn((
                Transform::from_xyz(10.0, 0.0, 0.0),
                GlobalTransform::IDENTITY,
            ))
            .id();

        let mut children = Vec::with_capacity(16_000);
        for i in 0..16_000 {
            let child = world
                .spawn((
                    Transform::from_xyz((i % 100) as f32 * 0.1, (i / 100) as f32 * 0.1, 0.0),
                    GlobalTransform::IDENTITY,
                    ChildOf(parent_a),
                    MarkerA,
                ))
                .id();
            children.push(child);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems((propagate_transforms, sync_simple_transforms).chain());

        schedule.run(&mut world);

        Self {
            world,
            schedule,
            children,
            parent_a,
            parent_b,
            frame: 0,
        }
    }

    fn run_churn_frame(&mut self, batch: usize) {
        let len = self.children.len();
        let start = (self.frame * batch) % len;
        let target_parent = if self.frame.is_multiple_of(2) {
            self.parent_b
        } else {
            self.parent_a
        };

        for offset in 0..batch {
            let idx = (start + offset) % len;
            self.world
                .entity_mut(self.children[idx])
                .insert(ChildOf(target_parent));
        }

        self.schedule.run(&mut self.world);
        self.frame = self.frame.wrapping_add(1);
    }
}

fn transform_pipeline_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("transform_pipeline"));
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(6));

    for static_optimizations_enabled in [true, false] {
        let id = if static_optimizations_enabled {
            |id: &str| format!("{id}_static_opt")
        } else {
            |id: &str| format!("{id}_no_static_opt")
        };

        let static_optimizations = if static_optimizations_enabled {
            StaticTransformOptimizations::Enabled
        } else {
            StaticTransformOptimizations::Disabled
        };

        group.bench_function(id("spawn_flat_many_moves"), |b| {
            let mut scene = WorldBench::new(static_optimizations, spawn_flat_many_moves);
            b.iter(|| scene.run_once(None));
        });

        group.bench_function(id("spawn_flat_few_moves"), |b| {
            let mut scene = WorldBench::new(static_optimizations, spawn_flat_few_moves);
            b.iter(|| scene.run_once(None));
        });

        group.bench_function(id("spawn_wide_hierarchy_only_root"), |b| {
            let mut scene = WorldBench::new(static_optimizations, spawn_wide_hierarchy_only_root);
            b.iter(|| scene.run_once(None));
        });

        group.bench_function(id("spawn_wide_hierarchy_only_few_children"), |b| {
            let mut scene =
                WorldBench::new(static_optimizations, spawn_wide_hierarchy_only_few_children);
            b.iter(|| scene.run_once(None));
        });

        group.bench_function(id("spawn_wide_hierarchy_mixed"), |b| {
            let mut scene = WorldBench::new(static_optimizations, spawn_wide_hierarchy_mixed);
            b.iter(|| scene.run_once(None));
        });
    }

    group.bench_function("childof_reparent_churn", |b| {
        let mut scene = ReparentBench::new();
        b.iter(|| {
            scene.run_churn_frame(4096);
            black_box(scene.frame);
        });
    });

    group.finish();
}
