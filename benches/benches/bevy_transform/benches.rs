use criterion::criterion_main;

mod hierarchy;

mod world_gen;

criterion_main!(
    hierarchy::init::transform_hierarchy_init,
    hierarchy::propagation::transform_hierarchy_configurations,
    hierarchy::propagation::transform_hierarchy_sizes,
);
