<<<<<<< HEAD
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bevy_render::mesh::TorusMeshBuilder;

fn torus(c: &mut Criterion) {
    c.bench_function("build_torus", |b| {
        b.iter(|| black_box(TorusMeshBuilder::new(black_box(0.5),black_box(1.0))));
    });
}

criterion_group!(
    benches,
    torus,
);
criterion_main!(benches);
=======
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bevy_render::mesh::TorusMeshBuilder;

fn torus(c: &mut Criterion) {
    c.bench_function("build_torus", |b| {
        b.iter(|| black_box(TorusMeshBuilder::new(black_box(0.5),black_box(1.0))));
    });
}

criterion_group!(
    benches,
    torus,
);
criterion_main!(benches);
>>>>>>> c421e29b718822d12efc067456c4c13ea2719da1
