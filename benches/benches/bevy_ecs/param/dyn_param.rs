use bevy_ecs::{
    prelude::*,
    system::{DynParamBuilder, DynSystemParam, ParamBuilder},
};
use criterion::Criterion;

pub fn dyn_param(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("param/combinator_system");

    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(3));

    #[derive(Resource)]
    struct R;

    world.insert_resource(R);

    let mut schedule = Schedule::default();
    let system = (
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
        DynParamBuilder::new::<Res<R>>(ParamBuilder),
    )
        .build_state(&mut world)
        .build_system(
            |_: DynSystemParam,
             _: DynSystemParam,
             _: DynSystemParam,
             _: DynSystemParam,
             _: DynSystemParam,
             _: DynSystemParam,
             _: DynSystemParam,
             _: DynSystemParam| {},
        );
    schedule.add_systems(system);
    // run once to initialize systems
    schedule.run(&mut world);
    group.bench_function("8_dyn_params_system", |bencher| {
        bencher.iter(|| {
            schedule.run(&mut world);
        });
    });

    group.finish();
}
