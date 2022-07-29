use criterion::criterion_main;

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
