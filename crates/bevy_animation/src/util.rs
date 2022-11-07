/// Steps between two different discrete values of any clonable type.
/// Returns a copy of `b` if `t >= 1.0`, otherwise returns a copy of `a`.
#[inline]
pub(crate) fn step_unclamped<T>(a: T, b: T, t: f32) -> T {
    if t >= 1.0 {
        a
    } else {
        b
    }
}
