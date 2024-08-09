//! This mod re-exports the correct versions of floating-point operations with
//! unspecified precision depending on whether the `libm` crate feature is enabled.

// Note: There are some Rust methods with unspecified precision without a `libm`
// equivalent:
// - `f32::powi`

#[cfg(feature = "libm")]
use libm;

mod std_ops {
    #[inline(always)]
    pub(crate) fn powf(x: f32, y: f32) -> f32 {
        f32::powf(x, y)
    }
}

#[cfg(feature = "libm")]
mod libm_ops {
    #[inline(always)]
    pub(crate) fn powf(x: f32, y: f32) -> f32 {
        libm::powf(x, y)
    }
}
