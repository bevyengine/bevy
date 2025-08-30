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
struct SimpleEvent;

#[derive(Clone, EntityEvent)]
struct SimpleEntityEvent {
    entity: Entity,
}

pub fn observe_simple(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    group.bench_function("trigger_simple", |bencher| {
        let mut world = World::new();
        world.add_observer(on_simple_event);
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(SimpleEvent);
            }
        });
    });

    group.bench_function("trigger_targets_simple/10000_entity", |bencher| {
        let mut world = World::new();
        let mut entities = vec![];
        for _ in 0..10000 {
            entities.push(world.spawn_empty().observe(on_simple_entity_event).id());
        }
        entities.shuffle(&mut deterministic_rand());
        bencher.iter(|| {
            for entity in entities.iter().copied() {
                world.trigger(SimpleEntityEvent { entity });
            }
        });
    });

    group.finish();
}

fn on_simple_event(event: On<SimpleEvent>) {
    black_box(event);
}

fn on_simple_entity_event(event: On<SimpleEntityEvent>) {
    black_box(event);
}
