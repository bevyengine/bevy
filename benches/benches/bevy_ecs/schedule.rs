use bevy_app::App;
use bevy_ecs::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};

criterion_group!(benches, build_schedule);
criterion_main!(benches);

fn build_schedule(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("build_schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(15));

    // Benchmark graphs of different sizes.
    for graph_size in [100, 500, 1000] {
        // Basic benchmark without constraints.
        group.bench_function(format!("{graph_size}_schedule_noconstraints"), |bencher| {
            bencher.iter(|| {
                let mut app = App::new();
                for _ in 0..graph_size {
                    // empty system
                    fn sys() {}
                    app.add_system(sys);
                }
            });
        });
        // Benchmark with constraints.
        group.bench_function(format!("{graph_size}_schedule"), |bencher| {
            bencher.iter(|| {
                // empty system
                fn sys() {}

                // Use multiple different kinds of label;
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
                struct EvenLabel(usize);
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
                struct OddLabel(usize);

                // unique label
                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemLabel)]
                struct NumLabel(usize);

                // Method: generate a set of `graph_size` systems which have a One True Ordering.
                // Add system to the stage with full constraints. Hopefully this should be maximimally
                // difficult for bevy to figure out.

                let mut app = App::new();
                for i in 0..graph_size {
                    let mut sys = if i % 2 == 0 {
                        sys.label(EvenLabel(i))
                    } else {
                        sys.label(OddLabel(i))
                    };
                    for a in 0..i {
                        sys = if a % 2 == 0 {
                            sys.after(EvenLabel(a))
                        } else {
                            sys.after(OddLabel(a))
                        }
                    }
                    for b in i + 1..graph_size {
                        sys = if b % 2 == 0 {
                            sys.before(EvenLabel(b))
                        } else {
                            sys.before(OddLabel(b))
                        }
                    }
                    app.add_system(sys);
                }
                app.run();
            });
        });
    }

    group.finish();
}
