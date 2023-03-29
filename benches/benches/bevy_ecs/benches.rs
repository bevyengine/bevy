use criterion::criterion_main;

mod components;
mod iteration;
mod scheduling;
mod events;
mod world;

criterion_main!(
    components::components_benches,
    events::event_benches,
    iteration::iterations_benches,
    scheduling::scheduling_benches,
    world::world_benches,
);
