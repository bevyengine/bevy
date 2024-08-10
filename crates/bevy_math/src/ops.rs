//! This mod re-exports the correct versions of floating-point operations with
//! unspecified precision in the standard library depending on whether the `libm`
//! crate feature is enabled.
//!
//! All the functions here are named according to their versions in the standard
//! library.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::disallowed_methods)]

// Note: There are some Rust methods with unspecified precision without a `libm`
// equivalent:
// - `f32::powi` (integer powers)
// - `f32::log` (logarithm with specified base)
// - `f32::abs_sub` (actually unsure if `libm` has this, but don't use it regardless)
//
// Additionally, the following nightly API functions are not presently integrated
// into this, but they would be candidates once standardized:
// - `f32::gamma`
// - `f32::ln_gamma`

#[cfg(not(feature = "libm"))]
mod std_ops {
    #[inline(always)]
    pub(crate) fn powf(x: f32, y: f32) -> f32 {
        f32::powf(x, y)
    }

    #[inline(always)]
    pub(crate) fn exp(x: f32) -> f32 {
        f32::exp(x)
    }

    #[inline(always)]
    pub(crate) fn exp2(x: f32) -> f32 {
        f32::exp2(x)
    }

    #[inline(always)]
    pub(crate) fn ln(x: f32) -> f32 {
        f32::ln(x)
    }

    #[inline(always)]
    pub(crate) fn log2(x: f32) -> f32 {
        f32::log2(x)
    }

    #[inline(always)]
    pub(crate) fn log10(x: f32) -> f32 {
        f32::log10(x)
    }

    #[inline(always)]
    pub(crate) fn cbrt(x: f32) -> f32 {
        f32::cbrt(x)
    }

    #[inline(always)]
    pub(crate) fn hypot(x: f32, y: f32) -> f32 {
        f32::hypot(x, y)
    }

    #[inline(always)]
    pub(crate) fn sin(x: f32) -> f32 {
        f32::sin(x)
    }

    #[inline(always)]
    pub(crate) fn cos(x: f32) -> f32 {
        f32::cos(x)
    }

    #[inline(always)]
    pub(crate) fn tan(x: f32) -> f32 {
        f32::tan(x)
    }

    #[inline(always)]
    pub(crate) fn asin(x: f32) -> f32 {
        f32::asin(x)
    }

    #[inline(always)]
    pub(crate) fn acos(x: f32) -> f32 {
        f32::acos(x)
    }

    #[inline(always)]
    pub(crate) fn atan(x: f32) -> f32 {
        f32::atan(x)
    }

    #[inline(always)]
    pub(crate) fn atan2(x: f32, y: f32) -> f32 {
        f32::atan2(x, y)
    }

    #[inline(always)]
    pub(crate) fn sin_cos(x: f32) -> (f32, f32) {
        f32::sin_cos(x)
    }

    #[inline(always)]
    pub(crate) fn exp_m1(x: f32) -> f32 {
        f32::exp_m1(x)
    }

    #[inline(always)]
    pub(crate) fn ln_1p(x: f32) -> f32 {
        f32::ln_1p(x)
    }

    #[inline(always)]
    pub(crate) fn sinh(x: f32) -> f32 {
        f32::sinh(x)
    }

    #[inline(always)]
    pub(crate) fn cosh(x: f32) -> f32 {
        f32::cosh(x)
    }

    #[inline(always)]
    pub(crate) fn tanh(x: f32) -> f32 {
        f32::tanh(x)
    }

    #[inline(always)]
    pub(crate) fn asinh(x: f32) -> f32 {
        f32::asinh(x)
    }

    #[inline(always)]
    pub(crate) fn acosh(x: f32) -> f32 {
        f32::acosh(x)
    }

    #[inline(always)]
    pub(crate) fn atanh(x: f32) -> f32 {
        f32::atanh(x)
    }
}

#[cfg(feature = "libm")]
mod libm_ops {
    #[inline(always)]
    pub(crate) fn powf(x: f32, y: f32) -> f32 {
        libm::powf(x, y)
    }

    #[inline(always)]
    pub(crate) fn exp(x: f32) -> f32 {
        libm::expf(x)
    }

    #[inline(always)]
    pub(crate) fn exp2(x: f32) -> f32 {
        libm::exp2f(x)
    }

    #[inline(always)]
    pub(crate) fn ln(x: f32) -> f32 {
        // This isn't documented in `libm` but this is actually the base e logarithm.
        libm::logf(x)
    }

    #[inline(always)]
    pub(crate) fn log2(x: f32) -> f32 {
        libm::log2f(x)
    }

    #[inline(always)]
    pub(crate) fn log10(x: f32) -> f32 {
        libm::log10f(x)
    }

    #[inline(always)]
    pub(crate) fn cbrt(x: f32) -> f32 {
        libm::cbrtf(x)
    }

    #[inline(always)]
    pub(crate) fn hypot(x: f32, y: f32) -> f32 {
        libm::hypotf(x, y)
    }

    #[inline(always)]
    pub(crate) fn sin(x: f32) -> f32 {
        libm::sinf(x)
    }

    #[inline(always)]
    pub(crate) fn cos(x: f32) -> f32 {
        libm::cosf(x)
    }

    #[inline(always)]
    pub(crate) fn tan(x: f32) -> f32 {
        libm::tanf(x)
    }

    #[inline(always)]
    pub(crate) fn asin(x: f32) -> f32 {
        libm::asinf(x)
    }

    #[inline(always)]
    pub(crate) fn acos(x: f32) -> f32 {
        libm::acosf(x)
    }

    #[inline(always)]
    pub(crate) fn atan(x: f32) -> f32 {
        libm::atanf(x)
    }

    #[inline(always)]
    pub(crate) fn atan2(x: f32, y: f32) -> f32 {
        libm::atan2f(x, y)
    }

    #[inline(always)]
    pub(crate) fn sin_cos(x: f32) -> (f32, f32) {
        libm::sincosf(x)
    }

    #[inline(always)]
    pub(crate) fn exp_m1(x: f32) -> f32 {
        libm::expm1f(x)
    }

    #[inline(always)]
    pub(crate) fn ln_1p(x: f32) -> f32 {
        libm::log1pf(x)
    }

    #[inline(always)]
    pub(crate) fn sinh(x: f32) -> f32 {
        libm::sinhf(x)
    }

    #[inline(always)]
    pub(crate) fn cosh(x: f32) -> f32 {
        libm::coshf(x)
    }

    #[inline(always)]
    pub(crate) fn tanh(x: f32) -> f32 {
        libm::tanhf(x)
    }

    #[inline(always)]
    pub(crate) fn asinh(x: f32) -> f32 {
        libm::asinhf(x)
    }

    #[inline(always)]
    pub(crate) fn acosh(x: f32) -> f32 {
        libm::acoshf(x)
    }

    #[inline(always)]
    pub(crate) fn atanh(x: f32) -> f32 {
        libm::atanhf(x)
    }
}

#[cfg(feature = "libm")]
pub(crate) use libm_ops::*;
#[cfg(not(feature = "libm"))]
pub(crate) use std_ops::*;

/// This extension trait covers shortfall in determinacy from the lack of a `libm` counterpart
/// to `f32::powi`. Use this for the common small exponents.
pub(crate) trait FloatPow {
    fn squared(self) -> Self;
    fn cubed(self) -> Self;
}

impl FloatPow for f32 {
    #[inline]
    fn squared(self) -> Self {
        self * self
    }
    #[inline]
    fn cubed(self) -> Self {
        self * self * self
    }
}
