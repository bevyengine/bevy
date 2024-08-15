use bevy_ecs::{
    component::Component,
    event::Event,
    observer::{EventSet, Observer, Trigger},
    world::World,
};
use criterion::{black_box, measurement::WallTime, BenchmarkGroup, Criterion};

pub fn observe_multievent(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("observe_multievent");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    group.bench_function("trigger_single", |bencher| {
        let mut world = World::new();
        world.observe(empty_listener_set::<TestEvent<1>>);
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(TestEvent::<1>);
            }
        });
    });

    bench_in_set::<1, (TestEvent<1>,)>(&mut group);
    bench_in_set::<2, (TestEvent<1>, TestEvent<2>)>(&mut group);
    bench_in_set::<4, (TestEvent<1>, TestEvent<2>, TestEvent<3>, TestEvent<4>)>(&mut group);
    bench_in_set::<
        8,
        (
            TestEvent<1>,
            TestEvent<2>,
            TestEvent<3>,
            TestEvent<4>,
            TestEvent<5>,
            TestEvent<6>,
            TestEvent<7>,
            TestEvent<8>,
        ),
    >(&mut group);
    bench_in_set::<
        12,
        (
            TestEvent<1>,
            TestEvent<2>,
            TestEvent<3>,
            TestEvent<4>,
            TestEvent<5>,
            TestEvent<6>,
            TestEvent<7>,
            TestEvent<8>,
            TestEvent<9>,
            TestEvent<10>,
            TestEvent<11>,
            TestEvent<12>,
        ),
    >(&mut group);
    bench_in_set::<
        15,
        (
            TestEvent<1>,
            TestEvent<2>,
            TestEvent<3>,
            TestEvent<4>,
            TestEvent<5>,
            TestEvent<6>,
            TestEvent<7>,
            TestEvent<8>,
            TestEvent<9>,
            TestEvent<10>,
            TestEvent<11>,
            TestEvent<12>,
            TestEvent<13>,
            TestEvent<14>,
            TestEvent<15>,
        ),
    >(&mut group);
}

fn bench_in_set<const LAST: usize, Set: EventSet>(group: &mut BenchmarkGroup<WallTime>) {
    group.bench_function(format!("trigger_first/{LAST}"), |bencher| {
        let mut world = World::new();
        world.observe(empty_listener_set::<Set>);
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(TestEvent::<1>);
            }
        });
    });
    group.bench_function(format!("trigger_last/{LAST}"), |bencher| {
        let mut world = World::new();
        world.observe(empty_listener_set::<Set>);
        bencher.iter(|| {
            for _ in 0..10000 {
                world.trigger(TestEvent::<LAST>);
            }
        });
    });
}

#[derive(Event)]
struct TestEvent<const N: usize>;

fn empty_listener_set<Set: EventSet>(trigger: Trigger<Set>) {
    black_box(trigger);
}
