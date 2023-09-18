use criterion::criterion_main;

mod components;
mod events;
mod iteration;
mod iteration_dynamic;
mod scheduling;
mod world;

criterion_main!(
    components::components_benches,
    events::event_benches,
    iteration::iterations_benches,
    iteration_dynamic::iterations_benches,
    scheduling::scheduling_benches,
    world::world_benches,
);
