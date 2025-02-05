mod index_iter_indexed;
mod index_iter_naive;
mod index_update_indexed;
mod index_update_naive;

use criterion::{criterion_group, Criterion};

criterion_group!(benches, index_iter, index_update,);

fn index_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_iter");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("naive", |b| {
        let mut bench = index_iter_naive::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("indexed", |b| {
        let mut bench = index_iter_indexed::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn index_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_update");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("naive", |b| {
        let mut bench = index_update_naive::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("indexed", |b| {
        let mut bench = index_update_indexed::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}
