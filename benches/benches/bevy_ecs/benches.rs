use criterion::criterion_main;

mod components;
mod iterations;
mod stages;
mod systems;
mod world;

criterion_main!(
    iterations::iterations_benches,
    components::components_benches,
    stages::stages_benches,
    systems::systems_benches,
    world::world_benches,
);
