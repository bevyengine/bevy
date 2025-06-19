use core::hint::black_box;

use benches::bench;
use bevy_ecs::{bundle::Bundle, component::Component, world::World};
use criterion::Criterion;

use super::MakeDynamic;

const ENTITY_COUNT: usize = 2_000;

#[derive(Component)]
struct C<const N: usize>(usize);

#[derive(Component)]
struct W<T>(T);

pub fn spawn_many(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_many"));

    group.bench_function("static", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    C::<0>(1),
                    C::<1>(1),
                    C::<2>(1),
                    C::<3>(1),
                    C::<4>(1),
                    C::<5>(1),
                    C::<6>(1),
                    C::<7>(1),
                    C::<8>(1),
                    C::<9>(1),
                    C::<10>(1),
                    C::<11>(1),
                    C::<12>(1),
                    C::<13>(1),
                    C::<14>(1),
                )));
            }
        });
    });

    group.bench_function("option_some_many", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box(Some((
                    C::<0>(1),
                    C::<1>(1),
                    C::<2>(1),
                    C::<3>(1),
                    C::<4>(1),
                    C::<5>(1),
                    C::<6>(1),
                    C::<7>(1),
                    C::<8>(1),
                    C::<9>(1),
                    C::<10>(1),
                    C::<11>(1),
                    C::<12>(1),
                    C::<13>(1),
                    C::<14>(1),
                ))));
            }
        });
    });

    group.bench_function("option_one_some", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    C::<0>(1),
                    C::<1>(1),
                    C::<2>(1),
                    C::<3>(1),
                    C::<4>(1),
                    C::<5>(1),
                    C::<6>(1),
                    C::<7>(1),
                    C::<8>(1),
                    C::<9>(1),
                    C::<10>(1),
                    C::<11>(1),
                    C::<12>(1),
                    C::<13>(1),
                    Some(C::<14>(1)),
                )));
            }
        });
    });

    group.bench_function("option_many_some", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    Some(C::<0>(1)),
                    Some(C::<1>(1)),
                    Some(C::<2>(1)),
                    Some(C::<3>(1)),
                    Some(C::<4>(1)),
                    Some(C::<5>(1)),
                    Some(C::<6>(1)),
                    Some(C::<7>(1)),
                    Some(C::<8>(1)),
                    Some(C::<9>(1)),
                    Some(C::<10>(1)),
                    Some(C::<11>(1)),
                    Some(C::<12>(1)),
                    Some(C::<13>(1)),
                    Some(C::<14>(1)),
                )));
            }
        });
    });

    group.bench_function("option_none_many", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box(
                    None::<(
                        C<0>,
                        C<1>,
                        C<2>,
                        C<3>,
                        C<4>,
                        C<5>,
                        C<6>,
                        C<7>,
                        C<8>,
                        C<9>,
                        C<10>,
                        C<11>,
                        C<12>,
                        C<13>,
                        C<14>,
                    )>,
                ));
            }
        });
    });

    group.bench_function("option_one_none", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    C::<0>(1),
                    C::<1>(1),
                    C::<2>(1),
                    C::<3>(1),
                    C::<4>(1),
                    C::<5>(1),
                    C::<6>(1),
                    C::<7>(1),
                    C::<8>(1),
                    C::<9>(1),
                    C::<10>(1),
                    C::<11>(1),
                    C::<12>(1),
                    C::<13>(1),
                    None::<C<14>>,
                )));
            }
        });
    });

    group.bench_function("option_many_none_and_static", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    (C::<0>(1), None::<W<C<0>>>),
                    (C::<1>(1), None::<W<C<1>>>),
                    (C::<2>(1), None::<W<C<2>>>),
                    (C::<3>(1), None::<W<C<3>>>),
                    (C::<4>(1), None::<W<C<4>>>),
                    (C::<5>(1), None::<W<C<5>>>),
                    (C::<6>(1), None::<W<C<6>>>),
                    (C::<7>(1), None::<W<C<7>>>),
                    (C::<8>(1), None::<W<C<8>>>),
                    (C::<9>(1), None::<W<C<9>>>),
                    (C::<10>(1), None::<W<C<10>>>),
                    (C::<11>(1), None::<W<C<11>>>),
                    (C::<12>(1), None::<W<C<12>>>),
                    (C::<13>(1), None::<W<C<13>>>),
                    (C::<14>(1), None::<W<C<14>>>),
                )));
            }
        });
    });

    group.bench_function("many_box_dyn_bundle", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                // Note: C<N> is a ZST so Box::new is not actually allocating
                // this is mostly measuring the overhead of dynamic bundles
                // and dynamic dispatch.
                world.spawn(black_box((
                    Box::new(C::<0>(1)) as Box<dyn Bundle>,
                    Box::new(C::<1>(1)) as Box<dyn Bundle>,
                    Box::new(C::<2>(1)) as Box<dyn Bundle>,
                    Box::new(C::<3>(1)) as Box<dyn Bundle>,
                    Box::new(C::<4>(1)) as Box<dyn Bundle>,
                    Box::new(C::<5>(1)) as Box<dyn Bundle>,
                    Box::new(C::<6>(1)) as Box<dyn Bundle>,
                    Box::new(C::<7>(1)) as Box<dyn Bundle>,
                    Box::new(C::<8>(1)) as Box<dyn Bundle>,
                    Box::new(C::<9>(1)) as Box<dyn Bundle>,
                    Box::new(C::<10>(1)) as Box<dyn Bundle>,
                    Box::new(C::<11>(1)) as Box<dyn Bundle>,
                    Box::new(C::<12>(1)) as Box<dyn Bundle>,
                    Box::new(C::<13>(1)) as Box<dyn Bundle>,
                    Box::new(C::<14>(1)) as Box<dyn Bundle>,
                )));
            }
        });
    });

    group.bench_function("box_dyn_bundle_many", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                // Note: This will also count the cost of allocating a box
                world.spawn(black_box(Box::new((
                    C::<0>(1),
                    C::<1>(1),
                    C::<2>(1),
                    C::<3>(1),
                    C::<4>(1),
                    C::<5>(1),
                    C::<6>(1),
                    C::<7>(1),
                    C::<8>(1),
                    C::<9>(1),
                    C::<10>(1),
                    C::<11>(1),
                    C::<12>(1),
                    C::<13>(1),
                    C::<14>(1),
                )) as Box<dyn Bundle>));
            }
        });
    });

    group.bench_function("dynamic_bundle_many", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                // This has no other overhead than opting out of the static and bounded caching
                // for the whole bundle
                world.spawn(MakeDynamic((
                    C::<0>(1),
                    C::<1>(1),
                    C::<2>(1),
                    C::<3>(1),
                    C::<4>(1),
                    C::<5>(1),
                    C::<6>(1),
                    C::<7>(1),
                    C::<8>(1),
                    C::<9>(1),
                    C::<10>(1),
                    C::<11>(1),
                    C::<12>(1),
                    C::<13>(1),
                    C::<14>(1),
                )));
            }
        });
    });

    group.bench_function("many_dynamic_bundle", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                // This has no other overhead than opting out of the static and bounded caching
                // for each sub-bundle
                world.spawn(black_box((
                    MakeDynamic(C::<0>(1)),
                    MakeDynamic(C::<1>(1)),
                    MakeDynamic(C::<2>(1)),
                    MakeDynamic(C::<3>(1)),
                    MakeDynamic(C::<4>(1)),
                    MakeDynamic(C::<5>(1)),
                    MakeDynamic(C::<6>(1)),
                    MakeDynamic(C::<7>(1)),
                    MakeDynamic(C::<8>(1)),
                    MakeDynamic(C::<9>(1)),
                    MakeDynamic(C::<10>(1)),
                    MakeDynamic(C::<11>(1)),
                    MakeDynamic(C::<12>(1)),
                    MakeDynamic(C::<13>(1)),
                    MakeDynamic(C::<14>(1)),
                )));
            }
        });
    });

    group.finish();
}
