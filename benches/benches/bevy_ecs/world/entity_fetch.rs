use bevy_ecs::{entity::Entity, world::World};
use core::hint::black_box;
use criterion::{BenchmarkId, Criterion};
use std::time::Duration;

pub fn get_entity_mut_slice(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("get_entity_mut_slice");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(4));

    for size in [10, 20, 40, 60, 80, 100, 200, 400, 600, 800, 1000, 2000].iter() {
        group.bench_with_input(BenchmarkId::new("size", size), size, |b, &size| {
            let mut world = World::new();
            let entities: Vec<Entity> = (0..size).map(|_| world.spawn_empty().id()).collect();

            b.iter(|| {
                // This triggers `WorldEntityFetch for &'_ [Entity]`
                let _ = world.get_entity_mut(black_box(entities.as_slice()));
            });
        });
    }
    group.finish();
}
