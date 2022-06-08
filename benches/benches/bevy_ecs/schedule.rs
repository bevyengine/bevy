use bevy_app::App;
use bevy_ecs::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};

criterion_group!(benches, build_schedule);
criterion_main!(benches);

fn build_schedule(criterion: &mut Criterion) {
    // empty system
    fn empty_system() {}

    // Use multiple different kinds of label to ensure that dynamic dispatch
    // doesn't somehow get optimized away.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
    struct NumLabel(usize);
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
    struct DummyLabel;

    let mut group = criterion.benchmark_group("build_schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(15));

    // Method: generate a set of `graph_size` systems which have a One True Ordering.
    // Add system to the stage with full constraints. Hopefully this should be maximimally
    // difficult for bevy to figure out.
    let labels: Vec<_> = (0..1000).map(NumLabel).collect();

    // Benchmark graphs of different sizes.
    for graph_size in [100, 500, 1000] {
        // Basic benchmark without constraints.
        group.bench_function(format!("{graph_size}_schedule_noconstraints"), |bencher| {
            bencher.iter(|| {
                let mut app = App::new();
                for _ in 0..graph_size {
                    app.add_system(empty_system);
                }
                app.update();
            });
        });

        // Benchmark with constraints.
        group.bench_function(format!("{graph_size}_schedule"), |bencher| {
            bencher.iter(|| {
                let mut app = App::new();
                app.add_system(empty_system.label(DummyLabel));

                // Build a fully-connected dependency graph describing the One True Ordering.
                // Not particularly realistic but this can be refined later.
                for i in 0..graph_size {
                    let mut sys = empty_system.label(labels[i]).before(DummyLabel);
                    for a in 0..i {
                        sys = sys.after(labels[a]);
                    }
                    for b in i + 1..graph_size {
                        sys = sys.before(labels[b]);
                    }
                    app.add_system(sys);
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
