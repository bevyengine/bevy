use benches::bench;
use bevy_ecs::{component::Component, world::World};
use criterion::Criterion;

const ENTITY_COUNT: usize = 2_000;

#[derive(Component)]
struct C<const N: usize>(usize);

pub fn insert_many(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("insert_many"));

    group.bench_function("all", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world
                    .spawn_empty()
                    .insert(C::<0>(1))
                    .insert(C::<1>(1))
                    .insert(C::<2>(1))
                    .insert(C::<3>(1))
                    .insert(C::<4>(1))
                    .insert(C::<5>(1))
                    .insert(C::<6>(1))
                    .insert(C::<7>(1))
                    .insert(C::<8>(1))
                    .insert(C::<9>(1))
                    .insert(C::<10>(1))
                    .insert(C::<11>(1))
                    .insert(C::<12>(1))
                    .insert(C::<13>(1))
                    .insert(C::<14>(1));
            }
            world.clear_entities();
        });
    });

    group.bench_function("only_last", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world
                    .spawn((
                        C::<0>(1),
                        C::<1>(1),
                        C::<2>(1),
                        C::<3>(1),
                        C::<4>(1),
                        C::<5>(1),
                        C::<6>(1),
                        C::<7>(1),
                        C::<8>(1),
                        C::<9>(1),
                        C::<10>(1),
                        C::<11>(1),
                        C::<12>(1),
                        C::<13>(1),
                    ))
                    .insert(C::<14>(1));
            }
            world.clear_entities();
        });
    });

    group.finish();
}
