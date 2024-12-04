#![expect(dead_code, reason = "Many fields are unused/unread as they are just for benchmarking purposes.")]

use criterion::criterion_main;

mod change_detection;
mod components;
mod empty_archetypes;
mod events;
mod fragmentation;
mod iteration;
mod observers;
mod param;
mod scheduling;
mod world;

criterion_main!(
    change_detection::change_detection_benches,
    components::components_benches,
    empty_archetypes::empty_archetypes_benches,
    events::event_benches,
    iteration::iterations_benches,
    fragmentation::fragmentation_benches,
    observers::observer_benches,
    scheduling::scheduling_benches,
    world::world_benches,
    param::param_benches,
);
