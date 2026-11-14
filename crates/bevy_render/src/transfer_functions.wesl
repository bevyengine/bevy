//! Transfer functions that encode linear light for HDR displays. They match
//! `bevy_render::transfer_functions` on the CPU.

import package::color_operations::linear_to_srgb;

const PQ_M1: f32 = 0.1593017578125;
const PQ_M2: f32 = 78.84375;
const PQ_C1: f32 = 0.8359375;
const PQ_C2: f32 = 18.8515625;
const PQ_C3: f32 = 18.6875;
const PQ_MAX_LUMINANCE_NITS: f32 = 10000.0;
const SCRGB_REFERENCE_WHITE_NITS: f32 = 80.0;

/// Scales color from paper white at `1.0` to scRGB, where `1.0` is 80 nits.
fn scrgb_encode(rgb: vec3<f32>, paper_white_nits: f32) -> vec3<f32> {
    return rgb * (paper_white_nits / SCRGB_REFERENCE_WHITE_NITS);
}

/// Converts absolute luminance in nits to a PQ signal. Negative values encode
/// as `0.0`, because `pow` is undefined for a negative base.
fn pq_inverse_eotf_from_nits(nits: vec3<f32>) -> vec3<f32> {
    let y = max(nits / PQ_MAX_LUMINANCE_NITS, vec3(0.0));
    let ym = pow(y, vec3(PQ_M1));
    return exp2(PQ_M2 * (log2(PQ_C1 + PQ_C2 * ym) - log2(1.0 + PQ_C3 * ym)));
}

/// The sRGB OETF continued above `1.0` and mirrored below `0.0`.
fn srgb_oetf_extended(rgb: vec3<f32>) -> vec3<f32> {
    return sign(rgb) * linear_to_srgb(abs(rgb));
}
