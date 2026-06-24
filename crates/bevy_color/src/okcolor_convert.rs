//! Functions for Okhsl/Okhsv <-> Oklab conversion.
//! See <https://github.com/bottosson/bottosson.github.io/blob/master/misc/ok_color.h>

// See comments start with `Patch` for how this differs from original `ok_color.h`

#![expect(
    non_snake_case,
    reason = "The code is translated from a C implementation."
)]

use crate::{okhsla::Okhsla, LinearRgba, Okhsva, Oklaba};
use bevy_math::ops;

#[derive(Clone, Copy)]
struct LC {
    L: f32,
    C: f32,
}

#[derive(Clone, Copy)]
struct ST {
    S: f32,
    T: f32,
}

fn to_ST(cusp: LC) -> ST {
    let L = cusp.L;
    let C = cusp.C;
    ST {
        S: C / L,
        T: C / (1. - L),
    }
}

// Finds the maximum saturation possible for a given hue that fits in sRGB
// Saturation here is defined as S = C/L
// a and b must be normalized so a^2 + b^2 == 1
fn compute_max_saturation(a: f32, b: f32) -> f32 {
    // Max saturation will be when one of r, g or b goes below zero.

    // Select different coefficients depending on which component goes below zero first
    let (k0, k1, k2, k3, k4, wl, wm, ws);

    if -1.881_703_3 * a - 0.809_364_9 * b > 1. {
        // Red component
        k0 = 1.190_862_8;
        k1 = 1.765_767_3;
        k2 = 0.596_626_4;
        k3 = 0.755_152;
        k4 = 0.567_712_4;
        wl = 4.076_741_7;
        wm = -3.307_711_6;
        ws = 0.230_969_94;
    } else if 1.814_441_1 * a - 1.194_452_8 * b > 1. {
        // Green component
        k0 = 0.73956515;
        k1 = -0.45954404;
        k2 = 0.08285427;
        k3 = 0.125_410_7;
        k4 = 0.14503204;
        wl = -1.268_438;
        wm = 2.609_757_4;
        ws = -0.341_319_38;
    } else {
        // Blue component
        k0 = 1.357_336_5;
        k1 = -0.00915799;
        k2 = -1.151_302_1;
        k3 = -0.50559606;
        k4 = 0.00692167;
        wl = -0.0041960863;
        wm = -0.703_418_6;
        ws = 1.707_614_7;
    }

    // Approximate max saturation using a polynomial:
    let mut S = k0 + k1 * a + k2 * b + k3 * a * a + k4 * a * b;

    // Do one step Halley's method to get closer
    // this gives an error less than 10e6, except for some blue hues where the dS/dh is close to infinite
    // this should be sufficient for most applications, otherwise do two/three steps

    let k_l = 0.396_337_78 * a + 0.215_803_76 * b;
    let k_m = -0.105_561_346 * a - 0.063_854_17 * b;
    let k_s = -0.089_484_18 * a - 1.291_485_5 * b;
    // Patch: Do two steps to reduce the error from 10e-4 to 10e-7 for some colors.
    for _ in 0..2 {
        let l_ = 1. + S * k_l;
        let m_ = 1. + S * k_m;
        let s_ = 1. + S * k_s;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let l_dS = 3. * k_l * l_ * l_;
        let m_dS = 3. * k_m * m_ * m_;
        let s_dS = 3. * k_s * s_ * s_;

        let l_dS2 = 6. * k_l * k_l * l_;
        let m_dS2 = 6. * k_m * k_m * m_;
        let s_dS2 = 6. * k_s * k_s * s_;

        let f = wl * l + wm * m + ws * s;
        let f1 = wl * l_dS + wm * m_dS + ws * s_dS;
        let f2 = wl * l_dS2 + wm * m_dS2 + ws * s_dS2;

        S -= f * f1 / (f1 * f1 - 0.5 * f * f2);
    }

    S
}

