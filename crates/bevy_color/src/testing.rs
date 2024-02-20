#[allow(unused_macros)]
macro_rules! assert_approx_eq {
    ($x:expr, $y:expr, $d:expr) => {
        if ($x - $y).abs() >= $d {
            panic!(
                "assertion failed: `(left !== right)` \
                 (left: `{:?}`, right: `{:?}`, tolerance: `{:?}`)",
                $x, $y, $d
            );
        }
    };
}

#[allow(unused_imports)]
pub(crate) use assert_approx_eq;
