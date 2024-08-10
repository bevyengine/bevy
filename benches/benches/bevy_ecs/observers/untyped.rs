use bevy_ecs::{
    event::Event,
    observer::{EventSet, Observer, Trigger, UntypedEvent},
    world::World,
};
use criterion::{black_box, Criterion};

pub fn observe_untyped(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe_untyped");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    group.bench_function("1", |bencher| {
        let mut world = World::new();
        let event_id_1 = world.init_component::<TestEvent<1>>();
        world.spawn(Observer::new(empty_listener_set::<UntypedEvent>).with_event(event_id_1));
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(TestEvent::<1>);
            }
        });
    });
    group.bench_function("2", |bencher| {
        let mut world = World::new();
        let event_id_1 = world.init_component::<TestEvent<1>>();
        let event_id_2 = world.init_component::<TestEvent<2>>();
        world.spawn(
            Observer::new(empty_listener_set::<UntypedEvent>)
                .with_event(event_id_1)
                .with_event(event_id_2),
        );
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(TestEvent::<2>);
            }
        });
    });
}

#[derive(Event)]
struct TestEvent<const N: usize>;

fn empty_listener_set<Set: EventSet>(trigger: Trigger<Set>) {
    black_box(trigger);
}
