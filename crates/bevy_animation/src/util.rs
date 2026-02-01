/// Steps between two different discrete values of any type.
/// Returns `a` if `t < 1.0`, otherwise returns `b`.
#[inline]
pub(crate) fn step_unclamped<T>(a: T, b: T, t: f32) -> T {
    if t < 1.0 {
        a
    } else {
        b
    }
}
