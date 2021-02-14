use std::ops::{Add, Div, Mul, Sub};

use bevy_math::{Vec2, Vec3};
use ultraviolet::{f32x4, f32x8, Vec4x4, Vec4x8};
use wide::{i32x4, i32x8};

// TODO: Add Quat support to ultraviolet?

/// Wide quaternion
#[derive(Debug, Clone)]
pub struct Quatx4(pub Vec4x4);

/// Wide quaternion
#[derive(Debug, Clone)]
pub struct Quatx8(pub Vec4x8);

// macro_rules! wide_quat {
//     ($t:tt, $) => {
//         impl $t {
//             pub fn identity() -> $t {
//                 $t::splat(Vec4::new(0.0, 0.0, 0.0, 0.1))
//             }
//         }
//     };
// }

// wide_quat!(Quat4);
// wide_quat!(Quat8);

/// Used to change the blending semantics of a `Vec2`
#[derive(Debug, Clone)]
pub struct Scale2(pub Vec2);

/// Used to change the blending semantics of a `Vec3`
#[derive(Debug, Clone)]
pub struct Scale3(pub Vec3);

/// Quake 3 fast inverse sqrt
///
/// Took this one from piston: https://github.com/PistonDevelopers/skeletal_animation
#[inline]
pub fn inv_sqrt(x: f32) -> f32 {
    let x2: f32 = x * 0.5;
    let mut y: f32 = x;

    let mut i: i32 = y.to_bits() as i32;
    i = 0x5f3759df - (i >> 1);
    y = unsafe { std::mem::transmute(i) };

    y = y * (1.5 - (x2 * y * y));
    y
}

/// Quake 3 fast inverse sqrt for `f32x4`
#[inline]
pub fn inv_sqrt4(x: f32x4) -> f32x4 {
    let x2: f32x4 = x * 0.5;
    let mut y: f32x4 = x;

    let mut i: i32x4 = unsafe { std::mem::transmute(y) };
    i = i32x4::splat(0x5f3759df) - (i >> 1);
    y = unsafe { std::mem::transmute(i) };

    y = y * (f32x4::splat(1.5) - (x2 * y * y));
    y
}

/// Quake 3 fast inverse sqrt for `f32x8`
#[inline]
pub fn inv_sqrt8(x: f32x8) -> f32x8 {
    let x2: f32x8 = x * 0.5;
    let mut y: f32x8 = x;

    let mut i: i32x8 = unsafe { std::mem::transmute(y) };
    i = i32x8::splat(0x5f3759df) - (i >> 1);
    y = unsafe { std::mem::transmute(i) };

    y = y * (f32x8::splat(1.5) - (x2 * y * y));
    y
}

#[inline]
pub fn step<T: Clone>(k0: &T, k1: &T, u: f32) -> T {
    if u > 0.99 {
        k0.clone()
    } else {
        k1.clone()
    }
}

#[inline]
pub fn lerp<T>(k0: T, k1: T, u: f32) -> T
where
    T: Add<Output = T> + Mul<f32, Output = T>,
{
    k0 * (1.0 - u) + k1 * u
}

/// Catmull-Rom spline interpolation
///
/// Source: http://archive.gamedev.net/archive/reference/articles/article1497.html
#[inline]
pub fn catmull_rom<T>(k0: T, t0: T, k1: T, t1: T, u: f32) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>,
{
    let u2 = u * u;
    let u3 = u2 * u;
    let _3u2 = 3.0 * u2;
    let _2u3 = 2.0 * u3;

    k0 * (_2u3 - _3u2 + 1.0) + k1 * (_3u2 - _2u3) + t0 * (u3 - 2.0 * u2 + u) + t1 * (u3 - u2)
}

/// Finds the tangent gradients for the Catmull-Rom spline
///
/// Source: http://archive.gamedev.net/archive/reference/articles/article1497.html
#[inline]
pub fn auto_tangent<T>(t0: f32, t1: f32, t2: f32, k0: T, k1: T, k2: T) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T> + Div<f32, Output = T>,
{
    // k'(t) = ½[k(t) - k(t-1)]/δx1 + ½[k(t+1) - k(t)]/δx2
    ((k1 - k0) / (t1 - t0) + (k2 - k1) / (t2 - t1)) * 0.5
}
