mod conversions;
mod hsla;
mod linear_srgba;
mod srgba;

pub use hsla::*;
pub use linear_srgba::*;
pub use srgba::*;

pub type Color = LinSrgba;

/// Creates a new [`Srgba`] color
#[macro_export]
macro_rules! srgba {
    ($r:expr, $g:expr, $b:expr) => {
        Srgba::new($r, $g, $b)
    };
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        Srgba::with_alpha($r, $g, $b, $a)
    };
}

/// Creates a new [`Hsla`] color
#[macro_export]
macro_rules! hsla {
    ($h:expr, $s:expr, $l:expr) => {
        Hsla::new($h, $s, $l)
    };
    ($h:expr, $s:expr, $l:expr, $a:expr) => {
        Hsla::with_alpha($h, $s, $l, $a)
    };
}

/// Creates a new [`LinSrgba`] color
#[macro_export]
macro_rules! rgba {
    ($r:expr, $g:expr, $b:expr) => {
        LinSrgba::new($r, $g, $b)
    };
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        LinSrgba::with_alpha($r, $g, $b, $a)
    };
}

#[test]
fn test_macros() {
    assert_eq!(rgba!(0.5, 0.5, 0.5), rgba!(0.5, 0.5, 0.5, 1.0));
    assert_eq!(srgba!(0.5, 0.5, 0.5), srgba!(0.5, 0.5, 0.5, 1.0));
    assert_eq!(hsla!(0.0, 0.0, 1.0), hsla!(0.0, 0.0, 1.0, 1.0));
}