// finds L_cusp and C_cusp for a given hue
// a and b must be normalized so a^2 + b^2 == 1
fn find_cusp(a: f32, b: f32) -> LC {
    // First, find the maximum saturation (saturation S = C/L)
    let S_cusp = compute_max_saturation(a, b);

    // Convert to linear sRGB to find the first point where at least one of r,g or b >= 1:
    let rgb_at_max: LinearRgba = Oklaba::lab(1., S_cusp * a, S_cusp * b).into();
    let L_cusp = libm_cbrtf(1. / ((rgb_at_max.red.max(rgb_at_max.green)).max(rgb_at_max.blue)));
    let C_cusp = L_cusp * S_cusp;

    LC {
        L: L_cusp,
        C: C_cusp,
    }
}

// Finds intersection of the line defined by
// L = L0 * (1 - t) + t * L1;
// C = t * C1;
// a and b must be normalized so a^2 + b^2 == 1
fn find_gamut_intersection(a: f32, b: f32, L1: f32, C1: f32, L0: f32, cusp: LC) -> f32 {
    // Find the intersection for upper and lower half separately
    let mut t;
    if ((L1 - L0) * cusp.C - (cusp.L - L0) * C1) <= 0. {
        // Lower half

        t = cusp.C * L0 / (C1 * cusp.L + cusp.C * (L0 - L1));
    } else {
        // Upper half

        // First intersect with triangle
        t = cusp.C * (L0 - 1.) / (C1 * (cusp.L - 1.) + cusp.C * (L0 - L1));

        // Then one step Halley's method
        {
            let dL = L1 - L0;
            let dC = C1;

            let k_l = 0.396_337_78 * a + 0.215_803_76 * b;
            let k_m = -0.105_561_346 * a - 0.063_854_17 * b;
            let k_s = -0.089_484_18 * a - 1.291_485_5 * b;

            let l_dt = dL + dC * k_l;
            let m_dt = dL + dC * k_m;
            let s_dt = dL + dC * k_s;

            // If higher accuracy is required, 2 or 3 iterations of the following block can be used:
            // Patch: Do two steps to reduce the error from 10e-4 to 10e-7 for some colors.
            for _ in 0..2 {
                let L = L0 * (1. - t) + t * L1;
                let C = t * C1;

                let l_ = L + C * k_l;
                let m_ = L + C * k_m;
                let s_ = L + C * k_s;

                let l = l_ * l_ * l_;
                let m = m_ * m_ * m_;
                let s = s_ * s_ * s_;

                let ldt = 3. * l_dt * l_ * l_;
                let mdt = 3. * m_dt * m_ * m_;
                let sdt = 3. * s_dt * s_ * s_;

                let ldt2 = 6. * l_dt * l_dt * l_;
                let mdt2 = 6. * m_dt * m_dt * m_;
                let sdt2 = 6. * s_dt * s_dt * s_;

                let r = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s - 1.;
                let r1 = 4.076_741_7 * ldt - 3.307_711_6 * mdt + 0.230_969_94 * sdt;
                let r2 = 4.076_741_7 * ldt2 - 3.307_711_6 * mdt2 + 0.230_969_94 * sdt2;

                let u_r = r1 / (r1 * r1 - 0.5 * r * r2);
                let mut t_r = -r * u_r;

                let g = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s - 1.;
                let g1 = -1.268_438 * ldt + 2.609_757_4 * mdt - 0.341_319_38 * sdt;
                let g2 = -1.268_438 * ldt2 + 2.609_757_4 * mdt2 - 0.341_319_38 * sdt2;

                let u_g = g1 / (g1 * g1 - 0.5 * g * g2);
                let mut t_g = -g * u_g;

                let b = -0.0041960863 * l - 0.703_418_6 * m + 1.707_614_7 * s - 1.;
                let b1 = -0.0041960863 * ldt - 0.703_418_6 * mdt + 1.707_614_7 * sdt;
                let b2 = -0.0041960863 * ldt2 - 0.703_418_6 * mdt2 + 1.707_614_7 * sdt2;

                let u_b = b1 / (b1 * b1 - 0.5 * b * b2);
                let mut t_b = -b * u_b;

                t_r = if u_r >= 0. { t_r } else { f32::MAX };
                t_g = if u_g >= 0. { t_g } else { f32::MAX };
                t_b = if u_b >= 0. { t_b } else { f32::MAX };

                t += t_r.min(t_g.min(t_b));
            }
        }
    }

    t
}

