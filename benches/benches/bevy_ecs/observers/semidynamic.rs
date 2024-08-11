use bevy_ecs::{
    event::Event,
    observer::{EmitDynamicTrigger, EventSet, Observer, SemiDynamicEvent, Trigger},
    world::{Command, World},
};
use criterion::{black_box, Criterion};

pub fn observe_semidynamic(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe_semidynamic");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    group.bench_function("static/1s-1d", |bencher| {
        let mut world = World::new();
        let event_id_1 = world.init_component::<Dynamic<1>>();
        world.spawn(
            Observer::new(empty_listener_set::<SemiDynamicEvent<Static<1>>>).with_event(event_id_1),
        );

        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(Static::<1>);
            }
        });
    });
    group.bench_function("dynamic/1s-1d", |bencher| {
        let mut world = World::new();
        let event_id_1 = world.init_component::<Dynamic<1>>();
        world.spawn(
            Observer::new(empty_listener_set::<SemiDynamicEvent<Static<1>>>).with_event(event_id_1),
        );

        bencher.iter(|| {
            for _ in 0..10000 {
                unsafe {
                    EmitDynamicTrigger::new_with_id(event_id_1, Dynamic::<1>, ()).apply(&mut world)
                };
            }
        });
    });

    group.bench_function("static/15s-15d", |bencher| {
        // Aint she perdy?
        let mut world = World::new();
        let event_id_1 = world.init_component::<Dynamic<1>>();
        let event_id_2 = world.init_component::<Dynamic<2>>();
        let event_id_3 = world.init_component::<Dynamic<3>>();
        let event_id_4 = world.init_component::<Dynamic<4>>();
        let event_id_5 = world.init_component::<Dynamic<5>>();
        let event_id_6 = world.init_component::<Dynamic<6>>();
        let event_id_7 = world.init_component::<Dynamic<7>>();
        let event_id_8 = world.init_component::<Dynamic<8>>();
        let event_id_9 = world.init_component::<Dynamic<9>>();
        let event_id_10 = world.init_component::<Dynamic<10>>();
        let event_id_11 = world.init_component::<Dynamic<11>>();
        let event_id_12 = world.init_component::<Dynamic<12>>();
        let event_id_13 = world.init_component::<Dynamic<13>>();
        let event_id_14 = world.init_component::<Dynamic<14>>();
        let event_id_15 = world.init_component::<Dynamic<15>>();
        world.spawn(
            Observer::new(
                empty_listener_set::<
                    SemiDynamicEvent<(
                        Static<1>,
                        Static<2>,
                        Static<3>,
                        Static<4>,
                        Static<5>,
                        Static<6>,
                        Static<7>,
                        Static<8>,
                        Static<9>,
                        Static<10>,
                        Static<11>,
                        Static<12>,
                        Static<13>,
                        Static<14>,
                        Static<15>,
                    )>,
                >,
            )
            .with_event(event_id_1)
            .with_event(event_id_2)
            .with_event(event_id_3)
            .with_event(event_id_4)
            .with_event(event_id_5)
            .with_event(event_id_6)
            .with_event(event_id_7)
            .with_event(event_id_8)
            .with_event(event_id_9)
            .with_event(event_id_10)
            .with_event(event_id_11)
            .with_event(event_id_12)
            .with_event(event_id_13)
            .with_event(event_id_14)
            .with_event(event_id_15),
        );

        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(Static::<14>);
            }
        });
    });
    group.bench_function("dynamic/15s-15d", |bencher| {
        // Aint she perdy?
        let mut world = World::new();
        let event_id_1 = world.init_component::<Dynamic<1>>();
        let event_id_2 = world.init_component::<Dynamic<2>>();
        let event_id_3 = world.init_component::<Dynamic<3>>();
        let event_id_4 = world.init_component::<Dynamic<4>>();
        let event_id_5 = world.init_component::<Dynamic<5>>();
        let event_id_6 = world.init_component::<Dynamic<6>>();
        let event_id_7 = world.init_component::<Dynamic<7>>();
        let event_id_8 = world.init_component::<Dynamic<8>>();
        let event_id_9 = world.init_component::<Dynamic<9>>();
        let event_id_10 = world.init_component::<Dynamic<10>>();
        let event_id_11 = world.init_component::<Dynamic<11>>();
        let event_id_12 = world.init_component::<Dynamic<12>>();
        let event_id_13 = world.init_component::<Dynamic<13>>();
        let event_id_14 = world.init_component::<Dynamic<14>>();
        let event_id_15 = world.init_component::<Dynamic<15>>();
        world.spawn(
            Observer::new(
                empty_listener_set::<
                    SemiDynamicEvent<(
                        Static<1>,
                        Static<2>,
                        Static<3>,
                        Static<4>,
                        Static<5>,
                        Static<6>,
                        Static<7>,
                        Static<8>,
                        Static<9>,
                        Static<10>,
                        Static<11>,
                        Static<12>,
                        Static<13>,
                        Static<14>,
                        Static<15>,
                    )>,
                >,
            )
            .with_event(event_id_1)
            .with_event(event_id_2)
            .with_event(event_id_3)
            .with_event(event_id_4)
            .with_event(event_id_5)
            .with_event(event_id_6)
            .with_event(event_id_7)
            .with_event(event_id_8)
            .with_event(event_id_9)
            .with_event(event_id_10)
            .with_event(event_id_11)
            .with_event(event_id_12)
            .with_event(event_id_13)
            .with_event(event_id_14)
            .with_event(event_id_15),
        );

        bencher.iter(|| {
            for _ in 0..10000 {
                unsafe {
                    EmitDynamicTrigger::new_with_id(event_id_14, Dynamic::<14>, ())
                        .apply(&mut world)
                };
            }
        });
    });
}

/// Static event type
#[derive(Event)]
struct Static<const N: usize>;

/// Dynamic event type
#[derive(Event)]
struct Dynamic<const N: usize>;

fn empty_listener_set<Set: EventSet>(trigger: Trigger<Set>) {
    black_box(trigger);
}
