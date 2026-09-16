use core::{f32::consts::TAU, hint::black_box, time::Duration};

use benches::bench;
use bevy_ecs::prelude::*;
use bevy_math::ops::{cos, sin};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_transform::{
    components::{GlobalTransform, Transform},
    systems::{
        mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms,
        StaticTransformOptimizations,
    },
};
use criterion::{criterion_group, Criterion};

criterion_group!(
    benches,
    mark_dirty_trees_bench,
    propagate_parent_transforms_bench,
    transform_pipeline_bench
);

const ROOTS: usize = 48;
const FANOUT_PER_LAYER: [usize; 6] = [4, 4, 3, 3, 2, 2];
const HOT_BATCH: usize = 4096;

#[derive(Component)]
#[expect(
    unused,
    reason = "Used to make archetypes closer to real gameplay worlds"
)]
struct MarkerA([f32; 8]);

#[derive(Component)]
#[expect(
    unused,
    reason = "Used to make archetypes closer to real gameplay worlds"
)]
struct MarkerB([f32; 8]);

#[derive(Component)]
#[expect(
    unused,
    reason = "Used to make archetypes closer to real gameplay worlds"
)]
struct MarkerC([f32; 8]);

struct SceneData {
    movers: Vec<Entity>,
    roots: Vec<Entity>,
}

struct WorldBench {
    world: World,
    schedule: Schedule,
    movers: Vec<Entity>,
    roots: Vec<Entity>,
    frame: usize,
}

impl WorldBench {
    fn new(
        static_optimizations: StaticTransformOptimizations,
        configure_schedule: impl FnOnce(&mut Schedule),
    ) -> Self {
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut world = World::new();
        world.insert_resource(static_optimizations);

        let SceneData { movers, roots } = spawn_complex_hierarchy(&mut world);

        // Keep world storage realistic so component scans and table access are not "cold world".
        world.spawn_batch((0..12_000).map(|i| {
            (
                Transform::from_xyz(i as f32 * 0.001, 0.0, 0.0),
                GlobalTransform::IDENTITY,
                MarkerA([i as f32; 8]),
                MarkerB([i as f32 * 0.5; 8]),
                MarkerC([i as f32 * 0.25; 8]),
            )
        }));

        let mut schedule = Schedule::default();
        configure_schedule(&mut schedule);

        // Warm steady-state caches / archetype paths once.
        schedule.run(&mut world);

        Self {
            world,
            schedule,
            movers,
            roots,
            frame: 0,
        }
    }

    fn mutate_movers(&mut self, batch: usize) {
        let len = self.movers.len();
        let start = (self.frame * batch) % len;

        for offset in 0..batch {
            let idx = (start + offset) % len;
            let entity = self.movers[idx];
            if let Some(mut transform) = self.world.get_mut::<Transform>(entity) {
                let phase = (self.frame + offset) as f32 * 0.0005;
                transform.translation.x += sin(phase) * 0.015;
                transform.translation.y += cos(phase) * 0.010;
                transform.rotate_z(0.001 + ((idx % 17) as f32 * 0.000_05));
            }
        }
    }

    fn mutate_roots(&mut self, batch: usize) {
        let len = self.roots.len();
        let start = (self.frame * batch) % len;

        for offset in 0..batch {
            let idx = (start + offset) % len;
            let entity = self.roots[idx];
            if let Some(mut transform) = self.world.get_mut::<Transform>(entity) {
                let phase = (self.frame + offset) as f32 * 0.001;
                transform.translation.z += sin(phase) * 0.02;
                transform.rotate_y(0.0015);
            }
        }
    }

    fn run_once(&mut self) {
        self.schedule.run(&mut self.world);
        self.frame = self.frame.wrapping_add(1);
    }
}

