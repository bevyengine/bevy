use criterion::*;

mod archetype_maniplation;
mod get_component;
mod query_iteration;
mod scheduling;

fn bench_simple_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_insert");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = archetype_manipulation::simple_insert::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("unbatched", |b| {
        let mut bench = archetype_manipulation::simple_insert_unbatched::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_simple_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = query_iteration::simple_iter::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("system", |b| {
        let mut bench = query_iteration::simple_iter_system::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse", |b| {
        let mut bench = query_iteration::simple_iter_sparse::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = query_iteration::simple_iter_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_foreach", |b| {
        let mut bench = query_iteration::simple_iter_sparse_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_frag_iter_bc(c: &mut Criterion) {
    let mut group = c.benchmark_group("fragmented_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = query_iteration::frag_iter::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = query_iteration::frag_iter_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_sparse_frag_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_fragmented_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = query_iteration::sparse_frag_iter::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("foreach", |b| {
        let mut bench = query_iteration::sparse_frag_iter_foreach::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_schedule(c: &mut Criterion) {
    let mut group = c.benchmark_group("schedule");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = scheduling::schedule::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_heavy_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_compute");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = query_iteration::heavy_compute::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_add_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove_component");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("table", |b| {
        let mut bench = archetype_manipulation::add_remove_table::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_set", |b| {
        let mut bench = archetype_manipulation::add_remove_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_add_remove_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove_component_big");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("table", |b| {
        let mut bench = archetype_manipulation::add_remove_big_table::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("sparse_set", |b| {
        let mut bench = archetype_manipulation::add_remove_big_sparse_set::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.finish();
}

fn bench_get_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_component");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut bench = get_component::get_component::Benchmark::new();
        b.iter(move || bench.run());
    });
    group.bench_function("system", |b| {
        let mut bench = get_component::get_component_system::Benchmark::new();
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
