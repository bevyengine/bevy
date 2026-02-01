use benches::bench;
use bevy_ecs::{component::Component, world::World};
use criterion::Criterion;

const ENTITY_COUNT: usize = 2_000;

#[derive(Component)]
struct C<const N: usize>;

pub fn spawn_many_zst(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(bench!("spawn_many_zst"));

    group.bench_function("static", |bencher| {
        let mut world = World::new();
        bencher.iter(|| {
            for _ in 0..ENTITY_COUNT {
                world.spawn((
                    C::<0>, C::<1>, C::<2>, C::<3>, C::<4>, C::<5>, C::<6>, C::<7>, C::<8>, C::<9>,
                    C::<10>, C::<11>, C::<12>, C::<13>, C::<14>,
                ));
            }
            world.clear_entities();
        });
    });

    group.finish();
}
