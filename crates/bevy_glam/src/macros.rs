#[cfg(any(
    all(debug_assertions, feature = "debug-glam-assert"),
    feature = "glam-assert"
))]
macro_rules! glam_assert {
    ($($arg:tt)*) => ( assert!($($arg)*); )
}
#[cfg(not(any(
    all(debug_assertions, feature = "debug-glam-assert"),
    feature = "glam-assert"
)))]
macro_rules! glam_assert {
    ($($arg:tt)*) => {};
}

macro_rules! is_normalized {
    ($self:expr, $max_diff:expr) => {
        ($self.length_squared() - 1.0).abs() <= $max_diff
    };
    ($self:expr) => {
        is_normalized!($self, 1e-6)
    };
}

macro_rules! abs_diff_eq {
    ($self:expr, $other:expr, $max_abs_diff:expr) => {
        ($self - $other)
            .abs()
            .cmple(Self::splat($max_abs_diff))
            .all()
    };
}
