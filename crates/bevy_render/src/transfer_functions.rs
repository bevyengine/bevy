//! Transfer functions that encode linear light, matching `transfer_functions.wesl`.

use bevy_color::Srgba;
use bevy_math::ops;
use bevy_window::MAX_LUMINANCE_NITS;

const PQ_M1: f32 = 2610.0 / 16384.0;
const PQ_M2: f32 = 2523.0 / 4096.0 * 128.0;
const PQ_C1: f32 = 3424.0 / 4096.0;
const PQ_C2: f32 = 2413.0 / 4096.0 * 32.0;
const PQ_C3: f32 = 2392.0 / 4096.0 * 32.0;

/// Converts absolute luminance in nits to a PQ signal, using the inverse EOTF
/// from SMPTE ST 2084.
///
/// Negative luminance encodes as `0.0`. Luminance above 10000 nits gives a
/// signal above `1.0`.
pub fn pq_inverse_eotf_from_nits(nits: f32) -> f32 {
    let y = (nits / MAX_LUMINANCE_NITS).max(0.0);
    let ym = ops::powf(y, PQ_M1);
    ops::exp2(PQ_M2 * (ops::log2(PQ_C1 + PQ_C2 * ym) - ops::log2(1.0 + PQ_C3 * ym)))
}

/// The sRGB OETF, extended above `1.0` and mirrored below `0.0` for extended
/// sRGB color spaces.
pub fn srgb_oetf_extended(linear: f32) -> f32 {
    Srgba::gamma_function_inverse(linear.abs()).copysign(linear)
}
