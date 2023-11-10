use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use criterion::Criterion;

pub fn schedule(c: &mut Criterion) {
    #[derive(Component)]
    struct A(f32);
    #[derive(Component)]
    struct B(f32);
    #[derive(Component)]
    struct C(f32);
    #[derive(Component)]
    struct D(f32);
    #[derive(Component)]
    struct E(f32);

    fn ab(mut query: Query<(&mut A, &mut B)>) {
        query.for_each_mut(|(mut a, mut b)| {
            std::mem::swap(&mut a.0, &mut b.0);
        });
    }

    fn cd(mut query: Query<(&mut C, &mut D)>) {
        query.for_each_mut(|(mut c, mut d)| {
            std::mem::swap(&mut c.0, &mut d.0);
        });
    }

    fn ce(mut query: Query<(&mut C, &mut E)>) {
        query.for_each_mut(|(mut c, mut e)| {
            std::mem::swap(&mut c.0, &mut e.0);
        });
    }

    let mut group = c.benchmark_group("schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut world = World::default();

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0), D(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0), E(0.0))));

        let mut schedule = Schedule::default();
        schedule.add_systems((ab, cd, ce));
        schedule.run(&mut world);

        b.iter(move || schedule.run(&mut world));
    });
    group.finish();
}

pub fn build_schedule(criterion: &mut Criterion) {
    // empty system
    fn empty_system() {}

    // Use multiple different kinds of label to ensure that dynamic dispatch
    // doesn't somehow get optimized away.
    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct NumSet(usize);

    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct DummySet;

    let mut group = criterion.benchmark_group("build_schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(15));

    // Method: generate a set of `graph_size` systems which have a One True Ordering.
    // Add system to the schedule with full constraints. Hopefully this should be maximally
    // difficult for bevy to figure out.
    let labels: Vec<_> = (0..1000).map(|i| NumSet(i)).collect();

    // Benchmark graphs of different sizes.
    for graph_size in [100, 500, 1000] {
        // Basic benchmark without constraints.
        group.bench_function(format!("{graph_size}_schedule_noconstraints"), |bencher| {
            bencher.iter(|| {
                let mut app = App::new();
                for _ in 0..graph_size {
                    app.add_systems(Update, empty_system);
                }
                app.update();
            });
        });

        // Benchmark with constraints.
        group.bench_function(format!("{graph_size}_schedule"), |bencher| {
            bencher.iter(|| {
                let mut app = App::new();
                app.add_systems(Update, empty_system.in_set(DummySet));

                // Build a fully-connected dependency graph describing the One True Ordering.
                // Not particularly realistic but this can be refined later.
                for i in 0..graph_size {
                    let mut sys = empty_system.in_set(labels[i]).before(DummySet);
                    for label in labels.iter().take(i) {
                        sys = sys.after(*label);
                    }
                    for label in &labels[i + 1..graph_size] {
                        sys = sys.before(*label);
                    }
                    app.add_systems(Update, sys);
                }
                // Run the app for a single frame.
                // This is necessary since dependency resolution does not occur until the game runs.
                // FIXME: Running the game clutters up the benchmarks, so ideally we'd be
                // able to benchmark the dependency resolution directly.
                app.update();
            });
        });
    }

    group.finish();
}
