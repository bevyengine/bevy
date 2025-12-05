use criterion::criterion_main;

mod compute_normals;
mod render_layers;
mod torus;

criterion_main!(
    render_layers::benches,
    compute_normals::benches,
    torus::benches
);
