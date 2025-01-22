use criterion::criterion_main;

mod render_layers;
mod torus;

criterion_main!(render_layers::benches, torus::benches);