fn spawn_complex_hierarchy(world: &mut World) -> SceneData {
    let mut roots = Vec::with_capacity(ROOTS);
    let mut movers = Vec::new();
    let mut current_layer = Vec::new();
    let mut next_layer = Vec::new();

    for root_idx in 0..ROOTS {
        let root = world
            .spawn((
                Transform::from_xyz(root_idx as f32 * 3.0, 0.0, 0.0),
                GlobalTransform::IDENTITY,
                MarkerA([root_idx as f32; 8]),
                MarkerB([1.0; 8]),
                MarkerC([2.0; 8]),
            ))
            .id();
        roots.push(root);

        current_layer.clear();
        current_layer.push(root);

        for (depth, fanout) in FANOUT_PER_LAYER.into_iter().enumerate() {
            next_layer.clear();
            for &parent in &current_layer {
                for child_idx in 0..fanout {
                    let seed = ((root_idx * 7919) + (depth * 313) + child_idx) as f32;
                    let angle = (seed * 0.11) % TAU;
                    let child = world
                        .spawn((
                            Transform::from_xyz(
                                cos(angle) * (depth as f32 + 1.0),
                                sin(angle) * (depth as f32 + 0.5),
                                depth as f32 * 0.75,
                            ),
                            GlobalTransform::IDENTITY,
                            MarkerA([seed; 8]),
                            MarkerB([seed * 0.5; 8]),
                            MarkerC([seed * 0.25; 8]),
                        ))
                        .id();

                    world.entity_mut(parent).add_child(child);
                    if depth >= 2 {
                        movers.push(child);
                    }
                    next_layer.push(child);
                }
            }
            core::mem::swap(&mut current_layer, &mut next_layer);
        }
    }

    SceneData { movers, roots }
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
                    MarkerA([i as f32; 8]),
                ))
                .id();
            children.push(child);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                propagate_parent_transforms,
                sync_simple_transforms,
            )
                .chain(),
        );

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

fn mark_dirty_trees_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("mark_dirty_trees"));
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(6));

    group.bench_function("localized_leaf_updates", |b| {
        let mut scene = WorldBench::new(StaticTransformOptimizations::Enabled, |schedule| {
            schedule.add_systems(mark_dirty_trees);
        });
        b.iter(|| {
            scene.mutate_movers(1024);
            scene.run_once();
            black_box(scene.frame);
        });
    });

    group.bench_function("distributed_leaf_updates", |b| {
        let mut scene = WorldBench::new(StaticTransformOptimizations::Enabled, |schedule| {
            schedule.add_systems(mark_dirty_trees);
        });
        b.iter(|| {
            scene.mutate_movers(HOT_BATCH);
            scene.run_once();
            black_box(scene.frame);
        });
    });

    group.finish();
}

fn propagate_parent_transforms_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("propagate_parent_transforms"));
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(6));

    // Disable static optimization to isolate pure propagation traversal cost.
    group.bench_function("full_recompute_roots_changed", |b| {
        let mut scene = WorldBench::new(StaticTransformOptimizations::Disabled, |schedule| {
            schedule.add_systems(propagate_parent_transforms);
        });
        b.iter(|| {
            scene.mutate_roots(8);
            scene.run_once();
            black_box(scene.frame);
        });
    });

    group.bench_function("full_recompute_leaves_changed", |b| {
        let mut scene = WorldBench::new(StaticTransformOptimizations::Disabled, |schedule| {
            schedule.add_systems(propagate_parent_transforms);
        });
        b.iter(|| {
            scene.mutate_movers(HOT_BATCH);
            scene.run_once();
            black_box(scene.frame);
        });
    });

    group.finish();
}

fn transform_pipeline_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("transform_pipeline"));
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(6));

    group.bench_function("localized_updates_static_optimized", |b| {
        let mut scene = WorldBench::new(StaticTransformOptimizations::Enabled, |schedule| {
            schedule.add_systems(
                (
                    mark_dirty_trees,
                    propagate_parent_transforms,
                    sync_simple_transforms,
                )
                    .chain(),
            );
        });
        b.iter(|| {
            scene.mutate_movers(1024);
            scene.run_once();
            black_box(scene.frame);
        });
    });

    group.bench_function("localized_updates_no_static_optimization", |b| {
        let mut scene = WorldBench::new(StaticTransformOptimizations::Disabled, |schedule| {
            schedule.add_systems(
                (
                    mark_dirty_trees,
                    propagate_parent_transforms,
                    sync_simple_transforms,
                )
                    .chain(),
            );
        });
        b.iter(|| {
            scene.mutate_movers(1024);
            scene.run_once();
            black_box(scene.frame);
        });
    });

    group.bench_function("childof_reparent_churn", |b| {
        let mut scene = ReparentBench::new();
        b.iter(|| {
            scene.run_churn_frame(4096);
            black_box(scene.frame);
        });
    });

    group.finish();
}
