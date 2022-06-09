use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::{Added, Changed},
    world::World,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

criterion_group!(benches, added_detection, changed_detection,);
criterion_main!(benches);

#[derive(Component, Default)]
#[component(storage = "Table")]
struct Table(f32);
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
struct Sparse(f32);

const RANGE: std::ops::Range<u32> = 5..7;

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

fn setup<T: Component + Default>(entity_count: u32) -> World {
    let mut world = World::default();
    world.spawn_batch((0..entity_count).map(|_| (T::default(),)));
    black_box(world)
}

fn added_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("added_detection");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(
            format!("added_{}_entities_table", entity_count),
            |bencher| {
                bencher.iter_batched(
                    || {
                        let mut world = setup::<Table>(entity_count);
                        world
                    },
                    |mut world| {
                        let mut count = 0;
                        let mut query = world.query_filtered::<Entity, Added<Table>>();
                        for entity in query.iter(&world) {
                            black_box(entity);
                            count += 1;
                        }
                        assert_eq!(entity_count, count);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );
        // TODO: Sparse test
        /*
        group.bench_function(format!("{}_entities_sparse", entity_count), |bencher| {
        }); */
    }

    group.finish();
}

fn changed_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("change_detection");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(
            format!("changed_{}_entities_table", entity_count),
            |bencher| {
                bencher.iter_batched(
                    || {
                        let mut world = setup::<Table>(entity_count);
                        world.clear_trackers();
                        let mut query = world.query::<&mut Table>();
                        for mut table in query.iter_mut(&mut world) {
                            black_box(&mut *table);
                        }
                        world
                    },
                    |mut world| {
                        let mut count = 0;
                        let mut query = world.query_filtered::<Entity, Changed<Table>>();
                        for entity in query.iter(&world) {
                            black_box(entity);
                            count += 1;
                        }
                        assert_eq!(entity_count, count);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );
        // TODO: Sparse test
        /*
        group.bench_function(format!("{}_entities_sparse", entity_count), |bencher| {
        }); */
    }

    group.finish();
}
