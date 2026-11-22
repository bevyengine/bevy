use criterion::criterion_main;

mod large_world;
mod propagate;

criterion_main!(propagate::benches, large_world::benches);
