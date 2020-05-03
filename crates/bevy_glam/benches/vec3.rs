#[path = "support/macros.rs"]
#[macro_use]
mod macros;
mod support;

use criterion::{criterion_group, criterion_main, Criterion};
use glam::f32::Vec3;
use std::ops::Mul;
use support::{random_mat3, random_quat, random_vec3};

bench_binop!(
    quat_mul_vec3,
    "quat * vec3",
    op => mul,
    from1 => random_quat,
    from2 => random_vec3
);

bench_binop!(
    mat3_mul_vec3,
    "mat3 * vec3",
    op => mul,
    from1 => random_mat3,
    from2 => random_vec3
);

#[inline]
fn vec3_to_rgb_op(v: Vec3) -> u32 {
    let (red, green, blue) = (v.min(Vec3::one()).max(Vec3::zero()) * 255.0).into();
    ((red as u32) << 16 | (green as u32) << 8 | (blue as u32)).into()
}

#[inline]
fn vec3_accessors(v: Vec3) -> [f32; 3] {
    [v.x(), v.y(), v.z()]
}

#[inline]
fn vec3_into_array(v: Vec3) -> [f32; 3] {
    v.into()
}

#[inline]
fn vec3_into_tuple(v: Vec3) -> (f32, f32, f32) {
    v.into()
}

bench_func!(
    vec3_to_rgb,
    "vec3 to rgb",
    op => vec3_to_rgb_op,
    from => random_vec3
    );

bench_func!(
    vec3_to_array_accessors,
    "vec3 into array slow",
    op => vec3_accessors,
    from => random_vec3
    );

bench_func!(
    vec3_to_array_into,
    "vec3 into array fast",
    op => vec3_into_array,
    from => random_vec3
    );

bench_func!(
    vec3_to_tuple_into,
    "vec3 into tuple fast",
    op => vec3_into_tuple,
    from => random_vec3
    );

euler!(vec3_euler, "vec3 euler", ty => Vec3, storage => Vec3, zero => Vec3::zero(), rand => random_vec3);

bench_binop!(
    vec3_angle_between,
    "vec3 angle_between",
    op => angle_between,
    from1 => random_vec3,
    from2 => random_vec3
);

criterion_group!(
    benches,
    quat_mul_vec3,
    mat3_mul_vec3,
    vec3_angle_between,
    vec3_euler,
    vec3_to_rgb,
    vec3_to_array_accessors,
    vec3_to_array_into,
    vec3_to_tuple_into,
);

criterion_main!(benches);
