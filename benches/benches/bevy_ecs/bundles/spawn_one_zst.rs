use core::hint::black_box;

use benches::bench;
use bevy_ecs::{component::Component, world::World};
use criterion::Criterion;

const ENTITY_COUNT: usize = 10_000;

#[derive(Component)]
struct A;

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

    group.finish();
}
