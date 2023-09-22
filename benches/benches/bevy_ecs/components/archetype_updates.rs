use bevy_ecs::{component::Component, schedule::Schedule, world::World};
use criterion::{BenchmarkId, Criterion};

#[derive(Component)]
struct A<const N: u16>(f32);

fn setup(system_count: usize) -> (World, Schedule) {
    let mut world = World::new();
    fn empty() {}
    let mut schedule = Schedule::default();
    for _ in 0..system_count {
        schedule.add_systems(empty);
    }
    schedule.run(&mut world);
    (world, schedule)
}

/// create `count` entities with distinct archetypes
fn add_archetypes(world: &mut World, count: u16) {
    for i in 0..count {
        let mut e = world.spawn_empty();
        if i & 1 << 0 != 0 {
            e.insert(A::<0>(1.0));
        }
        if i & 1 << 1 != 0 {
            e.insert(A::<1>(1.0));
        }
        if i & 1 << 2 != 0 {
            e.insert(A::<2>(1.0));
        }
        if i & 1 << 3 != 0 {
            e.insert(A::<3>(1.0));
        }
        if i & 1 << 4 != 0 {
            e.insert(A::<4>(1.0));
        }
        if i & 1 << 5 != 0 {
            e.insert(A::<5>(1.0));
        }
        if i & 1 << 6 != 0 {
            e.insert(A::<6>(1.0));
        }
        if i & 1 << 7 != 0 {
            e.insert(A::<7>(1.0));
        }
        if i & 1 << 8 != 0 {
            e.insert(A::<8>(1.0));
        }
        if i & 1 << 9 != 0 {
            e.insert(A::<9>(1.0));
        }
        if i & 1 << 10 != 0 {
            e.insert(A::<10>(1.0));
        }
        if i & 1 << 11 != 0 {
            e.insert(A::<11>(1.0));
        }
        if i & 1 << 12 != 0 {
            e.insert(A::<12>(1.0));
        }
        if i & 1 << 13 != 0 {
            e.insert(A::<13>(1.0));
        }
        if i & 1 << 14 != 0 {
            e.insert(A::<14>(1.0));
        }
        if i & 1 << 15 != 0 {
            e.insert(A::<15>(1.0));
        }
    }
}

pub fn no_archetypes(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("no_archetypes");
    for i in 0..=5 {
        let system_count = i * 20;
        let (mut world, mut schedule) = setup(system_count);
        group.bench_with_input(
            BenchmarkId::new("system_count", system_count),
            &system_count,
            |bencher, &_system_count| {
                bencher.iter(|| {
                    schedule.run(&mut world);
                });
            },
        );
    }
}

pub fn added_archetypes(criterion: &mut Criterion) {
    const SYSTEM_COUNT: usize = 100;
    let mut group = criterion.benchmark_group("added_archetypes");
    for archetype_count in [100, 200, 500, 1000, 2000, 5000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("archetype_count", archetype_count),
            &archetype_count,
            |bencher, &archetype_count| {
                bencher.iter_batched(
                    || {
                        let (mut world, schedule) = setup(SYSTEM_COUNT);
                        add_archetypes(&mut world, archetype_count);
                        (world, schedule)
                    },
                    |(mut world, mut schedule)| {
                        schedule.run(&mut world);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );
    }
}