#[derive(Clone, Copy)]
struct Cs {
    C_0: f32,
    C_mid: f32,
    C_max: f32,
}

fn get_Cs(L: f32, a_: f32, b_: f32) -> Cs {
    let cusp = find_cusp(a_, b_);

    let C_max = find_gamut_intersection(a_, b_, L, 1., L, cusp);
    let ST_max = to_ST(cusp);

    // Scale factor to compensate for the curved part of gamut shape:
    let k = C_max / ((L * ST_max.S).min((1. - L) * ST_max.T));

    let C_mid;
    {
        let ST_mid = get_ST_mid(a_, b_);

        // Use a soft minimum function, instead of a sharp triangle shape to get a smooth value for chroma.
        let C_a = L * ST_mid.S;
        let C_b = (1. - L) * ST_mid.T;
        C_mid = 0.9
            * k
            * ops::sqrt(ops::sqrt(
                1. / (1. / (C_a * C_a * C_a * C_a) + 1. / (C_b * C_b * C_b * C_b)),
            ));
    }

    let C_0;
    {
        // for C_0, the shape is independent of hue, so ST are constant. Values picked to roughly be the average values of ST.
        let C_a = L * 0.4;
        let C_b = (1. - L) * 0.8;

        // Use a soft minimum function, instead of a sharp triangle shape to get a smooth value for chroma.
        C_0 = ops::sqrt(1. / (1. / (C_a * C_a) + 1. / (C_b * C_b)));
    }

    Cs { C_0, C_mid, C_max }
}

// Returns a smooth approximation of the location of the cusp
// This polynomial was created by an optimization process
// It has been designed so that S_mid < S_max and T_mid < T_max
fn get_ST_mid(a_: f32, b_: f32) -> ST {
    let S = 0.11516993
        + 1. / (7.447_789_7
            + 4.159_012_3 * b_
            + a_ * (-2.195_573_6
                + 1.751_984 * b_
                + a_ * (-2.137_049_4 - 10.023_01 * b_
                    + a_ * (-4.248_945_7 + 5.387_708 * b_ + 4.698_91 * a_))));

    let T = 0.11239642
        + 1. / (1.613_203_2 - 0.681_243_8 * b_
            + a_ * (0.40370612
                + 0.901_481_2 * b_
                + a_ * (-0.27087943
                    + 0.612_239_9 * b_
                    + a_ * (0.00299215 - 0.45399568 * b_ - 0.14661872 * a_))));

    ST { S, T }
}

fn toe(x: f32) -> f32 {
    let k_1: f32 = 0.206;
    let k_2: f32 = 0.03;
    let k_3: f32 = (1. + k_1) / (1. + k_2);
    0.5 * (k_3 * x - k_1 + ops::sqrt((k_3 * x - k_1) * (k_3 * x - k_1) + 4. * k_2 * k_3 * x))
}

fn toe_inv(x: f32) -> f32 {
    let k_1 = 0.206;
    let k_2 = 0.03;
    let k_3 = (1. + k_1) / (1. + k_2);
    (x * x + k_1 * x) / (k_3 * (x + k_2))
}

pub(crate) fn oklab_to_okhsl(value: Oklaba) -> Okhsla {
    let Oklaba {
        lightness: lab_l,
        a: lab_a,
        b: lab_b,
        alpha,
    } = value;
    // Patch: Fixes NaN for pure black and white colors.
    if lab_l >= 1.0 {
        return Okhsla {
            hue: 0.0,
            saturation: 0.0,
            lightness: 1.0,
            alpha,
        };
    }
    if lab_l <= 0.0 {
        return Okhsla {
            hue: 0.0,
            saturation: 0.0,
            lightness: 0.0,
            alpha,
        };
    }
    let C = ops::sqrt(lab_a * lab_a + lab_b * lab_b);
    if C == 0. {
        let l = toe(lab_l);
        return Okhsla {
            hue: 0.,
            saturation: 0.,
            lightness: l,
            alpha,
        };
    }
    let a_ = lab_a / C;
    let b_ = lab_b / C;

    let L = lab_l;
    let h = 0.5 + 0.5 * ops::atan2(-lab_b, -lab_a) / core::f32::consts::PI;

    let cs = get_Cs(L, a_, b_);
    let C_0 = cs.C_0;
    let C_mid = cs.C_mid;
    let C_max = cs.C_max;

    // Inverse of the interpolation in okhsl_to_srgb:

    let mid = 0.8;
    let mid_inv = 1.25;

    let s = if C < C_mid {
        let k_1 = mid * C_0;
        let k_2 = 1. - k_1 / C_mid;

        let t = C / (k_1 + k_2 * C);
        t * mid
    } else {
        let k_0 = C_mid;
        let k_1 = (1. - mid) * C_mid * C_mid * mid_inv * mid_inv / C_0;
        let k_2 = 1. - (k_1) / (C_max - C_mid);

        let t = (C - k_0) / (k_1 + k_2 * (C - k_0));
        mid + (1. - mid) * t
    };

    let l = toe(L);
    Okhsla {
        hue: h * 360.,
        saturation: s,
        lightness: l,
        alpha,
    }
}

