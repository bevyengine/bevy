use core::hint::black_box;

use benches::bench;
use bevy_ecs::{bundle::Bundle, component::Component, world::World};
use criterion::Criterion;

use super::MakeDynamic;

const ENTITY_COUNT: usize = 2_000;

#[derive(Component)]
struct C<const N: usize>;

#[derive(Component)]
struct W<T>(T);

pub fn spawn_many_zst(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_many_zst"));

    group.bench_function("static", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    C::<0>, C::<1>, C::<2>, C::<3>, C::<4>, C::<5>, C::<6>, C::<7>, C::<8>, C::<9>,
                    C::<10>, C::<11>, C::<12>, C::<13>, C::<14>,
                )));
            }
        });
    });

    group.bench_function("option_some_many", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box(Some((
                    C::<0>, C::<1>, C::<2>, C::<3>, C::<4>, C::<5>, C::<6>, C::<7>, C::<8>, C::<9>,
                    C::<10>, C::<11>, C::<12>, C::<13>, C::<14>,
                ))));
            }
        });
    });

    group.bench_function("option_many_some", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    Some(C::<0>),
                    Some(C::<1>),
                    Some(C::<2>),
                    Some(C::<3>),
                    Some(C::<4>),
                    Some(C::<5>),
                    Some(C::<6>),
                    Some(C::<7>),
                    Some(C::<8>),
                    Some(C::<9>),
                    Some(C::<10>),
                    Some(C::<11>),
                    Some(C::<12>),
                    Some(C::<13>),
                    Some(C::<14>),
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

    group.bench_function("option_many_none_and_static", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                world.spawn(black_box((
                    (C::<0>, None::<W<C<0>>>),
                    (C::<1>, None::<W<C<1>>>),
                    (C::<2>, None::<W<C<2>>>),
                    (C::<3>, None::<W<C<3>>>),
                    (C::<4>, None::<W<C<4>>>),
                    (C::<5>, None::<W<C<5>>>),
                    (C::<6>, None::<W<C<6>>>),
                    (C::<7>, None::<W<C<7>>>),
                    (C::<8>, None::<W<C<8>>>),
                    (C::<9>, None::<W<C<9>>>),
                    (C::<10>, None::<W<C<10>>>),
                    (C::<11>, None::<W<C<11>>>),
                    (C::<12>, None::<W<C<12>>>),
                    (C::<13>, None::<W<C<13>>>),
                    (C::<14>, None::<W<C<14>>>),
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
                    Box::new(C::<0>) as Box<dyn Bundle>,
                    Box::new(C::<1>) as Box<dyn Bundle>,
                    Box::new(C::<2>) as Box<dyn Bundle>,
                    Box::new(C::<3>) as Box<dyn Bundle>,
                    Box::new(C::<4>) as Box<dyn Bundle>,
                    Box::new(C::<5>) as Box<dyn Bundle>,
                    Box::new(C::<6>) as Box<dyn Bundle>,
                    Box::new(C::<7>) as Box<dyn Bundle>,
                    Box::new(C::<8>) as Box<dyn Bundle>,
                    Box::new(C::<9>) as Box<dyn Bundle>,
                    Box::new(C::<10>) as Box<dyn Bundle>,
                    Box::new(C::<11>) as Box<dyn Bundle>,
                    Box::new(C::<12>) as Box<dyn Bundle>,
                    Box::new(C::<13>) as Box<dyn Bundle>,
                    Box::new(C::<14>) as Box<dyn Bundle>,
                )));
            }
        });
    });

    group.bench_function("box_dyn_bundle_many", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();
            for _ in 0..ENTITY_COUNT {
                // Note: C<N> is a ZST so Box::new is not actually allocating
                // this is mostly measuring the overhead of dynamic bundles
                // and dynamic dispatch.
                world.spawn(black_box(Box::new((
                    C::<0>, C::<1>, C::<2>, C::<3>, C::<4>, C::<5>, C::<6>, C::<7>, C::<8>, C::<9>,
                    C::<10>, C::<11>, C::<12>, C::<13>, C::<14>,
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
                    C::<0>, C::<1>, C::<2>, C::<3>, C::<4>, C::<5>, C::<6>, C::<7>, C::<8>, C::<9>,
                    C::<10>, C::<11>, C::<12>, C::<13>, C::<14>,
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
                    MakeDynamic(C::<0>),
                    MakeDynamic(C::<1>),
                    MakeDynamic(C::<2>),
                    MakeDynamic(C::<3>),
                    MakeDynamic(C::<4>),
                    MakeDynamic(C::<5>),
                    MakeDynamic(C::<6>),
                    MakeDynamic(C::<7>),
                    MakeDynamic(C::<8>),
                    MakeDynamic(C::<9>),
                    MakeDynamic(C::<10>),
                    MakeDynamic(C::<11>),
                    MakeDynamic(C::<12>),
                    MakeDynamic(C::<13>),
                    MakeDynamic(C::<14>),
                )));
            }
        });
    });

    group.finish();
}
