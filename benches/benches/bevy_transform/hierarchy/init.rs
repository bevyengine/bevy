use bevy_app::App;

use std::time::{Instant, Duration};

use criterion::*;

use crate::world_gen::*;

criterion_group!{
    name = transform_hierarchy_init;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_secs(3))
        .measurement_time(std::time::Duration::from_secs(20));
    targets = transform_init
}

/// This benchmark group tries to measure the cost of the initial transform propagation,
/// i.e. the first time transform propagation runs after we just added all our entities.
///
/// These benchmarks are probably not as useful as the transform update benchmarks
/// since the benchmark implementation is a little fragile and rather slow (see comments below).
/// They're included here nevertheless in case they're useful.
fn transform_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform_init");

    // Reduce sample size and enable flat sampling to make sure this benchmark doesn't
    // take a lot longer than the simplified benchmark.
    group.sample_size(50);
    group.sampling_mode(SamplingMode::Flat);

    for (name, cfg) in &CONFIGS {
        let (result, mut app) = build_app(cfg, TransformUpdates::Disabled);

        group.throughput(Throughput::Elements(result.inserted_nodes as u64));

        // Simplified benchmark for the initial propagation
        group.bench_function(BenchmarkId::new("reset", name), move |b| {
            // Building the World (in setup) takes a lot of time, so ideally we shouldn't do that
            // on every iteration since Criterion ideally wants to run the benchmark function in batches.
            // Unfortunately, we can't re-use an App directly in iter() because the World would no
            // longer be in its pristine, just initialized state from the second iteration onwards.
            // Furthermore, it's not possible to clone a pristine World since World doesn't implement
            // Clone.
            // As an alternative, we reuse the same App and reset it to a pseudo-pristine state by
            // simply marking all Parent, Children and Transform components as changed.
            // This should look like a pristine state to the propagation systems.
            //
            // Note: This is a tradeoff. The reset benchmark should deliver more reliable results
            // in the same time, while the reference benchmark below should be closer to the
            // real-world initialization cost.

            app.add_schedule(ResetSchedule, reset_schedule());

            // Run Main schedule once to ensure initial updates are done
            // This is a little counterintuitive since the initial delay is exactly what we want to
            // measure - however, we have the ResetSchedule in place to hopefully replicate the
            // World in its pristine state on every iteration.
            // We therefore run update here to prevent the first iteration having additional work
            // due to possible incompleteness of the reset mechanism
            app.update();

            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;

                for _i in 0..iters {
                    std::hint::black_box(app.world.run_schedule(ResetSchedule));

                    let start = Instant::now();
                    std::hint::black_box(app.world.run_schedule(bevy_app::Main));
                    let elapsed = start.elapsed();

                    app.world.clear_trackers();

                    total += elapsed;
                }

                total
            });
        });

        // Reference benchmark for the initial propagation - needs to rebuild the App
        // on every iteration, which makes the benchmark quite slow and results
        // in less precise results in the same time compared to the simplified benchmark.
        group.bench_with_input(BenchmarkId::new("reference", name), cfg, move |b, cfg| {
            // Use iter_batched_ref to prevent influence of Drop
            b.iter_batched_ref(
                || {
                    let (_result, app) = build_app(cfg, TransformUpdates::Disabled);
                    app
                },
                App::update,
                BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

