use criterion::*;

mod heavy_compute;
mod iter_frag;
mod iter_frag_foreach;
mod iter_frag_foreach_sparse;
mod iter_frag_foreach_wide;
mod iter_frag_foreach_wide_sparse;
mod iter_frag_sparse;
mod iter_frag_wide;
mod iter_frag_wide_sparse;
mod iter_simple;
mod iter_simple_foreach;
mod iter_simple_foreach_sparse_set;
mod iter_simple_foreach_wide;
mod iter_simple_foreach_wide_sparse_set;
mod iter_simple_sparse_set;
mod iter_simple_system;
mod iter_simple_wide;
mod iter_simple_wide_sparse_set;
mod par_iter_simple;

use heavy_compute::*;

criterion_group!(
    iterations_benches,
    iter_frag,
    iter_frag_sparse,
    iter_simple,
    heavy_compute,
    par_iter_simple,
);

fn iter_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("iter_simple");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = iter_simple::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("wide", |b| {
        let mut bench = iter_simple_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("system", |b| {
        let mut bench = iter_simple_system::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_set", |b| {
        let mut bench = iter_simple_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("wide_sparse_set", |b| {
        let mut bench = iter_simple_wide_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = iter_simple_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_wide", |b| {
        let mut bench = iter_simple_foreach_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_sparse_set", |b| {
        let mut bench = iter_simple_foreach_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_wide_sparse_set", |b| {
        let mut bench = iter_simple_foreach_wide_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn iter_frag(c: &mut Criterion) {
    let mut group = c.benchmark_group("iter_fragmented");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = iter_frag::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("wide", |b| {
        let mut bench = iter_frag_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = iter_frag_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_wide", |b| {
        let mut bench = iter_frag_foreach_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn iter_frag_sparse(c: &mut Criterion) {
    let mut group = c.benchmark_group("iter_fragmented_sparse");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = iter_frag_sparse::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("wide", |b| {
        let mut bench = iter_frag_wide_sparse::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = iter_frag_foreach_sparse::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_wide", |b| {
        let mut bench = iter_frag_foreach_wide_sparse::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn par_iter_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("par_iter_simple");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for f in [0, 10, 100, 1000] {
        group.bench_function(format!("with_{}_fragment", f), |b| {
            let mut bench = par_iter_simple::Benchmark::new(f);
            b.iter(move || bench.run());
        });
    }
}
