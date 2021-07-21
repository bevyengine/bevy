use std::ops::{Add, Div, Mul, Sub};

/// Fast approximated reciprocal square root
#[inline]
pub fn approx_rsqrt(x: f32) -> f32 {
    #[cfg(target_feature = "sse")]
    {
        // use SEE _mm_rsqrt_ss intrinsic which has a better accuracy and
        #[cfg(target_arch = "x86")]
        use core::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::*;
        unsafe {
            let y = _mm_rsqrt_ss(_mm_set1_ps(x));
            *(&y as *const _ as *const f32)
        }
    }
    #[cfg(not(target_feature = "sse"))]
    {
        // Fall back to Quake 3 fast inverse sqrt, is has a higher error but still good enough and faster than `.sqrt().recip()`,
        // implementation borrowed from Piston under the MIT License: [https://github.com/PistonDevelopers/skeletal_animation]
        let x2: f32 = x * 0.5;
        let mut y: f32 = x;

        let mut i: i32 = y.to_bits() as i32;
        i = 0x5f3759df - (i >> 1);
        y = f32::from_bits(i as u32);

        y = y * (1.5 - (x2 * y * y));
        y
    }
}

#[inline]
pub fn step_unclamped<T: Clone>(value0: &T, value1: &T, u: f32) -> T {
    if u < (1.0 - 1e-9) {
        value0.clone()
    } else {
        value1.clone()
    }
}

#[inline]
pub fn lerp_unclamped<T>(value0: T, value1: T, u: f32) -> T
where
    T: Add<Output = T> + Mul<f32, Output = T>,
{
    value0 * (1.0 - u) + value1 * u
}

/// Performs the cubic hermite spline interpolation based on the factor `u` whiting the 0 to 1 range.
/// The curve shape is defined by the keyframes values, tangents and by the delta time between the keyframes.
///
/// Source: http://archive.gamedev.net/archive/reference/articles/article1497.html
#[inline]
pub fn hermite_unclamped<T>(
    value0: T,
    tangent0: T,
    value1: T,
    tangent1: T,
    u: f32,
    delta_time: f32,
) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>,
{
    let u2 = u * u;
    let u3 = u2 * u;
    let _3u2 = 3.0 * u2;
    let _2u3 = 2.0 * u3;

    value0 * (_2u3 - _3u2 + 1.0)
        + value1 * (_3u2 - _2u3)
        + tangent0 * delta_time * (u3 - 2.0 * u2 + u)
        + tangent1 * delta_time * (u3 - u2)
}

/// Finds the tangent gradients for `k1` the hermite spline, takes the a keyframe value his point in time
/// as well as the surrounding keyframes values and time stamps.
///
/// Source: http://archive.gamedev.net/archive/reference/articles/article1497.html
#[inline]
pub fn auto_tangent<T>(time0: f32, time1: f32, time2: f32, value0: T, value1: T, value2: T) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T> + Div<f32, Output = T>,
{
    // k'(t) = ½[k(t) - k(t-1)]/δx1 + ½[k(t+1) - k(t)]/δx2
    ((value1 - value0) / (time1 - time0).max(1e-9) + (value2 - value1) / (time2 - time1).max(1e-9))
        * 0.5
}

// https://www.cubic.org/docs/hermite.htm
