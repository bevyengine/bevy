use criterion::{criterion_main, BenchmarkGroup};

mod components;
mod iteration;
mod scheduling;
mod world;

criterion_main!(
    iteration::iterations_benches,
    components::components_benches,
    scheduling::scheduling_benches,
    world::world_benches,
);

type BenchGroup<'a> = BenchmarkGroup<'a, criterion::measurement::WallTime>;

pub fn generic_bench<P: Copy>(
    bench_group: &mut BenchmarkGroup<criterion::measurement::WallTime>,
    mut benches: Vec<Box<dyn FnMut(&mut BenchGroup, P)>>,
    bench_parameters: P,
) {
    for b in &mut benches {
        b(bench_group, bench_parameters);
    }
}
