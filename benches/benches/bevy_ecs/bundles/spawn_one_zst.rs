use core::hint::black_box;

use benches::bench;
use bevy_ecs::{bundle::Bundle, component::Component, world::World};
use criterion::Criterion;

use super::MakeDynamic;

const ENTITY_COUNT: usize = 10_000;

#[derive(Component)]
struct A;

#[derive(Component)]
struct B;

pub fn spawn_one_zst(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_one_zst"));

    group.bench_function("static", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box(A));
            }
        });
    });

    group.bench_function("option_some", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box(Some(A)));
            }
        });
    });

    group.bench_function("option_none", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box(None::<A>));
            }
        });
    });

    group.bench_function("option_none_and_static", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((A, None::<B>)));
            }
        });
    });

    group.bench_function("box_dyn_bundle", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                // Note: A is a ZST so Box::new is not actually allocating
                // this is mostly measuring the overhead of dynamic bundles
                // and dynamic dispatch.
                world.spawn(black_box(Box::new(A) as Box<dyn Bundle>));
            }
        });
    });

    group.bench_function("dynamic_bundle", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                // This has no other overhead than opting out of the static and bounded caching
                world.spawn(black_box(MakeDynamic(A)));
            }
        });
    });

    group.finish();
}
