use bevy_ecs::prelude::*;
use criterion::Criterion;

pub fn param_set(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("param/combinator_system");

    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(3));

    #[derive(Resource)]
    struct R;

    world.insert_resource(R);

    let mut schedule = Schedule::default();
    schedule.add_systems(
        |_: ParamSet<(
            ResMut<R>,
            ResMut<R>,
            ResMut<R>,
            ResMut<R>,
            ResMut<R>,
            ResMut<R>,
            ResMut<R>,
            ResMut<R>,
        )>| {},
    );
    // run once to initialize systems
    schedule.run(&mut world);
    group.bench_function("8_variant_param_set_system", |bencher| {
        bencher.iter(|| {
            schedule.run(&mut world);
        });
    });

    group.finish();
}
