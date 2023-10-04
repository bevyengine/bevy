use bevy_ecs::{
    component::Component,
    prelude::EntityWorldMut,
    schedule::{ExecutorKind, Schedule},
    world::World,
};
use criterion::{BenchmarkId, Criterion};

#[derive(Component)]
struct A<const N: u16>(f32);

fn setup(system_count: usize) -> (World, Schedule) {
    let mut world = World::new();
    fn empty() {}
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    for _ in 0..system_count {
        schedule.add_systems(empty);
    }
    schedule.run(&mut world);
    (world, schedule)
}

fn insert_if_bit_enabled<const B: u16>(entity: &mut EntityWorldMut, i: u16) {
    if i & 1 << B != 0 {
        entity.insert(A::<B>(1.0));
    }
}
/// create `count` entities with distinct archetypes
fn add_archetypes(world: &mut World, count: u16) {
    for i in 0..count {
        let mut e = world.spawn_empty();
        insert_if_bit_enabled::<0>(&mut e, i);
        insert_if_bit_enabled::<1>(&mut e, i);
        insert_if_bit_enabled::<2>(&mut e, i);
        insert_if_bit_enabled::<3>(&mut e, i);
        insert_if_bit_enabled::<4>(&mut e, i);
        insert_if_bit_enabled::<5>(&mut e, i);
        insert_if_bit_enabled::<6>(&mut e, i);
        insert_if_bit_enabled::<7>(&mut e, i);
        insert_if_bit_enabled::<8>(&mut e, i);
        insert_if_bit_enabled::<9>(&mut e, i);
        insert_if_bit_enabled::<10>(&mut e, i);
        insert_if_bit_enabled::<11>(&mut e, i);
        insert_if_bit_enabled::<12>(&mut e, i);
        insert_if_bit_enabled::<13>(&mut e, i);
        insert_if_bit_enabled::<14>(&mut e, i);
        insert_if_bit_enabled::<15>(&mut e, i);
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
