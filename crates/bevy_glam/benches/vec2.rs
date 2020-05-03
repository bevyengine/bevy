#[path = "support/macros.rs"]
#[macro_use]
mod macros;
mod support;

use criterion::{criterion_group, criterion_main, Criterion};
use glam::f32::Vec2;
use std::ops::Mul;
use support::{random_mat2, random_vec2};

euler!(vec2_euler, "vec2 euler", ty => Vec2, storage => Vec2, zero => Vec2::zero(), rand => random_vec2);

bench_binop!(
    mat2_mul_vec2,
    "mat2 * vec2",
    op => mul,
    from1 => random_mat2,
    from2 => random_vec2
);

bench_binop!(
    vec2_angle_between,
    "vec2 angle_between",
    op => angle_between,
    from1 => random_vec2,
    from2 => random_vec2
);

criterion_group!(benches, vec2_euler, mat2_mul_vec2, vec2_angle_between);

criterion_main!(benches);
