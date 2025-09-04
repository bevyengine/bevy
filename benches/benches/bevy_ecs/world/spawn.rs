use bevy_ecs::prelude::*;
use criterion::Criterion;
use glam::*;

#[derive(Component)]
struct A(Mat4);
#[derive(Component)]
struct B(Vec4);

pub fn world_spawn(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("spawn_world");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in [1, 100, 10_000] {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            let mut world = World::default();
            bencher.iter(|| {
                for _ in 0..entity_count {
                    world.spawn((A(Mat4::default()), B(Vec4::default())));
                }
            });
        });
    }

    group.finish();
}
