use criterion::criterion_main;

mod bezier;
mod bounding;
mod ray_cast_2d;
mod ray_cast_3d;

criterion_main!(
    bezier::benches,
    bounding::benches,
    ray_cast_2d::benches,
    ray_cast_3d::benches
);
