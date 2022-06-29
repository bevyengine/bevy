use criterion::*;

mod add_remove_big_sparse_set;
mod add_remove_big_table;
mod add_remove_sparse_set;
mod add_remove_table;
mod frag_iter;
mod frag_iter_wide;
mod frag_iter_foreach;
mod frag_iter_foreach_wide;
mod get_component;
mod get_component_system;
mod heavy_compute;
mod schedule;
mod simple_insert;
mod simple_insert_unbatched;
mod simple_iter;
mod simple_iter_wide;
mod simple_iter_foreach;
mod simple_iter_foreach_wide;
mod simple_iter_sparse;
mod simple_iter_sparse_wide;
mod simple_iter_sparse_foreach;
mod simple_iter_sparse_foreach_wide;
mod simple_iter_system;
mod sparse_frag_iter;
mod sparse_frag_iter_wide;
mod sparse_frag_iter_foreach;
mod sparse_frag_iter_foreach_wide;

fn bench_simple_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_insert");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = simple_insert::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("unbatched", |b| {
        let mut bench = simple_insert_unbatched::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_simple_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = simple_iter::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("wide", |b| {
        let mut bench = simple_iter_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("system", |b| {
        let mut bench = simple_iter_system::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse", |b| {
        let mut bench = simple_iter_sparse::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_wide", |b| {
        let mut bench = simple_iter_sparse_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = simple_iter_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_wide", |b| {
        let mut bench = simple_iter_foreach_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_foreach", |b| {
        let mut bench = simple_iter_sparse_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_foreach_wide", |b| {
        let mut bench = simple_iter_sparse_foreach_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_frag_iter_bc(c: &mut Criterion) {
    let mut group = c.benchmark_group("fragmented_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = frag_iter::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("wide", |b| {
        let mut bench = frag_iter_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = frag_iter_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_wide", |b| {
        let mut bench = frag_iter_foreach_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_sparse_frag_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_fragmented_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = sparse_frag_iter::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("wide", |b| {
        let mut bench = sparse_frag_iter_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = sparse_frag_iter_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach_wide", |b| {
        let mut bench = sparse_frag_iter_foreach_wide::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_schedule(c: &mut Criterion) {
    let mut group = c.benchmark_group("schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = schedule::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_heavy_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_compute");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = heavy_compute::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_add_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove_component");
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

fn bench_add_remove_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove_component_big");
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

fn bench_get_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_component");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = get_component::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("system", |b| {
        let mut bench = get_component_system::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

criterion_group!(
    benchmarks,
    bench_simple_insert,
    bench_simple_iter,
    bench_frag_iter_bc,
    bench_sparse_frag_iter,
    bench_schedule,
    bench_heavy_compute,
    bench_add_remove,
    bench_add_remove_big,
    bench_get_component,
);
criterion_main!(benchmarks);
