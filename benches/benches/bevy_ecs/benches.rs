use criterion::{criterion_main, BenchmarkGroup};

mod components;
mod events;
mod fragmentation;
mod iteration;
mod observers;
mod scheduling;
mod world;

criterion_main!(
    components::components_benches,
    events::event_benches,
    iteration::iterations_benches,
    fragmentation::fragmentation_benches,
    observers::observer_benches,
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
