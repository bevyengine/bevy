use criterion::criterion_main;

mod components;
mod events;
mod iteration;
mod scheduling;
mod world;

criterion_main!(
    components::components_benches,
    events::event_benches,
    iteration::iterations_benches,
    scheduling::scheduling_benches,
    world::world_benches,
);
