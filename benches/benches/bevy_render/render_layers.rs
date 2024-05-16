use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bevy_render::view::RenderLayers;

fn render_layers(c: &mut Criterion) {
    c.bench_function("layers_intersect", |b| {
        let layer_a = RenderLayers::layer(1).with(2);
        let layer_b = RenderLayers::layer(1);
        b.iter(|| {
            black_box(layer_a.intersects(&layer_b))
        });
    });
}

criterion_group!(
    benches,
    render_layers,
);
criterion_main!(benches);
