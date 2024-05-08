#[cfg(test)]
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

#[cfg(test)]
pub(crate) use assert_approx_eq;

#[cfg(test)]
macro_rules! call_for_every_color {
    ($m:ident) => {
        $m::<crate::Hsla>();
        $m::<crate::Hsva>();
        $m::<crate::Hwba>();
        $m::<crate::Laba>();
        $m::<crate::Lcha>();
        $m::<crate::LinearRgba>();
        $m::<crate::Oklaba>();
        $m::<crate::Oklcha>();
        $m::<crate::Xyza>();
    };
}

#[cfg(test)]
pub(crate) use call_for_every_color;
