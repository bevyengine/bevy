use bevy_ecs::prelude::*;
use criterion::Criterion;

/// A run `Condition` that always returns true
fn yes() -> bool {
    true
}

/// A run `Condition` that always returns false
fn no() -> bool {
    false
}

pub fn run_condition_yes(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("run_condition/yes");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(3));
    fn empty() {}
    for amount in [10, 100, 1_000] {
        let mut schedule = Schedule::default();
        for _ in 0..(amount / 5) {
            schedule.add_systems((empty, empty, empty, empty, empty).distributive_run_if(yes));
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(format!("{amount}_systems"), |bencher| {
            bencher.iter(|| {
                schedule.run(&mut world);
            });
        });
    }
    group.finish();
}

pub fn run_condition_no(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("run_condition/no");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(3));
    fn empty() {}
    for amount in [10, 100, 1_000] {
        let mut schedule = Schedule::default();
        for _ in 0..(amount / 5) {
            schedule.add_systems((empty, empty, empty, empty, empty).distributive_run_if(no));
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(format!("{amount}_systems"), |bencher| {
            bencher.iter(|| {
                schedule.run(&mut world);
            });
        });
    }
    group.finish();
}

#[derive(Component, Resource)]
struct TestBool(pub bool);

pub fn run_condition_yes_with_query(criterion: &mut Criterion) {
    let mut world = World::new();
    world.spawn(TestBool(true));
    let mut group = criterion.benchmark_group("run_condition/yes_using_query");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(3));
    fn empty() {}
    fn yes_with_query(query: Single<&TestBool>) -> bool {
        query.0
    }
    for amount in [10, 100, 1_000] {
        let mut schedule = Schedule::default();
        for _ in 0..(amount / 5) {
            schedule.add_systems(
                (empty, empty, empty, empty, empty).distributive_run_if(yes_with_query),
            );
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(format!("{amount}_systems"), |bencher| {
            bencher.iter(|| {
                schedule.run(&mut world);
            });
        });
    }
    group.finish();
}

pub fn run_condition_yes_with_resource(criterion: &mut Criterion) {
    let mut world = World::new();
    world.insert_resource(TestBool(true));
    let mut group = criterion.benchmark_group("run_condition/yes_using_resource");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(3));
    fn empty() {}
    fn yes_with_resource(res: Res<TestBool>) -> bool {
        res.0
    }
    for amount in [10, 100, 1_000] {
        let mut schedule = Schedule::default();
        for _ in 0..(amount / 5) {
            schedule.add_systems(
                (empty, empty, empty, empty, empty).distributive_run_if(yes_with_resource),
            );
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(format!("{amount}_systems"), |bencher| {
            bencher.iter(|| {
                schedule.run(&mut world);
            });
        });
    }
    group.finish();
}
