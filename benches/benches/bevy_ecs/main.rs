#![expect(
    dead_code,
    reason = "Many fields are unused/unread as they are just for benchmarking purposes."
)]
#![expect(clippy::type_complexity)]

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
    change_detection::benches,
    components::benches,
    empty_archetypes::benches,
    events::benches,
    iteration::benches,
    fragmentation::benches,
    observers::benches,
    scheduling::benches,
    world::benches,
    param::benches,
);
