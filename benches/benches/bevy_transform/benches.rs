use criterion::criterion_main;

mod transform_hierarchy;

criterion_main!(
    transform_hierarchy::transform_hierarchy_benches,
);
