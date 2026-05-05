#[cfg(test)]
macro_rules! assert_approx_eq {
    ($x:expr, $y:expr, $d:expr) => {
        if ($x - $y).abs() >= $d {
            panic!(
                "assertion failed: `(left !== right)` \
                 (left: `{}`, right: `{}`, tolerance: `{}`)",
                $x, $y, $d
            );
        }
    };

    ($x:expr, $y:expr, $d:expr, $msg:expr) => {
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
