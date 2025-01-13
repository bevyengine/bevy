use bevy_ecs::prelude::*;
use bevy_ecs::world::CommandQueue;
use bevy_hierarchy::prelude::*;
use core::hint::black_box;
use criterion::{criterion_group, Criterion};

criterion_group!(benches, spawn_children);

fn spawn_children(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_children");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in (1..7).map(|i| 10i32.pow(i)) {
        group.bench_function(format!("{}_entities", entity_count), |bencher| {
            let mut world = World::default();
            let mut command_queue = CommandQueue::default();

            bencher.iter(|| {
                let mut commands = Commands::new(&mut command_queue, &world);
                let mut entity = commands.spawn_empty();

                entity.with_children(|c| {
                    for _ in 0..entity_count {
                        c.spawn_empty();
                    }
                });

                entity.with_child(());
                entity.with_children(|c| {
                    for _ in 0..entity_count {
                        c.spawn_empty();
                    }
                });
                command_queue.apply(black_box(&mut world));
            });

            black_box(world);
        });
    }
}
