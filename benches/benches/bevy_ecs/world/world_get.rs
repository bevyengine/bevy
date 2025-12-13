use core::hint::black_box;

use bevy_ecs::{
    bundle::{Bundle, NoBundleEffect},
    component::Component,
    entity::Entity,
    system::{Query, SystemState},
    world::World,
};
use criterion::Criterion;
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Component, Default)]
#[component(storage = "Table")]
struct Table(f32);
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
struct Sparse(f32);
#[derive(Component, Default)]
#[component(storage = "Table")]
struct WideTable<const X: usize>(f32);
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
struct WideSparse<const X: usize>(f32);

const RANGE: core::ops::Range<u32> = 5..6;

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

fn setup<T: Component + Default>(entity_count: u32) -> (World, Vec<Entity>) {
    let mut world = World::default();
    let entities: Vec<Entity> = world
        .spawn_batch((0..entity_count).map(|_| T::default()))
        .collect();
    black_box((world, entities))
}

fn setup_wide<T: Bundle<Effect: NoBundleEffect> + Default>(
    entity_count: u32,
) -> (World, Vec<Entity>) {
    let mut world = World::default();
    let entities: Vec<Entity> = world
        .spawn_batch((0..entity_count).map(|_| T::default()))
        .collect();
    black_box((world, entities))
}

pub fn world_entity(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_entity");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            let (world, entities) = setup::<Table>(entity_count);

            bencher.iter(|| {
                for entity in &entities {
                    black_box(world.entity(*entity));
                }
            });
        });
    }

    group.finish();
}

pub fn world_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_get");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{entity_count}_entities_table"), |bencher| {
            let (world, entities) = setup::<Table>(entity_count);

            bencher.iter(|| {
                for entity in &entities {
                    assert!(world.get::<Table>(*entity).is_some());
                }
            });
        });
        group.bench_function(format!("{entity_count}_entities_sparse"), |bencher| {
            let (world, entities) = setup::<Sparse>(entity_count);

            bencher.iter(|| {
                for entity in &entities {
                    assert!(world.get::<Sparse>(*entity).is_some());
                }
            });
        });
    }

    group.finish();
}

pub fn world_query_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_get");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{entity_count}_entities_table"), |bencher| {
            let (mut world, entities) = setup::<Table>(entity_count);
            let mut query = world.query::<&Table>();

            bencher.iter(|| {
                for entity in &entities {
                    assert!(query.get(&world, *entity).is_ok());
                }
            });
        });
        group.bench_function(format!("{entity_count}_entities_table_wide"), |bencher| {
            let (mut world, entities) = setup_wide::<(
                WideTable<0>,
                WideTable<1>,
                WideTable<2>,
                WideTable<3>,
                WideTable<4>,
                WideTable<5>,
            )>(entity_count);
            let mut query = world.query::<(
                &WideTable<0>,
                &WideTable<1>,
                &WideTable<2>,
                &WideTable<3>,
                &WideTable<4>,
                &WideTable<5>,
            )>();

            bencher.iter(|| {
                for entity in &entities {
                    assert!(query.get(&world, *entity).is_ok());
                }
            });
        });
        group.bench_function(format!("{entity_count}_entities_sparse"), |bencher| {
            let (mut world, entities) = setup::<Sparse>(entity_count);
            let mut query = world.query::<&Sparse>();

            bencher.iter(|| {
                for entity in &entities {
                    assert!(query.get(&world, *entity).is_ok());
                }
            });
        });
        group.bench_function(format!("{entity_count}_entities_sparse_wide"), |bencher| {
            let (mut world, entities) = setup_wide::<(
                WideSparse<0>,
                WideSparse<1>,
                WideSparse<2>,
                WideSparse<3>,
                WideSparse<4>,
                WideSparse<5>,
            )>(entity_count);
            let mut query = world.query::<(
                &WideSparse<0>,
                &WideSparse<1>,
                &WideSparse<2>,
                &WideSparse<3>,
                &WideSparse<4>,
                &WideSparse<5>,
            )>();

            bencher.iter(|| {
                for entity in &entities {
                    assert!(query.get(&world, *entity).is_ok());
                }
            });
        });
    }

    group.finish();
}

