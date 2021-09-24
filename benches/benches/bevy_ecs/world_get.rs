use bevy::ecs::{component::Component, entity::Entity, world::World};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

criterion_group!(
    benches,
    world_entity,
    world_get,
    world_query_get,
    world_query_iter,
    world_query_for_each,
);
criterion_main!(benches);

#[derive(Component, Default)]
#[component(storage = "Table")]
struct Table(f32);
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
struct Sparse(f32);

const RANGE: std::ops::Range<u32> = 5..6;

fn setup<T: Component + Default>(entity_count: u32) -> World {
    let mut world = World::default();
    world.spawn_batch((0..entity_count).map(|_| (T::default(),)));
    world
}

fn world_entity(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_entity");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{}_entities", entity_count), |bencher| {
            let world = setup::<Table>(entity_count);

            bencher.iter(|| {
                for i in 0..entity_count {
                    let entity = Entity::new(i);
                    black_box(world.entity(entity));
                }
            });
        });
    }

    group.finish();
}

fn world_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_get");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{}_entities_table", entity_count), |bencher| {
            let world = setup::<Table>(entity_count);

            bencher.iter(|| {
                for i in 0..entity_count {
                    let entity = Entity::new(i);
                    assert!(world.get::<Table>(entity).is_some());
                }
            });
        });
        group.bench_function(format!("{}_entities_sparse", entity_count), |bencher| {
            let world = setup::<Sparse>(entity_count);

            bencher.iter(|| {
                for i in 0..entity_count {
                    let entity = Entity::new(i);
                    assert!(world.get::<Sparse>(entity).is_some());
                }
            });
        });
    }

    group.finish();
}

fn world_query_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_get");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{}_entities_table", entity_count), |bencher| {
            let mut world = setup::<Table>(entity_count);
            let mut query = world.query::<&Table>();

            bencher.iter(|| {
                for i in 0..entity_count {
                    let entity = Entity::new(i);
                    assert!(query.get(&world, entity).is_ok());
                }
            });
        });
        group.bench_function(format!("{}_entities_sparse", entity_count), |bencher| {
            let mut world = setup::<Sparse>(entity_count);
            let mut query = world.query::<&Sparse>();

            bencher.iter(|| {
                for i in 0..entity_count {
                    let entity = Entity::new(i);
                    assert!(query.get(&world, entity).is_ok());
                }
            });
        });
    }

    group.finish();
}

fn world_query_iter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{}_entities_table", entity_count), |bencher| {
            let mut world = setup::<Table>(entity_count);
            let mut query = world.query::<&Table>();

            bencher.iter(|| {
                let mut count = 0;
                for comp in query.iter(&world) {
                    black_box(comp);
                    count += 1;
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
        group.bench_function(format!("{}_entities_sparse", entity_count), |bencher| {
            let mut world = setup::<Sparse>(entity_count);
            let mut query = world.query::<&Sparse>();

            bencher.iter(|| {
                let mut count = 0;
                for comp in query.iter(&world) {
                    black_box(comp);
                    count += 1;
                }
                assert_eq!(black_box(count), entity_count);
            });
        });
    }

    group.finish();
}

fn world_query_for_each(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_for_each");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{}_entities_table", entity_count), |bencher| {
            let mut world = setup::<Table>(entity_count);
            let mut query = world.query::<&Table>();

            bencher.iter(|| {
                let mut count = 0;
                query.for_each(&world, |comp| {
                    black_box(comp);
                    count += 1;
                });
                assert_eq!(black_box(count), entity_count);
            });
        });
        group.bench_function(format!("{}_entities_sparse", entity_count), |bencher| {
            let mut world = setup::<Sparse>(entity_count);
            let mut query = world.query::<&Sparse>();

            bencher.iter(|| {
                let mut count = 0;
                query.for_each(&world, |comp| {
                    black_box(comp);
                    count += 1;
                });
                assert_eq!(black_box(count), entity_count);
            });
        });
    }

    group.finish();
}
