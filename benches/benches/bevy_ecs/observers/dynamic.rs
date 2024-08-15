use bevy_ecs::{
    event::Event,
    observer::{DynamicEvent, EmitDynamicTrigger, EventSet, Observer, Trigger},
    world::{Command, World},
};
use criterion::{black_box, Criterion};

pub fn observe_dynamic(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe_dynamic");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    group.bench_function("1", |bencher| {
        let mut world = World::new();
        let event_id_1 = world.init_component::<TestEvent<1>>();
        world.spawn(Observer::new(empty_listener_set::<DynamicEvent>).with_event(event_id_1));
        bencher.iter(|| {
            for _ in 0..10000 {
                unsafe {
                    EmitDynamicTrigger::new_with_id(event_id_1, TestEvent::<1>, ())
                        .apply(&mut world)
                };
            }
        });
    });
    group.bench_function("2", |bencher| {
        let mut world = World::new();
        let event_id_1 = world.init_component::<TestEvent<1>>();
        let event_id_2 = world.init_component::<TestEvent<2>>();
        world.spawn(
            Observer::new(empty_listener_set::<DynamicEvent>)
                .with_event(event_id_1)
                .with_event(event_id_2),
        );
        bencher.iter(|| {
            for _ in 0..10000 {
                unsafe {
                    EmitDynamicTrigger::new_with_id(event_id_2, TestEvent::<2>, ())
                        .apply(&mut world)
                };
            }
        });
    });
}

#[derive(Event)]
struct TestEvent<const N: usize>;

fn empty_listener_set<Set: EventSet>(trigger: Trigger<Set>) {
    black_box(trigger);
}
