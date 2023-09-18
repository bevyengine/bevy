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
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    for amount in 0..21 {
        let mut schedule = Schedule::default();
        schedule.add_systems(empty.run_if(yes));
        for _ in 0..amount {
            schedule.add_systems((empty, empty, empty, empty, empty).distributive_run_if(yes));
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
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
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    for amount in 0..21 {
        let mut schedule = Schedule::default();
        schedule.add_systems(empty.run_if(no));
        for _ in 0..amount {
            schedule.add_systems((empty, empty, empty, empty, empty).distributive_run_if(no));
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
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
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn yes_with_query(query: Query<&TestBool>) -> bool {
        query.single().0
    }
    for amount in 0..21 {
        let mut schedule = Schedule::default();
        schedule.add_systems(empty.run_if(yes_with_query));
        for _ in 0..amount {
            schedule.add_systems(
                (empty, empty, empty, empty, empty).distributive_run_if(yes_with_query),
            );
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
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
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    fn yes_with_resource(res: Res<TestBool>) -> bool {
        res.0
    }
    for amount in 0..21 {
        let mut schedule = Schedule::default();
        schedule.add_systems(empty.run_if(yes_with_resource));
        for _ in 0..amount {
            schedule.add_systems(
                (empty, empty, empty, empty, empty).distributive_run_if(yes_with_resource),
            );
        }
        // run once to initialize systems
        schedule.run(&mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount + 1), |bencher| {
            bencher.iter(|| {
                schedule.run(&mut world);
            });
        });
    }
    group.finish();
}
