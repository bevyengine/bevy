use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bevy_math::{prelude::*, *};

fn easing(c: &mut Criterion) {
    let cubic_bezier = CubicSegment::new_bezier(vec2(0.25, 0.1), vec2(0.25, 1.0));
    c.bench_function("easing_1000", |b| {
        b.iter(|| {
            (0..1000).map(|i| i as f32 / 1000.0).for_each(|t| {
                black_box(cubic_bezier.ease(black_box(t)));
            })
        });
    });
}

fn cubic_2d(c: &mut Criterion) {
    let bezier = CubicBezier::new([[
        vec2(0.0, 0.0),
        vec2(0.0, 1.0),
        vec2(1.0, 0.0),
        vec2(1.0, 1.0),
    ]])
    .to_curve();
    c.bench_function("cubic_position_Vec2", |b| {
        b.iter(|| black_box(bezier.position(black_box(0.5))));
    });
}

fn cubic(c: &mut Criterion) {
    let bezier = CubicBezier::new([[
        vec3a(0.0, 0.0, 0.0),
        vec3a(0.0, 1.0, 0.0),
        vec3a(1.0, 0.0, 0.0),
        vec3a(1.0, 1.0, 1.0),
    ]])
    .to_curve();
    c.bench_function("cubic_position_Vec3A", |b| {
        b.iter(|| black_box(bezier.position(black_box(0.5))));
    });
}

fn cubic_vec3(c: &mut Criterion) {
    let bezier = CubicBezier::new([[
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(1.0, 1.0, 1.0),
    ]])
    .to_curve();
    c.bench_function("cubic_position_Vec3", |b| {
        b.iter(|| black_box(bezier.position(black_box(0.5))));
    });
}

fn build_pos_cubic(c: &mut Criterion) {
    let bezier = CubicBezier::new([[
        vec3a(0.0, 0.0, 0.0),
        vec3a(0.0, 1.0, 0.0),
        vec3a(1.0, 0.0, 0.0),
        vec3a(1.0, 1.0, 1.0),
    ]])
    .to_curve();
    c.bench_function("build_pos_cubic_100_points", |b| {
        b.iter(|| black_box(bezier.iter_positions(black_box(100)).collect::<Vec<_>>()));
    });
}

fn build_accel_cubic(c: &mut Criterion) {
    let bezier = CubicBezier::new([[
        vec3a(0.0, 0.0, 0.0),
        vec3a(0.0, 1.0, 0.0),
        vec3a(1.0, 0.0, 0.0),
        vec3a(1.0, 1.0, 1.0),
    ]])
    .to_curve();
    c.bench_function("build_accel_cubic_100_points", |b| {
        b.iter(|| black_box(bezier.iter_positions(black_box(100)).collect::<Vec<_>>()));
    });
}

criterion_group!(
    benches,
    easing,
    cubic_2d,
    cubic_vec3,
    cubic,
    build_pos_cubic,
    build_accel_cubic,
);
criterion_main!(benches);
