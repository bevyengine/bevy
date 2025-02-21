use core::{hint::black_box, iter, time::Duration};

use benches::bench;
use bevy_reflect::{DynamicList, List};
use criterion::{
    criterion_group, measurement::Measurement, AxisScale, BatchSize, BenchmarkGroup, BenchmarkId,
    Criterion, PlotConfiguration, Throughput,
};

criterion_group!(
    benches,
    concrete_list_apply,
    concrete_list_clone_dynamic,
    dynamic_list_apply,
    dynamic_list_push
);

// Use a shorter warm-up time (from 3 to 0.5 seconds) and measurement time (from 5 to 4) because we
// have so many combinations (>50) to benchmark.
const WARM_UP_TIME: Duration = Duration::from_millis(500);
const MEASUREMENT_TIME: Duration = Duration::from_secs(4);

/// An array of list sizes used in benchmarks.
///
/// This scales logarithmically.
const SIZES: [usize; 5] = [100, 316, 1000, 3162, 10000];

/// Creates a [`BenchmarkGroup`] with common configuration shared by all benchmarks within this
/// module.
fn create_group<'a, M: Measurement>(c: &'a mut Criterion<M>, name: &str) -> BenchmarkGroup<'a, M> {
    let mut group = c.benchmark_group(name);

    group
        .warm_up_time(WARM_UP_TIME)
        .measurement_time(MEASUREMENT_TIME)
        // Make the plots logarithmic, matching `SIZES`' scale.
        .plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    group
}

fn list_apply<M, LBase, LPatch, F1, F2, F3>(
    group: &mut BenchmarkGroup<M>,
    bench_name: &str,
    f_base: F1,
    f_patch: F3,
) where
    M: Measurement,
    LBase: List,
    LPatch: List,
    F1: Fn(usize) -> F2,
    F2: Fn() -> LBase,
    F3: Fn(usize) -> LPatch,
{
    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new(bench_name, size),
            &size,
            |bencher, &size| {
                let f_base = f_base(size);
                let patch = f_patch(size);
                bencher.iter_batched(
                    f_base,
                    |mut base| base.apply(black_box(&patch)),
                    BatchSize::SmallInput,
                );
            },
        );
    }
}

fn concrete_list_apply(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("concrete_list_apply"));

    let empty_base = |_: usize| Vec::<u64>::new;
    let full_base = |size: usize| move || iter::repeat(0).take(size).collect::<Vec<u64>>();
    let patch = |size: usize| iter::repeat(1).take(size).collect::<Vec<u64>>();

    list_apply(&mut group, "empty_base_concrete_patch", empty_base, patch);

    list_apply(&mut group, "empty_base_dynamic_patch", empty_base, |size| {
        patch(size).clone_dynamic()
    });

    list_apply(&mut group, "same_len_concrete_patch", full_base, patch);

    list_apply(&mut group, "same_len_dynamic_patch", full_base, |size| {
        patch(size).clone_dynamic()
    });

    group.finish();
}

fn concrete_list_clone_dynamic(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("concrete_list_clone_dynamic"));

    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                let v = iter::repeat(0).take(size).collect::<Vec<_>>();

                bencher.iter(|| black_box(&v).clone_dynamic());
            },
        );
    }

    group.finish();
}

fn dynamic_list_push(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("dynamic_list_push"));

    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                let src = iter::repeat(()).take(size).collect::<Vec<_>>();
                let dst = DynamicList::default();

                bencher.iter_batched(
                    || (src.clone(), dst.clone_dynamic()),
                    |(src, mut dst)| {
                        for item in src {
                            dst.push(item);
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn dynamic_list_apply(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("dynamic_list_apply"));

    let empty_base = |_: usize| || Vec::<u64>::new().clone_dynamic();
    let full_base = |size: usize| move || iter::repeat(0).take(size).collect::<Vec<u64>>();
    let patch = |size: usize| iter::repeat(1).take(size).collect::<Vec<u64>>();

    list_apply(&mut group, "empty_base_concrete_patch", empty_base, patch);

    list_apply(&mut group, "empty_base_dynamic_patch", empty_base, |size| {
        patch(size).clone_dynamic()
    });

    list_apply(&mut group, "same_len_concrete_patch", full_base, patch);

    list_apply(&mut group, "same_len_dynamic_patch", full_base, |size| {
        patch(size).clone_dynamic()
    });

    group.finish();
}
