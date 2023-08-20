use bevy_ecs::prelude::*;

use std::time::{Instant, Duration};

use criterion::{*, measurement::WallTime};

use crate::world_gen::*;

criterion_group!{
    name = transform_hierarchy_configurations;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(15))
        ;

    targets = transform_propagation_configurations
}

criterion_group!{
    name = transform_hierarchy_sizes;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_millis(300))
        .measurement_time(std::time::Duration::from_secs(5))
        .sample_size(50)
        ;

    targets = transform_propagation_sizes
}

/// Inner transform propagation benchmark function
/// This version only measures time spent during PostUpdate, therefore removing
/// the impact of simulating transform updates which happen during the Update schedule.
fn update_bench_postupdate_only(b: &mut Bencher<WallTime>, &(cfg, enable_update): &(&Cfg, TransformUpdates)) {
    let (_result, mut app) = build_app(cfg, enable_update);

    // Run Main schedule once to ensure initial updates are done
    app.update();

    // We want to benchmark the transform updates in the PostUpdate schedule without
    // benchmarking the update function which is intended to simulate changes to Transform
    // in a typical game.
    // Therefore, we simply remove the PostUpdate and Last schedules here in order to
    // measure the time spent in PostUpdate itself, without the time spent in the
    // schedules before PostUpdate (PreUpdate, Update, ...) and the schedules after
    // PostUpdate (only Last currently).
    // If the schedules that are part of main change, this logic needs to be changed
    // accordingly.
    let mut schedules = app.world.get_resource_mut::<Schedules>().unwrap();
    let (_, mut postupdate) = schedules.remove_entry(&bevy_app::PostUpdate).unwrap();
    let (_, mut last) = schedules.remove_entry(&bevy_app::Last).unwrap();

    b.iter_custom(|iters| {
        let mut total = Duration::ZERO;

        for _i in 0..iters {
            std::hint::black_box(app.world.run_schedule(bevy_app::Main));

            let start = Instant::now();
            std::hint::black_box(postupdate.run(&mut app.world));
            let elapsed = start.elapsed();

            std::hint::black_box({
                last.run(&mut app.world);
                app.world.clear_trackers();
            });

            total += elapsed;
        }

        total
    });
}

/// Inner transform propagation benchmark function
///
/// Simpler alternative to update_bench_postupdate_only that is retained here
/// for future reference. This benchmark includes the time spent simulating
/// transform updates in the Update schedule which makes the comparison between
/// noop and transform_updates benchmarks meaningful.
fn update_bench_reference(b: &mut Bencher<WallTime>, &(cfg, enable_update): &(&Cfg, TransformUpdates)) {
    let (_result, mut app) = build_app(cfg, enable_update);

    // Run Main schedule once to ensure initial updates are done
    app.update();

    b.iter(move || { app.update(); });

}

fn inner_update_bench(b: &mut Bencher<WallTime>, bench_cfg: &(&Cfg, TransformUpdates)) {
    const UPDATE_BENCH_POSTUPDATE_ONLY: bool = false;

    if UPDATE_BENCH_POSTUPDATE_ONLY {
        update_bench_postupdate_only(b, bench_cfg);
    } else {
        update_bench_reference(b, bench_cfg);
    }
}

#[derive(Clone, Copy)]
enum IdSource {
    Fixed(&'static str),
    NodeCount,
}

fn bench_single(group: &mut BenchmarkGroup<WallTime>, id_source: IdSource, cfg: &Cfg) {
    // Run build_app once to get an inserted node count
    let (result, _app) = build_app(cfg, TransformUpdates::Disabled);
    group.throughput(Throughput::Elements(result.inserted_nodes as u64));

    let id = |function_name| {
        match id_source {
            IdSource::Fixed(id_str) => {
                BenchmarkId::new(function_name, id_str)
            },
            IdSource::NodeCount => { 
                BenchmarkId::new(function_name, result.inserted_nodes)
            },
        }
    };

    // Measures hierarchy propagation systems when some transforms are updated.
    group.bench_with_input(id("updates"), &(cfg, TransformUpdates::Enabled), inner_update_bench);

    // Measures hierarchy propagation systems when there are no changes
    // during the Update schedule.
    group.bench_with_input(id("noop"), &(cfg, TransformUpdates::Disabled), inner_update_bench);
}

fn bench_group<F>(c: &mut Criterion, name: &str, bench_function: F) 
where
    F: FnOnce(&mut BenchmarkGroup<WallTime>) -> ()
{
    let mut group = c.benchmark_group(format!("transform_propagation_{}", name));

    // Always use linear sampling for these benchmarks
    // (they are close enough in performance, and this way the iteration time plots are consistent)
    group.sampling_mode(SamplingMode::Linear);
    
    group.sample_size(50);

    group.warm_up_time(std::time::Duration::from_millis(400));
    group.measurement_time(std::time::Duration::from_secs(5));

    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    bench_function(&mut group);

    group.finish();
}

fn bench_sizes<I>(c: &mut Criterion, name: &str, cfgs: I) 
where
    I: IntoIterator<Item = Cfg>
{
    bench_group(c, name, |group| {
        for cfg in cfgs {
            bench_single(group, IdSource::NodeCount, &cfg);
        }
    });
}

fn transform_propagation_sizes(c: &mut Criterion) {
    bench_sizes(c, "large", (6u32..=18u32).map(|depth| {
        Cfg {
            test_case: TestCase::NonUniformTree {
                depth,
                branch_width: 8,
            },
            update_filter: Default::default(),
        }
    }));
    bench_sizes(c, "deep", (8u32..=24u32).map(|depth| {
        Cfg {
            test_case: TestCase::NonUniformTree {
                depth,
                branch_width: 2,
            },
            update_filter: Default::default(),
        }
    }));
    bench_sizes(c, "wide", (20u32..=470u32).step_by(30).map(|branch_width| {
        Cfg {
            test_case: TestCase::Tree {
                depth: 3,
                branch_width,
            },
            update_filter: Default::default(),
        }
    }));
}

fn transform_propagation_configurations(c: &mut Criterion) {
    bench_group(c, "all_configurations", |group| {
        for (name, cfg) in &CONFIGS {
            bench_single(group, IdSource::Fixed(name), cfg);
        }
    });
}

