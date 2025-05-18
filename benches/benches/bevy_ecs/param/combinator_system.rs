use bevy_ecs::prelude::*;
use criterion::Criterion;

pub fn combinator_system(criterion: &mut Criterion) {
    let mut world = World::new();
    let mut group = criterion.benchmark_group("param/combinator_system");

    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(3));

    let mut schedule = Schedule::default();
    schedule.add_systems(
        (|| {})
            .pipe(|| {})
            .pipe(|| {})
            .pipe(|| {})
            .pipe(|| {})
            .pipe(|| {})
            .pipe(|| {})
            .pipe(|| {}),
    );
    // run once to initialize systems
    schedule.run(&mut world);
    group.bench_function("8_piped_systems", |bencher| {
        bencher.iter(|| {
            schedule.run(&mut world);
        });
    });

    group.finish();
}
