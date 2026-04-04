use benches::bench;
use bevy_ecs::{component::Component, world::World};
use criterion::Criterion;

#[derive(Component)]
struct A;

#[derive(Component)]
struct B;

#[derive(Component)]
#[constraint(require(A))]
struct C;

#[derive(Component)]
#[constraint(and(require(E), or(require(F), require(G))))]
struct D;

#[derive(Component)]
#[constraint(require(D))]
struct E;

#[derive(Component)]
#[constraint(and(require(E), forbid(G)))]
struct F;

#[derive(Component)]
struct G;

const ENTITY_COUNT: usize = 2_000;

pub fn spawn_no_constraint(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_no_constraint"));

    group.bench_function("static", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world.spawn((A, B));
            }
            world.clear_entities();
        });
    });

    group.finish();
}

pub fn spawn_with_simple_constraint(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_with_simple_constraint"));

    group.bench_function("static", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world.spawn((A, C));
            }
            world.clear_entities();
        });
    });

    group.finish();
}

pub fn spawn_with_complex_constraint(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_with_complex_constraint"));

    group.bench_function("static", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world.spawn((D, E, F));
            }
            world.clear_entities();
        });
    });

    group.finish();
}

#[derive(Component)]
#[constraint(require(G2))]
struct G1;

#[derive(Component)]
#[constraint(require(G3))]
struct G2;

#[derive(Component)]
#[constraint(require(G4))]
struct G3;

#[derive(Component)]
#[constraint(require(G5))]
struct G4;

#[derive(Component)]
#[constraint(require(G6))]
struct G5;

#[derive(Component)]
#[constraint(require(G7))]
struct G6;

#[derive(Component)]
#[constraint(require(G8))]
struct G7;

#[derive(Component)]
#[constraint(require(G9))]
struct G8;

#[derive(Component)]
#[constraint(require(G10))]
struct G9;

#[derive(Component)]
struct G10;

#[derive(Component)]
struct H1;
#[derive(Component)]
struct H2;
#[derive(Component)]
struct H3;
#[derive(Component)]
struct H4;
#[derive(Component)]
struct H5;
#[derive(Component)]
struct H6;
#[derive(Component)]
struct H7;
#[derive(Component)]
struct H8;
#[derive(Component)]
struct H9;
#[derive(Component)]
struct H10;

pub fn spawn_chain_10_no_constraint(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_chain_10_no_constraint"));

    group.bench_function("static", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world.spawn((H1, H2, H3, H4, H5, H6, H7, H8, H9, H10));
            }
            world.clear_entities();
        });
    });

    group.finish();
}

pub fn spawn_chain_10_constraint(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_chain_10_constraint"));

    group.bench_function("static", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world.spawn((G1, G2, G3, G4, G5, G6, G7, G8, G9, G10));
            }
            world.clear_entities();
        });
    });

    group.finish();
}
