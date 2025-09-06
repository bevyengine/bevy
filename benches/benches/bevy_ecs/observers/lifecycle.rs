use bevy_ecs::{component::Component, lifecycle::Insert, observer::On, world::World};
use core::hint::black_box;
use criterion::Criterion;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

pub fn observer_lifecycle(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    group.bench_function("observer_lifecycle_insert", |bencher| {
        let mut world = World::new();
        world.add_observer(on_insert);
        let mut entity = world.spawn(A);
        bencher.iter(|| {
            for _ in 0..10000 {
                entity.insert(A);
            }
        });
    });

    group.finish();
}

#[derive(Component)]
struct A;

fn on_insert(event: On<Insert, A>) {
    black_box(event);
}
