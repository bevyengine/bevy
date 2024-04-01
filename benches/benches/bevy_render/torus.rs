use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bevy_render::mesh::TorusMeshBuilder;

criterion_group!(benches, torus_low_res, torus_default, torus_high_res,);

fn torus_low_res(c: &mut Criterion) {
    c.bench_function("build_torus_low_res", |b| {
        b.iter(|| {
            black_box(
                TorusMeshBuilder::new(black_box(0.5), black_box(1.0))
                    .minor_resolution(black_box(12))
                    .major_resolution(black_box(16))
                    .build(),
            )
        });
    });
}

fn torus_default(c: &mut Criterion) {
    c.bench_function("build_torus_default", |b| {
        b.iter(|| black_box(TorusMeshBuilder::new(black_box(0.5), black_box(1.0)).build()));
    });
}

fn torus_high_res(c: &mut Criterion) {
    c.bench_function("build_torus_high_res", |b| {
        b.iter(|| {
            black_box(
                TorusMeshBuilder::new(black_box(0.5), black_box(1.0))
                    .minor_resolution(black_box(48))
                    .major_resolution(black_box(64))
                    .build(),
            )
        });
    });
}

criterion_main!(benches);
