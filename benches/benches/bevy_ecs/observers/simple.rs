use core::hint::black_box;

use bevy_ecs::{
    event::EntityEvent,
    observer::{On, TriggerTargets},
    world::World,
};

use criterion::Criterion;
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

#[derive(Clone, EntityEvent)]
struct EventBase;

pub fn observe_simple(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    group.bench_function("trigger_simple", |bencher| {
        let mut world = World::new();
        world.add_observer(empty_listener_base);
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(EventBase);
            }
        });
    });

    group.bench_function("trigger_targets_simple/10000_entity", |bencher| {
        let mut world = World::new();
        let mut entities = vec![];
        for _ in 0..10000 {
            entities.push(world.spawn_empty().observe(empty_listener_base).id());
        }
        entities.shuffle(&mut deterministic_rand());
        bencher.iter(|| {
            send_base_event(&mut world, &entities);
        });
    });

    group.finish();
}

fn empty_listener_base(trigger: On<EventBase>) {
    black_box(trigger);
}

fn send_base_event(world: &mut World, entities: impl TriggerTargets) {
    world.trigger_targets(EventBase, entities);
}
