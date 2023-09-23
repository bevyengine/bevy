use bevy_ecs::{component::Component, schedule::Schedule, system::Query, world::World};
use criterion::Criterion;

#[derive(Component)]
struct A(f32);
#[derive(Component)]
struct B(f32);
#[derive(Component)]
struct C(f32);
#[derive(Component)]
struct D(f32);
#[derive(Component)]
struct E(f32);

const ENTITY_BUNCH: usize = 5000;

pub fn empty_systems(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("empty_systems");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    fn empty() {}
    for amount in 0..5 {
        let mut schedule = Schedule::default();
        for _ in 0..amount {
            schedule.add_systems(empty);
        }
        schedule.run(&mut world);
        group.bench_function(&format!("{:03}_systems", amount), |bencher| {
            bencher.iter(|| {
                schedule.run(&mut world);
            });
        });
    }
    for amount in 1..21 {
        let mut schedule = Schedule::default();
        for _ in 0..amount {
            schedule.add_systems((empty, empty, empty, empty, empty));
        }
        schedule.run(&mut world);
        group.bench_function(&format!("{:03}_systems", 5 * amount), |bencher| {
            bencher.iter(|| {
                schedule.run(&mut world);
            });
        });
    }
    group.finish();
}

pub fn busy_systems(criterion: &mut Criterion) {
    fn ab(mut q: Query<(&mut A, &mut B)>) {
        q.for_each_mut(|(mut a, mut b)| {
            std::mem::swap(&mut a.0, &mut b.0);
        });
    }
    fn cd(mut q: Query<(&mut C, &mut D)>) {
        q.for_each_mut(|(mut c, mut d)| {
            std::mem::swap(&mut c.0, &mut d.0);
        });
    }
    fn ce(mut q: Query<(&mut C, &mut E)>) {
        q.for_each_mut(|(mut c, mut e)| {
            std::mem::swap(&mut c.0, &mut e.0);
        });
    }
    let mut world = World::new();
    let mut group = criterion.benchmark_group("busy_systems");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    for entity_bunches in 1..6 {
        world.spawn_batch((0..4 * ENTITY_BUNCH).map(|_| (A(0.0), B(0.0))));
        world.spawn_batch((0..4 * ENTITY_BUNCH).map(|_| (A(0.0), B(0.0), C(0.0))));
        world.spawn_batch((0..ENTITY_BUNCH).map(|_| (A(0.0), B(0.0), C(0.0), D(0.0))));
        world.spawn_batch((0..ENTITY_BUNCH).map(|_| (A(0.0), B(0.0), C(0.0), E(0.0))));
        for system_amount in 0..5 {
            let mut schedule = Schedule::default();
            schedule.add_systems((ab, cd, ce));
            for _ in 0..system_amount {
                schedule.add_systems((ab, cd, ce));
            }
            schedule.run(&mut world);
            group.bench_function(
                &format!(
                    "{:02}x_entities_{:02}_systems",
                    entity_bunches,
                    3 * system_amount + 3
                ),
                |bencher| {
                    bencher.iter(|| {
                        schedule.run(&mut world);
                    });
                },
            );
        }
    }
    group.finish();
}

pub fn contrived(criterion: &mut Criterion) {
    fn s_0(mut q_0: Query<(&mut A, &mut B)>) {
        q_0.for_each_mut(|(mut c_0, mut c_1)| {
            std::mem::swap(&mut c_0.0, &mut c_1.0);
        });
    }
    fn s_1(mut q_0: Query<(&mut A, &mut C)>, mut q_1: Query<(&mut B, &mut D)>) {
        q_0.for_each_mut(|(mut c_0, mut c_1)| {
            std::mem::swap(&mut c_0.0, &mut c_1.0);
        });
        q_1.for_each_mut(|(mut c_0, mut c_1)| {
            std::mem::swap(&mut c_0.0, &mut c_1.0);
        });
    }
    fn s_2(mut q_0: Query<(&mut C, &mut D)>) {
        q_0.for_each_mut(|(mut c_0, mut c_1)| {
            std::mem::swap(&mut c_0.0, &mut c_1.0);
        });
    }
    let mut world = World::new();
    let mut group = criterion.benchmark_group("contrived");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));
    for entity_bunches in 1..6 {
        world.spawn_batch((0..ENTITY_BUNCH).map(|_| (A(0.0), B(0.0), C(0.0), D(0.0))));
        world.spawn_batch((0..ENTITY_BUNCH).map(|_| (A(0.0), B(0.0))));
        world.spawn_batch((0..ENTITY_BUNCH).map(|_| (C(0.0), D(0.0))));
        for system_amount in 0..5 {
            let mut schedule = Schedule::default();
            schedule.add_systems((s_0, s_1, s_2));
            for _ in 0..system_amount {
                schedule.add_systems((s_0, s_1, s_2));
            }
            schedule.run(&mut world);
            group.bench_function(
                &format!(
                    "{:02}x_entities_{:02}_systems",
                    entity_bunches,
                    3 * system_amount + 3
                ),
                |bencher| {
                    bencher.iter(|| {
                        schedule.run(&mut world);
                    });
                },
            );
        }
    }
    group.finish();
}
