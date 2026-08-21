#[cfg(test)]
macro_rules! assert_approx_eq {
    ($x:expr, $y:expr, $d:expr) => {
        assert!(!f32::is_nan($x));
        assert!(!f32::is_nan($y));
        if ($x - $y).abs() >= $d {
            panic!(
                "assertion failed: `(left !== right)` \
                 (left: `{}`, right: `{}`, tolerance: `{}`)",
                $x, $y, $d
            );
        }
    };

    ($x:expr, $y:expr, $d:expr, $msg:expr) => {
        assert!(!f32::is_nan($x));
        assert!(!f32::is_nan($y));
        if ($x - $y).abs() >= $d {
            panic!(
                "assertion failed: `(left !== right)` \
                 (left: `{}`, right: `{}`, tolerance: `{}`). {}",
                $x, $y, $d, $msg
            );
        }
    };
}

#[cfg(test)]
pub(crate) use assert_approx_eq;

#[cfg(test)]
pub(crate) fn assert_mat3_approx_eq(a: bevy_math::Mat3, b: bevy_math::Mat3, tolerance: f32) {
    for col in 0..3 {
        for row in 0..3 {
            assert_approx_eq!(
                a.col(col)[row],
                b.col(col)[row],
                tolerance,
                alloc::format!("matrices differ at column {col}, row {row}: {a} != {b}")
            );
        }
    }
}
