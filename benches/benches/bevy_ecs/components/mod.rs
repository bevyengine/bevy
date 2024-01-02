use criterion::*;

mod add_remove_big_sparse_set;
mod add_remove_big_table;
mod add_remove_sparse_set;
mod add_remove_table;
mod archetype_updates;
mod insert_simple;
mod insert_simple_unbatched;

use archetype_updates::*;

criterion_group!(
    components_benches,
    add_remove,
    add_remove_big,
    insert_simple,
    no_archetypes,
    added_archetypes,
);

fn add_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
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
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
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

fn insert_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_simple");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
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
