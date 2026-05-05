use criterion::criterion_main;

mod compute_normals;
mod extract_render_asset;
mod render_layers;
mod torus;

criterion_main!(
    render_layers::benches,
    compute_normals::benches,
    torus::benches,
    extract_render_asset::benches
);
