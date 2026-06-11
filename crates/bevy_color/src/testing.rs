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

macro_rules! assert_approx_less {
    ($x:expr, $y:expr, $d:expr) => {
        if $x > $y + $d {
            panic!(
                "assertion failed: `(left > right)` \
                 (left: `{}`, right: `{}`, tolerance: `{}`)",
                $x, $y, $d
            );
        }
    };
}

#[cfg(test)]
pub(crate) use assert_approx_eq;

#[cfg(test)]
pub(crate) use assert_approx_less;
