#![expect(
    dead_code,
    reason = "Many fields are unused/unread as they are just for benchmarking purposes."
)]

use criterion::criterion_main;

mod bundles;
mod change_detection;
mod components;
mod empty_archetypes;
mod entity_cloning;
mod events;
mod fragmentation;
mod iteration;
mod observers;
mod param;
mod scheduling;
mod world;

criterion_main!(
    bundles::benches,
    change_detection::benches,
    components::benches,
    empty_archetypes::benches,
    entity_cloning::benches,
    events::benches,
    iteration::benches,
    fragmentation::benches,
    observers::benches,
    scheduling::benches,
    world::benches,
    param::benches,
);
