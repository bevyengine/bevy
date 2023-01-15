use std::{iter, time::Duration};

use bevy_reflect::{DynamicList, List};
use criterion::{
    black_box, criterion_group, criterion_main, measurement::Measurement, BatchSize,
    BenchmarkGroup, BenchmarkId, Criterion, Throughput,
};

criterion_group!(
    benches,
    concrete_list_apply,
    concrete_list_clone_dynamic,
    dynamic_list_apply,
    dynamic_list_push
);
criterion_main!(benches);

const WARM_UP_TIME: Duration = Duration::from_millis(500);
const MEASUREMENT_TIME: Duration = Duration::from_secs(4);

// log10 scaling
const SIZES: [usize; 5] = [100_usize, 316, 1000, 3162, 10000];

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
    let mut group = criterion.benchmark_group("concrete_list_apply");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

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
    let mut group = criterion.benchmark_group("concrete_list_clone_dynamic");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

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
    let mut group = criterion.benchmark_group("dynamic_list_push");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

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
    let mut group = criterion.benchmark_group("dynamic_list_apply");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);

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
