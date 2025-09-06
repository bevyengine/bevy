use bevy_ecs::prelude::*;
use criterion::Criterion;
use glam::*;

#[derive(Component, PartialEq, Eq, Hash, Clone)]
#[component(immutable, key=Self)]
struct Fragmenting<const N: usize>(u32);

#[derive(Component)]
struct NonFragmenting<const N: usize>(Vec3);

pub fn insert_fragmenting_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_fragmenting_value");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(5));
    group.bench_function("base", |b| {
        b.iter(move || {
            let mut world = World::new();
            world.spawn_batch((0..10_000).map(|_| {
                (
                    Fragmenting::<1>(1),
                    NonFragmenting::<1>(Vec3::ONE),
                    NonFragmenting::<2>(Vec3::ONE),
                    NonFragmenting::<3>(Vec3::ONE),
                )
            }));
        });
    });
    group.bench_function("unbatched", |b| {
        b.iter(move || {
            let mut world = World::new();
            for _ in 0..10_000 {
                world.spawn((
                    Fragmenting::<1>(1),
                    NonFragmenting::<1>(Vec3::ONE),
                    NonFragmenting::<2>(Vec3::ONE),
                    NonFragmenting::<3>(Vec3::ONE),
                ));
            }
        });
    });
    group.bench_function("high_fragmentation_base", |b| {
        b.iter(move || {
            let mut world = World::new();
            world.spawn_batch((0..10_000).map(|i| {
                (
                    Fragmenting::<1>(i % 100),
                    NonFragmenting::<1>(Vec3::ONE),
                    NonFragmenting::<2>(Vec3::ONE),
                    NonFragmenting::<3>(Vec3::ONE),
                )
            }));
        });
    });
    group.bench_function("high_fragmentation_unbatched", |b| {
        b.iter(move || {
            let mut world = World::new();
            for i in 0..10_000 {
                world.spawn((
                    Fragmenting::<1>(i % 100),
                    NonFragmenting::<1>(Vec3::ONE),
                    NonFragmenting::<2>(Vec3::ONE),
                    NonFragmenting::<3>(Vec3::ONE),
                ));
            }
        });
    });
    group.finish();
}

pub fn add_remove_fragmenting_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove_fragmenting_value");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(5));

    group.bench_function("non_fragmenting", |b| {
        let mut world = World::new();
        let entities: Vec<_> = world
            .spawn_batch((0..10_000).map(|_| {
                (
                    Fragmenting::<1>(1),
                    NonFragmenting::<1>(Vec3::ONE),
                    NonFragmenting::<2>(Vec3::ONE),
                    NonFragmenting::<3>(Vec3::ONE),
                )
            }))
            .collect();
        b.iter(move || {
            for entity in &entities {
                world
                    .entity_mut(*entity)
                    .insert(NonFragmenting::<4>(Vec3::ZERO));
            }

            for entity in &entities {
                world.entity_mut(*entity).remove::<NonFragmenting<4>>();
            }
        });
    });

    group.bench_function("fragmenting", |b| {
        let mut world = World::new();
        let entities: Vec<_> = world
            .spawn_batch((0..10_000).map(|_| {
                (
                    Fragmenting::<1>(1),
                    NonFragmenting::<1>(Vec3::ONE),
                    NonFragmenting::<2>(Vec3::ONE),
                    NonFragmenting::<3>(Vec3::ONE),
                )
            }))
            .collect();
        b.iter(move || {
            for entity in &entities {
                world.entity_mut(*entity).insert(Fragmenting::<1>(2));
            }

            for entity in &entities {
                world.entity_mut(*entity).remove::<NonFragmenting<1>>();
            }
        });
    });
}
