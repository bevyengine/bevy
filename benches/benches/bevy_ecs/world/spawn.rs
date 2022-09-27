use bevy_ecs::prelude::*;
use criterion::Criterion;
use glam::*;

#[derive(Component)]
struct A(Mat4);
#[derive(Component)]
struct B(Vec4);

pub fn world_spawn(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("spawn_world");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    for entity_count in (0..5).map(|i| 10_u32.pow(i)) {
        group.bench_function(format!("{}_entities", entity_count), |bencher| {
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