pub(crate) fn okhsl_to_oklab(value: Okhsla) -> Oklaba {
    let Okhsla {
        hue: h,
        saturation: s,
        lightness: l,
        alpha,
    } = value;
    let h = h / 360.;

    if l >= 1. {
        return LinearRgba::new(1., 1., 1., alpha).into();
    } else if l <= 0. {
        return LinearRgba::new(0., 0., 0., alpha).into();
    }

    let a_ = ops::cos(2. * core::f32::consts::PI * h);
    let b_ = ops::sin(2. * core::f32::consts::PI * h);
    let L = toe_inv(l);

    let cs = get_Cs(L, a_, b_);
    let C_0 = cs.C_0;
    let C_mid = cs.C_mid;
    let C_max = cs.C_max;

    let mid = 0.8;
    let mid_inv = 1.25;

    let (C, t, k_0, k_1, k_2);

    if s < mid {
        t = mid_inv * s;

        k_1 = mid * C_0;
        k_2 = 1. - k_1 / C_mid;

        C = t * k_1 / (1. - k_2 * t);
    } else {
        t = (s - mid) / (1. - mid);

        k_0 = C_mid;
        k_1 = (1. - mid) * C_mid * C_mid * mid_inv * mid_inv / C_0;
        k_2 = 1. - (k_1) / (C_max - C_mid);

        C = k_0 + t * k_1 / (1. - k_2 * t);
    }

    Oklaba::new(L, C * a_, C * b_, alpha)
}

pub(crate) fn oklab_to_okhsv(value: Oklaba) -> Okhsva {
    let Oklaba {
        lightness: lab_l,
        a: lab_a,
        b: lab_b,
        alpha,
    } = value;
    // Patch: Fixes NaN for pure black and white colors.
    if lab_l >= 1.0 {
        return Okhsva {
            hue: 0.0,
            saturation: 0.0,
            value: 1.0,
            alpha,
        };
    }
    if lab_l <= 0.0 {
        return Okhsva {
            hue: 0.0,
            saturation: 0.0,
            value: 0.0,
            alpha,
        };
    }
    let C = ops::sqrt(lab_a * lab_a + lab_b * lab_b);
    if C == 0. {
        // In this case, value is equal to lightness.
        let l = toe(lab_l);
        return Okhsva {
            hue: 0.,
            saturation: 0.,
            value: l,
            alpha,
        };
    }
    let a_ = lab_a / C;
    let b_ = lab_b / C;

    let mut L = lab_l;
    let h = 0.5 + 0.5 * ops::atan2(-lab_b, -lab_a) / core::f32::consts::PI;

    let cusp = find_cusp(a_, b_);
    let ST_max = to_ST(cusp);
    let S_max = ST_max.S;
    let T_max = ST_max.T;
    let S_0 = 0.5;
    let k = 1. - S_0 / S_max;

    // first we find L_v, C_v, L_vt and C_vt

    let t = T_max / (C + L * T_max);
    let L_v = t * L;
    let C_v = t * C;

    let L_vt = toe_inv(L_v);
    let C_vt = C_v * L_vt / L_v;

    // we can then use these to invert the step that compensates for the toe and the curved top part of the triangle:
    let rgb_scale: LinearRgba = Oklaba::lab(L_vt, a_ * C_vt, b_ * C_vt).into();
    let scale_L =
        libm_cbrtf(1. / ((rgb_scale.red.max(rgb_scale.green)).max(rgb_scale.blue.max(0.))));

    L /= scale_L;

    L = toe(L);

    // we can now compute v and s:

    let v = L / L_v;
    let s = (S_0 + T_max) * C_v / ((T_max * S_0) + T_max * k * C_v);

    Okhsva {
        hue: h * 360.,
        saturation: s,
        value: v,
        alpha,
    }
}

