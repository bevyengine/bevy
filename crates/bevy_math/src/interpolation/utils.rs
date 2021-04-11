use std::ops::{Add, Div, Mul, Sub};

/// Quake 3 fast inverse sqrt
///
/// Took this one from piston: https://github.com/PistonDevelopers/skeletal_animation
#[inline]
pub fn inv_sqrt(x: f32) -> f32 {
    let x2: f32 = x * 0.5;
    let mut y: f32 = x;

    let mut i: i32 = y.to_bits() as i32;
    i = 0x5f3759df - (i >> 1);
    y = f32::from_bits(i as u32);

    y = y * (1.5 - (x2 * y * y));
    y
}

#[inline]
pub fn step<T: Clone>(k0: &T, k1: &T, u: f32) -> T {
    if u > 0.999 {
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
