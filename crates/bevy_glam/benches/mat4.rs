#[path = "support/macros.rs"]
#[macro_use]
mod macros;
mod support;

use criterion::{criterion_group, criterion_main, Criterion};
use glam::Mat4;
use std::ops::Mul;
use support::*;

bench_unop!(
    mat4_transpose,
    "mat4 transpose",
    op => transpose,
    from => random_srt_mat4
);
bench_unop!(
    mat4_determinant,
    "mat4 determinant",
    op => determinant,
    from => random_srt_mat4
);
bench_unop!(mat4_inverse, "mat4 inverse", op => inverse, from => random_srt_mat4);
bench_binop!(mat4_mul_mat4, "mat4 * mat4", op => mul, from => random_srt_mat4);
bench_from_ypr!(mat4_from_ypr, "mat4 from ypr", ty => Mat4);

criterion_group!(
    benches,
    mat4_transpose,
    mat4_determinant,
    mat4_inverse,
    mat4_mul_mat4,
    mat4_from_ypr,
);

criterion_main!(benches);