pub(crate) fn okhsv_to_oklab(value: Okhsva) -> Oklaba {
    let Okhsva {
        hue: h,
        saturation: s,
        value: v,
        alpha,
    } = value;
    let h = h / 360.;

    if v <= 0. {
        return LinearRgba::new(0., 0., 0., alpha).into();
    }

    let a_ = ops::cos(2. * core::f32::consts::PI * h);
    let b_ = ops::sin(2. * core::f32::consts::PI * h);

    let cusp = find_cusp(a_, b_);
    let ST_max = to_ST(cusp);
    let S_max = ST_max.S;
    let T_max = ST_max.T;
    let S_0 = 0.5;
    let k = 1. - S_0 / S_max;

    // first we compute L and V as if the gamut is a perfect triangle:

    // L, C when v==1:
    let L_v = 1. - s * S_0 / (S_0 + T_max - T_max * k * s);
    let C_v = s * T_max * S_0 / (S_0 + T_max - T_max * k * s);

    let mut L = v * L_v;
    let mut C = v * C_v;

    // then we compensate for both toe and the curved top part of the triangle:
    let L_vt = toe_inv(L_v);
    let C_vt = C_v * L_vt / L_v;

    let L_new = toe_inv(L);
    C = C * L_new / L;
    L = L_new;

    let rgb_scale: LinearRgba = Oklaba::lab(L_vt, a_ * C_vt, b_ * C_vt).into();
    let scale_L =
        libm_cbrtf(1. / ((rgb_scale.red.max(rgb_scale.green)).max(rgb_scale.blue.max(0.))));

    L *= scale_L;
    C *= scale_L;

    Oklaba::new(L, C * a_, C * b_, alpha)
}

// Note: This is copied from `libm` to fix precision issue on Windows CI testing.

const B1: u32 = 709958130; /* B1 = (127-127.0/3-0.03306235651)*2**23 */
const B2: u32 = 642849266; /* B2 = (127-127.0/3-24/3-0.03306235651)*2**23 */
/// Cube root (f32)
///
/// Computes the cube root of the argument.
pub(crate) fn libm_cbrtf(x: f32) -> f32 {
    let x1p24 = f32::from_bits(0x4b800000); // 0x1p24f === 2 ^ 24

    let mut r: f64;
    let mut t: f64;
    let mut ui: u32 = x.to_bits();
    let mut hx: u32 = ui & 0x7fffffff;

    if hx >= 0x7f800000 {
        /* cbrt(NaN,INF) is itself */
        return x + x;
    }

    /* rough cbrt to 5 bits */
    if hx < 0x00800000 {
        /* zero or subnormal? */
        if hx == 0 {
            return x; /* cbrt(+-0) is itself */
        }
        ui = (x * x1p24).to_bits();
        hx = ui & 0x7fffffff;
        hx = hx / 3 + B2;
    } else {
        hx = hx / 3 + B1;
    }
    ui &= 0x80000000;
    ui |= hx;

    /*
     * First step Newton iteration (solving t*t-x/t == 0) to 16 bits.  In
     * double precision so that its terms can be arranged for efficiency
     * without causing overflow or underflow.
     */
    t = f32::from_bits(ui) as f64;
    r = t * t * t;
    t = t * (x as f64 + x as f64 + r) / (x as f64 + r + r);

    /*
     * Second step Newton iteration to 47 bits.  In double precision for
     * efficiency and accuracy.
     */
    r = t * t * t;
    t = t * (x as f64 + x as f64 + r) / (x as f64 + r + r);

    /* rounding to 24 bits is perfect in round-to-nearest mode */
    t as f32
}
