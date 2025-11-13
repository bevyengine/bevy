use core::hint::black_box;

use criterion::{criterion_group, Criterion};

use bevy_mesh::TorusMeshBuilder;

fn torus(c: &mut Criterion) {
    c.bench_function("build_torus", |b| {
        b.iter(|| black_box(TorusMeshBuilder::new(black_box(0.5), black_box(1.0))));
    });
}

criterion_group!(benches, torus);