pub fn world_query_iter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_iter");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{entity_count}_entities_table"), |bencher| {
            let (mut world, _) = setup::<Table>(entity_count);
            let mut query = world.query::<&Table>();

            bencher.iter(|| {
                let mut count = 0;
                for comp in query.iter(&world) {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
        group.bench_function(format!("{entity_count}_entities_sparse"), |bencher| {
            let (mut world, _) = setup::<Sparse>(entity_count);
            let mut query = world.query::<&Sparse>();

            bencher.iter(|| {
                let mut count = 0;
                for comp in query.iter(&world) {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
    }

    group.finish();
}

pub fn world_query_for_each(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_for_each");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{entity_count}_entities_table"), |bencher| {
            let (mut world, _) = setup::<Table>(entity_count);
            let mut query = world.query::<&Table>();

            bencher.iter(|| {
                let mut count = 0;
                query.iter(&world).for_each(|comp| {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                });
                assert_eq!(black_box(count), entity_count);
            });
        });
        group.bench_function(format!("{entity_count}_entities_sparse"), |bencher| {
            let (mut world, _) = setup::<Sparse>(entity_count);
            let mut query = world.query::<&Sparse>();

            bencher.iter(|| {
                let mut count = 0;
                query.iter(&world).for_each(|comp| {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                });
                assert_eq!(black_box(count), entity_count);
            });
        });
    }

    group.finish();
}

pub fn query_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("query_get");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{entity_count}_entities_table"), |bencher| {
            let mut world = World::default();
            let mut entities: Vec<_> = world
                .spawn_batch((0..entity_count).map(|_| Table::default()))
                .collect();
            entities.shuffle(&mut deterministic_rand());
            let mut query = SystemState::<Query<&Table>>::new(&mut world);
            let query = query.get(&world);

            bencher.iter(|| {
                let mut count = 0;
                for comp in entities.iter().flat_map(|&e| query.get(e)) {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
        group.bench_function(format!("{entity_count}_entities_sparse"), |bencher| {
            let mut world = World::default();
            let mut entities: Vec<_> = world
                .spawn_batch((0..entity_count).map(|_| Sparse::default()))
                .collect();
            entities.shuffle(&mut deterministic_rand());
            let mut query = SystemState::<Query<&Sparse>>::new(&mut world);
            let query = query.get(&world);

            bencher.iter(|| {
                let mut count = 0;
                for comp in entities.iter().flat_map(|&e| query.get(e)) {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
    }

    group.finish();
}

pub fn query_get_many<const N: usize>(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(format!("query_get_many_{N}"));
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(2 * N as u64));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{entity_count}_calls_table"), |bencher| {
            let mut world = World::default();
            let mut entity_groups: Vec<_> = (0..entity_count)
                .map(|_| [(); N].map(|_| world.spawn(Table::default()).id()))
                .collect();
            entity_groups.shuffle(&mut deterministic_rand());

            let mut query = SystemState::<Query<&Table>>::new(&mut world);
            let query = query.get(&world);

            bencher.iter(|| {
                let mut count = 0;
                for comp in entity_groups
                    .iter()
                    .filter_map(|&ids| query.get_many(ids).ok())
                {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
        group.bench_function(format!("{entity_count}_calls_sparse"), |bencher| {
            let mut world = World::default();
            let mut entity_groups: Vec<_> = (0..entity_count)
                .map(|_| [(); N].map(|_| world.spawn(Sparse::default()).id()))
                .collect();
            entity_groups.shuffle(&mut deterministic_rand());

            let mut query = SystemState::<Query<&Sparse>>::new(&mut world);
            let query = query.get(&world);

            bencher.iter(|| {
                let mut count = 0;
                for comp in entity_groups
                    .iter()
                    .filter_map(|&ids| query.get_many(ids).ok())
                {
                    black_box(comp);
                    count += 1;
                    black_box(count);
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
    }
}
