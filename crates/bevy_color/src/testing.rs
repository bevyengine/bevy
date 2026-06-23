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
