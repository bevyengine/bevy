use core::{fmt::Write, hint::black_box, iter, time::Duration};

use benches::bench;
use bevy_platform::collections::HashMap;
use bevy_reflect::{DynamicMap, Map};
use criterion::{
    criterion_group, measurement::Measurement, AxisScale, BatchSize, BenchmarkGroup, BenchmarkId,
    Criterion, PlotConfiguration, Throughput,
};

criterion_group!(
    benches,
    concrete_map_apply,
    dynamic_map_apply,
    dynamic_map_get,
    dynamic_map_insert
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

/// Generic benchmark for applying one `Map` to another.
///
/// `f_base` is a function which takes an input size and produces a generator
/// for new base maps. `f_patch` is a function which produces a map to be
/// applied to the base map.
fn map_apply<M, MBase, MPatch, F1, F2, F3>(
    group: &mut BenchmarkGroup<M>,
    bench_name: &str,
    f_base: F1,
    f_patch: F3,
) where
    M: Measurement,
    MBase: Map,
    MPatch: Map,
    F1: Fn(usize) -> F2,
    F2: Fn() -> MBase,
    F3: Fn(usize) -> MPatch,
{
    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new(bench_name, size),
            &size,
            |bencher, &size| {
                let f_base = f_base(size);
                bencher.iter_batched(
                    || (f_base(), f_patch(size)),
                    |(mut base, patch)| base.apply(black_box(&patch)),
                    BatchSize::SmallInput,
                );
            },
        );
    }
}

fn concrete_map_apply(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("concrete_map_apply"));

    let empty_base = |_: usize| HashMap::<u64, u64>::default;

    let key_range_base = |size: usize| {
        move || {
            (0..size as u64)
                .zip(iter::repeat(0))
                .collect::<HashMap<u64, u64>>()
        }
    };

    let key_range_patch = |size: usize| {
        (0..size as u64)
            .zip(iter::repeat(1))
            .collect::<HashMap<u64, u64>>()
    };

    let disjoint_patch = |size: usize| {
        (size as u64..2 * size as u64)
            .zip(iter::repeat(1))
            .collect::<HashMap<u64, u64>>()
    };

    map_apply(
        &mut group,
        "empty_base_concrete_patch",
        empty_base,
        key_range_patch,
    );

    map_apply(&mut group, "empty_base_dynamic_patch", empty_base, |size| {
        key_range_patch(size).to_dynamic_map()
    });

    map_apply(
        &mut group,
        "same_keys_concrete_patch",
        key_range_base,
        key_range_patch,
    );

    map_apply(
        &mut group,
        "same_keys_dynamic_patch",
        key_range_base,
        |size| key_range_patch(size).to_dynamic_map(),
    );

    map_apply(
        &mut group,
        "disjoint_keys_concrete_patch",
        key_range_base,
        disjoint_patch,
    );

    map_apply(
        &mut group,
        "disjoint_keys_dynamic_patch",
        key_range_base,
        |size| disjoint_patch(size).to_dynamic_map(),
    );
}

fn u64_to_n_byte_key(k: u64, n: usize) -> String {
    let mut key = String::with_capacity(n);
    write!(&mut key, "{k}").unwrap();

    // Pad key to n bytes.
    key.extend(iter::repeat_n('\0', n - key.len()));
    key
}

fn dynamic_map_apply(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("dynamic_map_apply"));

    let empty_base = |_: usize| DynamicMap::default;

    let key_range_base = |size: usize| {
        move || {
            (0..size as u64)
                .zip(iter::repeat(0))
                .collect::<HashMap<u64, u64>>()
                .to_dynamic_map()
        }
    };

    let key_range_patch = |size: usize| {
        (0..size as u64)
            .zip(iter::repeat(1))
            .collect::<HashMap<u64, u64>>()
    };

    let disjoint_patch = |size: usize| {
        (size as u64..2 * size as u64)
            .zip(iter::repeat(1))
            .collect::<HashMap<u64, u64>>()
    };

    map_apply(
        &mut group,
        "empty_base_concrete_patch",
        empty_base,
        key_range_patch,
    );

    map_apply(&mut group, "empty_base_dynamic_patch", empty_base, |size| {
        key_range_patch(size).to_dynamic_map()
    });

    map_apply(
        &mut group,
        "same_keys_concrete_patch",
        key_range_base,
        key_range_patch,
    );

    map_apply(
        &mut group,
        "same_keys_dynamic_patch",
        key_range_base,
        |size| key_range_patch(size).to_dynamic_map(),
    );

    map_apply(
        &mut group,
        "disjoint_keys_concrete_patch",
        key_range_base,
        disjoint_patch,
    );

    map_apply(
        &mut group,
        "disjoint_keys_dynamic_patch",
        key_range_base,
        |size| disjoint_patch(size).to_dynamic_map(),
    );
}

fn dynamic_map_get(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("dynamic_map_get"));

    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("u64_keys", size),
            &size,
            |bencher, &size| {
                let mut map = DynamicMap::default();
                for i in 0..size as u64 {
                    map.insert(i, i);
                }

                bencher.iter(|| {
                    for i in 0..size as u64 {
                        let key = black_box(i);
                        black_box(map.get(&key));
                    }
                });
            },
        );
    }

    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("64_byte_keys", size),
            &size,
            |bencher, &size| {
                let mut map = DynamicMap::default();
                let mut keys = Vec::with_capacity(size);
                for i in 0..size as u64 {
                    let key = u64_to_n_byte_key(i, 64);
                    map.insert(key.clone(), i);
                    keys.push(key);
                }

                bencher.iter(|| {
                    for key in keys.iter().take(size) {
                        let key = black_box(key);
                        assert!(map.get(key).is_some());
                    }
                });
            },
        );
    }
}

fn dynamic_map_insert(criterion: &mut Criterion) {
    let mut group = create_group(criterion, bench!("dynamic_map_insert"));

    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("u64_keys", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    DynamicMap::default,
                    |mut map| {
                        for i in 0..size as u64 {
                            let key = black_box(i);
                            map.insert(key, black_box(i));
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("64_byte_keys", size),
            &size,
            |bencher, &size| {
                let mut keys = Vec::with_capacity(size);
                for i in 0..size {
                    let key = u64_to_n_byte_key(i as u64, 64);
                    keys.push(key);
                }

                bencher.iter_batched(
                    || (DynamicMap::default(), keys.clone()),
                    |(mut map, keys)| {
                        for (i, key) in keys.into_iter().enumerate() {
                            let key = black_box(key);
                            map.insert(key, i);
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
}
