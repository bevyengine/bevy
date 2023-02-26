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
    let bezier = Bezier::<Vec3A, 16>::new([
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ]);
    c.bench_function("fifteen_degree_position", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn quadratic_2d(c: &mut Criterion) {
    let bezier = QuadraticBezier2d::new([[0.0, 0.0], [0.0, 1.0], [1.0, 1.0]]);
    c.bench_function("quadratic_position_Vec2", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn quadratic(c: &mut Criterion) {
    let bezier = QuadraticBezier3d::new([[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 1.0]]);
    c.bench_function("quadratic_position_Vec3A", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn quadratic_vec3(c: &mut Criterion) {
    let bezier = Bezier::<Vec3, 3>::new([[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 1.0]]);
    c.bench_function("quadratic_position_Vec3", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn cubic_2d(c: &mut Criterion) {
    let bezier = CubicBezier2d::new([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
    c.bench_function("cubic_position_Vec2", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn cubic(c: &mut Criterion) {
    let bezier = CubicBezier3d::new([
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ]);
    c.bench_function("cubic_position_Vec3A", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn cubic_vec3(c: &mut Criterion) {
    let bezier = Bezier::<Vec3, 4>::new([
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ]);
    c.bench_function("cubic_position_Vec3", |b| {
        b.iter(|| bezier.position(black_box(0.5)));
    });
}

fn build_pos_cubic(c: &mut Criterion) {
    let bezier = CubicBezier3d::new([
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ]);
    c.bench_function("build_pos_cubic_100_points", |b| {
        b.iter(|| bezier.iter_positions(black_box(100)).collect::<Vec<_>>());
    });
}

fn build_accel_cubic(c: &mut Criterion) {
    let bezier = CubicBezier3d::new([
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ]);
    c.bench_function("build_accel_cubic_100_points", |b| {
        b.iter(|| bezier.iter_positions(black_box(100)).collect::<Vec<_>>());
    });
}

criterion_group!(
    benches,
    easing,
    fifteen_degree,
    quadratic_2d,
    quadratic,
    quadratic_vec3,
    cubic_2d,
    cubic,
    cubic_vec3,
    build_pos_cubic,
    build_accel_cubic,
);
criterion_main!(benches);
