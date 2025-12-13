use bevy_ecs::prelude::*;
use criterion::{BatchSize, Criterion};
use glam::*;

#[derive(Component)]
struct A(Mat4);
#[derive(Component)]
struct B(Vec4);

pub fn world_despawn_recursive(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("despawn_world_recursive");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in [1, 100, 10_000] {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = World::default();
                    let parent_ents = (0..entity_count)
                        .map(|_| {
                            world
                                .spawn((A(Mat4::default()), B(Vec4::default())))
                                .with_children(|parent| {
                                    parent.spawn((A(Mat4::default()), B(Vec4::default())));
                                })
                                .id()
                        })
                        .collect::<Vec<_>>();

                    (world, parent_ents)
                },
                |(world, parent_ents)| {
                    parent_ents.iter().for_each(|e| {
                        world.despawn(*e);
                    });
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
