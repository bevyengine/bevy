mod add_remove;
mod add_remove_big_sparse_set;
mod add_remove_big_table;
mod add_remove_sparse_set;
mod add_remove_table;
mod add_remove_very_big_table;
mod archetype_updates;
mod insert_simple;
mod insert_simple_unbatched;

use archetype_updates::*;
use criterion::{criterion_group, Criterion};

criterion_group!(
    benches,
    add_remove,
    add_remove_big,
    add_remove_very_big,
    insert_simple,
    no_archetypes,
    added_archetypes,
);

fn add_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("table", |b| {
        let mut bench = add_remove_table::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_set", |b| {
        let mut bench = add_remove_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn add_remove_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove_big");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("table", |b| {
        let mut bench = add_remove_big_table::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_set", |b| {
        let mut bench = add_remove_big_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn add_remove_very_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove_very_big");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("table", |b| {
        let mut bench = add_remove_very_big_table::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn insert_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_simple");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = insert_simple::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("unbatched", |b| {
        let mut bench = insert_simple_unbatched::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}
