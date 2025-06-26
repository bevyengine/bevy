use bevy_ecs::prelude::*;
use criterion::{BatchSize, Criterion};
use glam::*;

#[derive(Component)]
struct A(Mat4);
#[derive(Component)]
struct B(Vec4);

pub fn world_despawn(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("despawn_world");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in [1, 100, 10_000] {
        group.bench_function(format!("{}_entities", entity_count), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = World::default();
                    for _ in 0..entity_count {
                        world.spawn((A(Mat4::default()), B(Vec4::default())));
                    }
                    let ents = world.iter_entities().map(|e| e.id()).collect::<Vec<_>>();
                    (world, ents)
                },
                |(world, ents)| {
                    ents.iter().for_each(|e| {
                        world.despawn(*e);
                    });
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
