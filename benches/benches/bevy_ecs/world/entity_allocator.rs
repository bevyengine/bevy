use core::hint::black_box;

use bevy_ecs::prelude::*;
use criterion::{BatchSize, Criterion};

pub fn entity_allocator_benches(criterion: &mut Criterion) {
    const ENTITY_COUNTS: [u32; 3] = [1, 100, 10_000];

    let mut group = criterion.benchmark_group("entity_allocator_allocate_fresh");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in ENTITY_COUNTS {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            bencher.iter_batched_ref(
                World::default,
                |world| {
                    for _ in 0..entity_count {
                        let entity = world.entity_allocator().alloc();
                        black_box(entity);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();

    let mut group = criterion.benchmark_group("entity_allocator_allocate_fresh_bulk");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in ENTITY_COUNTS {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            bencher.iter_batched_ref(
                World::default,
                |world| {
                    for entity in world.entity_allocator().alloc_many(entity_count) {
                        black_box(entity);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();

    let mut group = criterion.benchmark_group("entity_allocator_free");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in ENTITY_COUNTS {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let world = World::new();
                    let entities =
                        Vec::from_iter(world.entity_allocator().alloc_many(entity_count));
                    (world, entities)
                },
                |(world, entities)| {
                    entities
                        .drain(..)
                        .for_each(|e| world.entity_allocator_mut().free(e));
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();

    let mut group = criterion.benchmark_group("entity_allocator_free_bulk");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in ENTITY_COUNTS {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let world = World::new();
                    let entities =
                        Vec::from_iter(world.entity_allocator().alloc_many(entity_count));
                    (world, entities)
                },
                |(world, entities)| {
                    world.entity_allocator_mut().free_many(&entities);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();

    let mut group = criterion.benchmark_group("entity_allocator_allocate_reused");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in ENTITY_COUNTS {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = World::new();
                    let mut entities =
                        Vec::from_iter(world.entity_allocator().alloc_many(entity_count));
                    world.entity_allocator_mut().free_many(&entities);
                    world
                },
                |world| {
                    for _ in 0..entity_count {
                        let entity = world.entity_allocator().alloc();
                        black_box(entity);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();

    let mut group = criterion.benchmark_group("entity_allocator_allocate_reused_bulk");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in ENTITY_COUNTS {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = World::new();
                    let mut entities =
                        Vec::from_iter(world.entity_allocator().alloc_many(entity_count));
                    world.entity_allocator_mut().free_many(&entities);
                    world
                },
                |world| {
                    for entity in world.entity_allocator().alloc_many(entity_count) {
                        black_box(entity);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
