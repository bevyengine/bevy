use bevy::ecs::{
    component::{ComponentDescriptor, StorageType},
    entity::Entity,
    world::World,
};
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

struct A(f32);

const RANGE: std::ops::Range<u32> = 5..6;

fn setup(entity_count: u32, storage: StorageType) -> World {
    let mut world = World::default();
    world
        .register_component(ComponentDescriptor::new::<A>(storage))
        .unwrap();
    world.spawn_batch((0..entity_count).map(|_| (A(0.0),)));
    world
}

fn world_entity(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_entity");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        group.bench_function(format!("{}_entities", entity_count), |bencher| {
            let world = setup(entity_count, StorageType::Table);

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
        for storage in [StorageType::Table, StorageType::SparseSet] {
            group.bench_function(
                format!("{}_entities_{:?}", entity_count, storage),
                |bencher| {
                    let world = setup(entity_count, storage);

                    bencher.iter(|| {
                        for i in 0..entity_count {
                            let entity = Entity::new(i);
                            assert!(world.get::<A>(entity).is_some());
                        }
                    });
                },
            );
        }
    }

    group.finish();
}

fn world_query_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_get");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        for storage in [StorageType::Table, StorageType::SparseSet] {
            group.bench_function(
                format!("{}_entities_{:?}", entity_count, storage),
                |bencher| {
                    let mut world = setup(entity_count, storage);
                    let mut query = world.query::<&A>();

                    bencher.iter(|| {
                        for i in 0..entity_count {
                            let entity = Entity::new(i);
                            assert!(query.get(&world, entity).is_ok());
                        }
                    });
                },
            );
        }
    }

    group.finish();
}

fn world_query_iter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        for storage in [StorageType::Table, StorageType::SparseSet] {
            group.bench_function(
                format!("{}_entities_{:?}", entity_count, storage),
                |bencher| {
                    let mut world = setup(entity_count, storage);
                    let mut query = world.query::<&A>();

                    bencher.iter(|| {
                        let mut count = 0;
                        for comp in query.iter(&world) {
                            black_box(comp);
                            count += 1;
                        }
                        assert_eq!(black_box(count), entity_count);
                    });
                },
            );
        }
    }

    group.finish();
}

fn world_query_for_each(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("world_query_for_each");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in RANGE.map(|i| i * 10_000) {
        for storage in [StorageType::Table, StorageType::SparseSet] {
            group.bench_function(
                format!("{}_entities_{:?}", entity_count, storage),
                |bencher| {
                    let mut world = setup(entity_count, storage);
                    let mut query = world.query::<&A>();

                    bencher.iter(|| {
                        let mut count = 0;
                        query.for_each(&world, |comp| {
                            black_box(comp);
                            count += 1;
                        });
                        assert_eq!(black_box(count), entity_count);
                    });
                },
            );
        }
    }

    group.finish();
}
