use std::ops::{Add, Div, Mul, Sub};

/// Quake 3 fast inverse sqrt
///
/// Implementation borrowed from Piston under the MIT License: https://github.com/PistonDevelopers/skeletal_animation
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
pub fn step_unclamped<T: Clone>(k0: &T, k1: &T, u: f32) -> T {
    if u < (1.0 - 1e-9) {
        k0.clone()
    } else {
        k1.clone()
    }
}

#[inline]
pub fn lerp_unclamped<T>(k0: T, k1: T, u: f32) -> T
where
    T: Add<Output = T> + Mul<f32, Output = T>,
{
    k0 * (1.0 - u) + k1 * u
}

/// Cubic hermite spline
///
/// Source: http://archive.gamedev.net/archive/reference/articles/article1497.html
#[inline]
pub fn hermite_unclamped<T>(k0: T, t0: T, k1: T, t1: T, u: f32, dx: f32) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>,
{
    let v_u2 = u * u;
    let v_u3 = v_u2 * u;
    let v_3u2 = 3.0 * v_u2;
    let v_2u3 = 2.0 * v_u3;

    k0 * (v_2u3 - v_3u2 + 1.0)
        + k1 * (v_3u2 - v_2u3)
        + t0 * dx * (v_u3 - 2.0 * v_u2 + u)
        + t1 * dx * (v_u3 - v_u2)
}

/// Finds the tangent gradients for the hermite spline
///
/// Source: http://archive.gamedev.net/archive/reference/articles/article1497.html
#[inline]
pub fn auto_tangent<T>(t0: f32, t1: f32, t2: f32, k0: T, k1: T, k2: T) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T> + Div<f32, Output = T>,
{
    // k'(t) = ½[k(t) - k(t-1)]/δx1 + ½[k(t+1) - k(t)]/δx2
    ((k1 - k0) / (t1 - t0).max(1e-9) + (k2 - k1) / (t2 - t1).max(1e-9)) * 0.5
}

// https://www.cubic.org/docs/hermite.htm
