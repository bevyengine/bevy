use criterion::criterion_main;

mod components;
mod events;
mod fragmentation;
mod iteration;
mod observers;
mod param;
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
    param::param_benches,
);
