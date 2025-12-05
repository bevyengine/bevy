use core::hint::black_box;

use bevy_ecs::{
    entity::Entity,
    event::{EntityEvent, Event},
    observer::On,
    world::World,
};

use criterion::Criterion;
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

#[derive(Clone, Event)]
struct A;

pub fn observer_custom(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    group.bench_function("observer_custom", |bencher| {
        let mut world = World::new();
        world.add_observer(on_a);
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(A);
            }
        });
    });

    group.bench_function("observer_custom/10000_entity", |bencher| {
        let mut world = World::new();
        let mut entities = vec![];
        for _ in 0..10000 {
            entities.push(world.spawn_empty().observe(on_b).id());
        }
        entities.shuffle(&mut deterministic_rand());
        bencher.iter(|| {
            for entity in entities.iter().copied() {
                world.trigger(B { entity });
            }
        });
    });

    group.finish();
}

fn on_a(event: On<A>) {
    black_box(event);
}

#[derive(Clone, EntityEvent)]
struct B {
    entity: Entity,
}

fn on_b(event: On<B>) {
    black_box(event);
}
