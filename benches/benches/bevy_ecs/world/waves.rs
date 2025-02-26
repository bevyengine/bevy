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

macro_rules! expand_benches {
    ($bench:ident, $group:ident, $bench_name:literal, $component:ty) => {
        $group.bench_function(
            format!(
                "{0}_waves_of_{1} `{2}` spawn_batch",
                $bench.waves, $bench.entities, $bench_name
            ),
            |bencher| {
                bencher.iter(|| {
                    let mut world = World::default();
                    let mut entities = Vec::with_capacity($bench.entities as usize);
                    for _ in 0..($bench.waves - 1) {
                        entities
                            .extend(world.spawn_batch(
                                (0..$bench.entities).map(|_| <$component>::default()),
                            ));
                        for entity in entities.drain(..) {
                            world.despawn(entity);
                        }
                    }
                });
            },
        );

        $group.bench_function(
            format!(
                "{0}_waves_of_{1} `{2}` insert_or_spawn_batch",
                $bench.waves, $bench.entities, $bench_name
            ),
            |bencher| {
                bencher.iter(|| {
                    let mut world = World::default();
                    let entities = world
                        .spawn_batch((0..$bench.entities).map(|_| <$component>::default()))
                        .collect::<Vec<_>>();
                    for _ in 0..($bench.waves - 1) {
                        for entity in entities.iter().copied() {
                            world.despawn(entity);
                        }
                        world
                            .insert_or_spawn_batch(
                                entities
                                    .iter()
                                    .copied()
                                    .map(|e| (e, <$component>::default())),
                            )
                            .unwrap();
                    }
                    for entity in entities {
                        world.despawn(entity);
                    }
                });
            },
        );
    };
}

pub fn world_wave_spawn(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("spawn_waves");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(16));

    for bench in WAVES {
        expand_benches!(bench, group, "EmptyComponent", EmptyComponent);
        expand_benches!(bench, group, "LargeComponent", LargeComponent);
    }

    group.finish();
}
