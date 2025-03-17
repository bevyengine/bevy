use bevy_ecs::prelude::*;
use criterion::Criterion;
use glam::*;

#[derive(Component)]
struct A(Mat4);
#[derive(Component)]
struct B(Vec4);

pub fn world_despawn_recursive(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("despawn_world_recursive");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in (0..5).map(|i| 10_u32.pow(i)) {
        let mut world = World::default();
        for _ in 0..entity_count {
            world
                .spawn((A(Mat4::default()), B(Vec4::default())))
                .with_children(|parent| {
                    parent.spawn((A(Mat4::default()), B(Vec4::default())));
                });
        }

        let ents = world.iter_entities().map(|e| e.id()).collect::<Vec<_>>();
        group.bench_function(format!("{}_entities", entity_count), |bencher| {
            bencher.iter(|| {
                ents.iter().for_each(|e| {
                    world.entity_mut(*e).despawn();
                });
            });
        });
    }

    group.finish();
}
