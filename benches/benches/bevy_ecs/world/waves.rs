use bevy_ecs::prelude::*;
use criterion::Criterion;

#[derive(Component, Default)]
struct EmptyComponent;

#[derive(Component, Default)]
struct LargeComponent([u64; 32]);

const WAVES: &[WaveBench] = &[
    WaveBench {
        waves: 1,
        entities: 100_000,
    },
    WaveBench {
        waves: 16,
        entities: 100_000,
    },
];

struct WaveBench {
    waves: u8,
    entities: u32,
}

pub fn world_spawn(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("spawn_waves");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(16));

    for bench in WAVES {
        group.bench_function(
            format!(
                "{0}_waves_of_{1} `EmptyComponent` spawn_batch",
                bench.waves, bench.entities
            ),
            |bencher| {
                let mut world = World::default();
                bencher.iter(|| {
                    let mut entities = Vec::with_capacity(bench.entities as usize);
                    for _ in 0..(bench.waves - 1) {
                        entities
                            .extend(world.spawn_batch((0..bench.entities).map(|_| EmptyComponent)));
                        for entity in entities.drain(..) {
                            world.despawn(entity);
                        }
                    }
                });
            },
        );
    }

    group.finish();
}
