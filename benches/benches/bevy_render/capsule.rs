use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bevy_render::mesh::Capsule3dMeshBuilder;

criterion_group!(benches, capsule_low_res, capsule_default, capsule_high_res);
fn capsule_low_res(c: &mut Criterion) {
    c.bench_function("build_capsule_low_res", |b| {
        b.iter(|| {
            black_box(
                Capsule3dMeshBuilder::new(
                    black_box(0.5),
                    black_box(1.0),
                    black_box(16),
                    black_box(4),
                )
                .build(),
            )
        });
    });
}
fn capsule_default(c: &mut Criterion) {
    c.bench_function("build_capsule_default", |b| {
        b.iter(|| {
            black_box(
                Capsule3dMeshBuilder::new(
                    black_box(0.5),
                    black_box(1.0),
                    black_box(32),
                    black_box(8),
                )
                .build(),
            )
        });
    });
}

fn capsule_high_res(c: &mut Criterion) {
    c.bench_function("build_capsule_high_res", |b| {
        b.iter(|| {
            black_box(
                Capsule3dMeshBuilder::new(
                    black_box(0.5),
                    black_box(1.0),
                    black_box(64),
                    black_box(16),
                )
                .build(),
            )
        });
    });
}
criterion_main!(benches);
