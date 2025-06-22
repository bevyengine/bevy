use benches::bench;
use bevy_ecs::{component::Component, world::World};
use criterion::Criterion;

const ENTITY_COUNT: usize = 10_000;

#[derive(Component)]
struct A;

pub fn spawn_one_zst(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_one_zst"));

    group.bench_function("static", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world.spawn(A);
            }
            world.clear_entities();
        });
    });

    group.finish();
}
