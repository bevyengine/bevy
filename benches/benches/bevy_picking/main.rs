use criterion::criterion_main;

mod ray_mesh_intersection;

criterion_main!(ray_mesh_intersection::benches);
