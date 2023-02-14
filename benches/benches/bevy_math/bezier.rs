use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bevy_math::*;

fn easing(c: &mut Criterion) {
    let cubic_bezier = CubicBezierEasing::new(vec2(0.25, 0.1), vec2(0.25, 1.0));
    c.bench_function("easing_1000", |b| {
        b.iter(|| {
            (0..1000).map(|i| i as f32 / 1000.0).for_each(|t| {
                cubic_bezier.ease(black_box(t));
            })
        });
    });
}

fn fifteen_degree(c: &mut Criterion) {
    let bezier = Bezier::new([
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(1.0, 1.0, 1.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(1.0, 1.0, 1.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(1.0, 1.0, 1.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(1.0, 1.0, 1.0),
    ]);
    c.bench_function("fifteen_degree_position", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn quadratic(c: &mut Criterion) {
    let bezier = QuadraticBezier3d::new([
        vec3a(0.0, 0.0, 0.0),
        vec3a(0.0, 1.0, 0.0),
        vec3a(1.0, 1.0, 1.0),
    ]);
    c.bench_function("quadratic_position", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn quadratic_vec3(c: &mut Criterion) {
    let bezier = Bezier::new([
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 1.0, 1.0),
    ]);
    c.bench_function("quadratic_position_Vec3", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn cubic(c: &mut Criterion) {
    let bezier = CubicBezier3d::new([
        vec3a(0.0, 0.0, 0.0),
        vec3a(0.0, 1.0, 0.0),
        vec3a(1.0, 0.0, 0.0),
        vec3a(1.0, 1.0, 1.0),
    ]);
    c.bench_function("cubic_position", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn cubic_vec3(c: &mut Criterion) {
    let bezier = Bezier::new([
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(1.0, 1.0, 1.0),
    ]);
    c.bench_function("cubic_position_Vec3", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

criterion_group!(
    benches,
    easing,
    fifteen_degree,
    quadratic,
    quadratic_vec3,
    cubic,
    cubic_vec3
);
criterion_main!(benches);
