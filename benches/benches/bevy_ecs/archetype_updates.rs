use bevy_ecs::{
    component::Component,
    schedule::{Stage, SystemStage},
    world::World,
};
use criterion::{criterion_group, criterion_main, Criterion};

criterion_group!(benches, no_archetypes, added_archetypes);
criterion_main!(benches);

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

const SYSTEMS: usize = 50;

fn setup() -> (World, SystemStage) {
    let mut world = World::new();
    fn empty() {}
    let mut stage = SystemStage::parallel();
    for _ in 0..SYSTEMS {
        stage.add_system(empty);
    }
    stage.run(&mut world);
    (world, stage)
}

fn no_archetypes(criterion: &mut Criterion) {
    let (mut world, mut stage) = setup();
    criterion.bench_function("no_archetypes", |bencher| {
        bencher.iter(|| {
            stage.run(&mut world);
        });
    });
}

fn added_archetypes(criterion: &mut Criterion) {
    criterion.bench_function("added_archetypes", |bencher| {
        bencher.iter(|| {
            let (mut world, mut stage) = setup();
            world.spawn_batch(vec![(A(1.0), B(1.0))]);
            world.spawn_batch(vec![(A(1.0), C(1.0))]);
            world.spawn_batch(vec![(A(1.0), D(1.0))]);
            world.spawn_batch(vec![(A(1.0), E(1.0))]);
            world.spawn_batch(vec![(B(1.0), C(1.0))]);
            world.spawn_batch(vec![(B(1.0), D(1.0))]);
            world.spawn_batch(vec![(B(1.0), E(1.0))]);
            world.spawn_batch(vec![(C(1.0), D(1.0))]);
            world.spawn_batch(vec![(C(1.0), E(1.0))]);
            world.spawn_batch(vec![(D(1.0), E(1.0))]);
            stage.run(&mut world);
        })
    });
}
