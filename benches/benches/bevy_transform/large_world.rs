//! Large-world transform pipeline benchmarks.

use core::{hint::black_box, time::Duration};

use benches::bench;
use bevy_ecs::prelude::*;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_transform::{
    components::Transform,
    systems::{
        mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms,
        StaticTransformOptimizations,
    },
};
use criterion::{criterion_group, BenchmarkId, Criterion};

const ENTITIES: usize = 100_000;

#[derive(Clone, Copy)]
enum Scene {
    Flat,
    Mixed,
}

impl Scene {
    const ALL: [Self; 2] = [Self::Flat, Self::Mixed];

    fn name(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Mixed => "mixed",
        }
    }

    fn spawn(self, world: &mut World) {
        match self {
            Self::Flat => spawn_flat_roots(world),
            Self::Mixed => spawn_mixed_world(world),
        }
    }
}

struct PipelineBench {
    world: World,
    movers: QueryState<&'static mut Transform>,
    schedule: Schedule,
    frame: usize,
}

impl PipelineBench {
    fn new(optimizations: StaticTransformOptimizations, scene: Scene) -> Self {
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut world = World::new();
        world.insert_resource(optimizations);
        scene.spawn(&mut world);
        let movers = world.query::<&mut Transform>();

        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                propagate_parent_transforms,
                sync_simple_transforms,
            )
                .chain(),
        );

        Self {
            world,
            movers,
            schedule,
            frame: 0,
        }
    }

    fn run_once(&mut self) {
        for mut transform in self.movers.iter_mut(&mut self.world) {
            transform.translation.x += 0.001;
        }
        self.schedule.run(&mut self.world);
        self.frame = self.frame.wrapping_add(1);
    }
}

// Different component sets, so children land in different archetypes and
// tables like entities in a real scene rather than one uniform archetype. The
// payloads are never read; they exist only to give each archetype a distinct
// table layout.
#[derive(Component)]
struct Lod0;
#[derive(Component)]
struct RenderData(#[expect(dead_code, reason = "payload only affects table layout")] u32);
#[derive(Component)]
struct Interactable(#[expect(dead_code, reason = "payload only affects table layout")] u32);
#[derive(Component)]
struct Collider(#[expect(dead_code, reason = "payload only affects table layout")] f32);

fn spawn_flat_roots(world: &mut World) {
    world.spawn_batch((0..ENTITIES).map(|i| Transform::from_xyz(i as f32 * 0.001, 0.0, 0.0)));
}

fn spawn_tree_child(
    world: &mut World,
    transform: Transform,
    parent: Entity,
    index: usize,
) -> Entity {
    match index % 4 {
        0 => world.spawn((transform, ChildOf(parent))),
        1 => world.spawn((transform, ChildOf(parent), Lod0)),
        2 => world.spawn((transform, ChildOf(parent), RenderData(index as u32))),
        _ => world.spawn((transform, ChildOf(parent), Interactable(index as u32))),
    }
    .id()
}

fn spawn_tree_grandchild(
    world: &mut World,
    transform: Transform,
    parent: Entity,
    index: usize,
) -> Entity {
    match index % 3 {
        0 => world.spawn((transform, ChildOf(parent), Collider(0.5))),
        1 => world.spawn((transform, ChildOf(parent), Lod0, RenderData(index as u32))),
        _ => world.spawn((
            transform,
            ChildOf(parent),
            Interactable(index as u32),
            Collider(0.25),
        )),
    }
    .id()
}

fn spawn_mixed_world(world: &mut World) {
    spawn_flat_roots(world);

    // A field of small trees: roots with children, some grandchildren. This is
    // the shape a game world usually has, not one huge hierarchy.
    for t in 0..128 {
        let root = world
            .spawn(Transform::from_xyz(t as f32 * 0.5, 10.0, 0.0))
            .id();
        let children = 4 + (t % 13);
        for c in 0..children {
            let child = spawn_tree_child(
                world,
                Transform::from_xyz(c as f32 * 0.01, 10.0, 0.0),
                root,
                c,
            );
            if c % 5 == 0 {
                for g in 0..2 {
                    spawn_tree_grandchild(
                        world,
                        Transform::from_xyz(g as f32 * 0.004, 10.0, 0.0),
                        child,
                        c + g,
                    );
                }
            }
        }
    }
}

fn large_world_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("large_world"));
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(6));

    for scene in Scene::ALL {
        for (static_label, static_enabled) in [("static_opt", true), ("no_static_opt", false)] {
            let input = (scene, static_enabled);
            group.bench_with_input(
                BenchmarkId::new(scene.name(), static_label),
                &input,
                |b, &(scene, static_enabled)| {
                    let optimizations = if static_enabled {
                        StaticTransformOptimizations::Enabled
                    } else {
                        StaticTransformOptimizations::Disabled
                    };
                    let mut bench = PipelineBench::new(optimizations, scene);
                    b.iter(|| {
                        bench.run_once();
                        black_box(bench.frame);
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, large_world_bench);
