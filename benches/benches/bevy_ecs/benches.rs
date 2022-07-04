use criterion::criterion_main;

mod components;
mod iteration;
mod stages;
mod systems;
mod world;

criterion_main!(
    iteration::iterations_benches,
    components::components_benches,
    stages::stages_benches,
    systems::systems_benches,
    world::world_benches,
);
