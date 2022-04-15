/// Fast approximated reciprocal square root.
#[inline]
pub(crate) fn approx_rsqrt(x: f32) -> f32 {
    // Fall back to Quake 3 fast inverse sqrt, is has a higher error
    // but still good enough and faster than `.sqrt().recip()`,
    // implementation borrowed from Piston under the MIT License:
    // [https://github.com/PistonDevelopers/skeletal_animation]
    let x2: f32 = x * 0.5;
    let mut y: f32 = x;

    let mut i: i32 = y.to_bits() as i32;
    i = 0x5f3759df - (i >> 1);
    y = f32::from_bits(i as u32);

    y = y * (1.5 - (x2 * y * y));
    y
}

/// Steps between two different discrete values of any clonable type.
/// Returns a copy of `b` if `t >= 1.0`, otherwise returns a copy of `b`.
#[inline]
pub(crate) fn step_unclamped<T>(a: T, b: T, t: f32) -> T {
    if t >= 1.0 {
        a
    } else {
        b
    }
}
