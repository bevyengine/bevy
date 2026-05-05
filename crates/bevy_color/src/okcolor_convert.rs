use crate::{okhsla::Okhsla, LinearRgba, Okhsva, Oklaba};
use bevy_math::ops;

#[derive(Clone, Copy)]
pub(crate) struct LC {
    pub(crate) L: f32,
    pub(crate) C: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct ST {
    pub(crate) S: f32,
    pub(crate) T: f32,
}

pub(crate) fn to_ST(cusp: LC) -> ST {
    let L = cusp.L;
    let C = cusp.C;
    return ST {
        S: C / L,
        T: C / (1. - L),
    };
}

// Finds the maximum saturation possible for a given hue that fits in sRGB
// Saturation here is defined as S = C/L
// a and b must be normalized so a^2 + b^2 == 1
fn compute_max_saturation(a: f32, b: f32) -> f32 {
    // Max saturation will be when one of r, g or b goes below zero.

    // Select different coefficients depending on which component goes below zero first
    let (k0, k1, k2, k3, k4, wl, wm, ws);

    if (-1.88170328 * a - 0.80936493 * b > 1.) {
        // Red component
        k0 = 1.19086277;
        k1 = 1.76576728;
        k2 = 0.59662641;
        k3 = 0.75515197;
        k4 = 0.56771245;
        wl = 4.0767416621;
        wm = -3.3077115913;
        ws = 0.2309699292;
    } else if (1.81444104 * a - 1.19445276 * b > 1.) {
        // Green component
        k0 = 0.73956515;
        k1 = -0.45954404;
        k2 = 0.08285427;
        k3 = 0.12541070;
        k4 = 0.14503204;
        wl = -1.2684380046;
        wm = 2.6097574011;
        ws = -0.3413193965;
    } else {
        // Blue component
        k0 = 1.35733652;
        k1 = -0.00915799;
        k2 = -1.15130210;
        k3 = -0.50559606;
        k4 = 0.00692167;
        wl = -0.0041960863;
        wm = -0.7034186147;
        ws = 1.7076147010;
    }

    // Approximate max saturation using a polynomial:
    let mut S = k0 + k1 * a + k2 * b + k3 * a * a + k4 * a * b;

    // Do one step Halley's method to get closer
    // this gives an error less than 10e6, except for some blue hues where the dS/dh is close to infinite
    // this should be sufficient for most applications, otherwise do two/three steps

    let k_l = 0.3963377774 * a + 0.2158037573 * b;
    let k_m = -0.1055613458 * a - 0.0638541728 * b;
    let k_s = -0.0894841775 * a - 1.2914855480 * b;

    {
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

        S = S - f * f1 / (f1 * f1 - 0.5 * f * f2);
    }

    return S;
}

// finds L_cusp and C_cusp for a given hue
// a and b must be normalized so a^2 + b^2 == 1
pub(crate) fn find_cusp(a: f32, b: f32) -> LC {
    // First, find the maximum saturation (saturation S = C/L)
    let S_cusp = compute_max_saturation(a, b);

    // Convert to linear sRGB to find the first point where at least one of r,g or b >= 1:
    let rgb_at_max: LinearRgba = Oklaba::lab(1., S_cusp * a, S_cusp * b).into();
    let L_cusp = ops::cbrt((1. / ((rgb_at_max.red.max(rgb_at_max.green)).max(rgb_at_max.blue))));
    let C_cusp = L_cusp * S_cusp;

    return LC {
        L: L_cusp,
        C: C_cusp,
    };
}

// Finds intersection of the line defined by
// L = L0 * (1 - t) + t * L1;
// C = t * C1;
// a and b must be normalized so a^2 + b^2 == 1
fn find_gamut_intersection(a: f32, b: f32, L1: f32, C1: f32, L0: f32, cusp: LC) -> f32 {
    // Find the intersection for upper and lower half seprately
    let mut t;
    if (((L1 - L0) * cusp.C - (cusp.L - L0) * C1) <= 0.) {
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

            let k_l = 0.3963377774 * a + 0.2158037573 * b;
            let k_m = -0.1055613458 * a - 0.0638541728 * b;
            let k_s = -0.0894841775 * a - 1.2914855480 * b;

            let l_dt = dL + dC * k_l;
            let m_dt = dL + dC * k_m;
            let s_dt = dL + dC * k_s;

            // If higher accuracy is required, 2 or 3 iterations of the following block can be used:
            {
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

                let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s - 1.;
                let r1 = 4.0767416621 * ldt - 3.3077115913 * mdt + 0.2309699292 * sdt;
                let r2 = 4.0767416621 * ldt2 - 3.3077115913 * mdt2 + 0.2309699292 * sdt2;

                let u_r = r1 / (r1 * r1 - 0.5 * r * r2);
                let mut t_r = -r * u_r;

                let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s - 1.;
                let g1 = -1.2684380046 * ldt + 2.6097574011 * mdt - 0.3413193965 * sdt;
                let g2 = -1.2684380046 * ldt2 + 2.6097574011 * mdt2 - 0.3413193965 * sdt2;

                let u_g = g1 / (g1 * g1 - 0.5 * g * g2);
                let mut t_g = -g * u_g;

                let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s - 1.;
                let b1 = -0.0041960863 * ldt - 0.7034186147 * mdt + 1.7076147010 * sdt;
                let b2 = -0.0041960863 * ldt2 - 0.7034186147 * mdt2 + 1.7076147010 * sdt2;

                let u_b = b1 / (b1 * b1 - 0.5 * b * b2);
                let mut t_b = -b * u_b;

                t_r = if u_r >= 0. { t_r } else { core::f32::MAX };
                t_g = if u_g >= 0. { t_g } else { core::f32::MAX };
                t_b = if u_b >= 0. { t_b } else { core::f32::MAX };

                t += (t_r.min(t_g.min(t_b)));
            }
        }
    }

    return t;
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
            * (1. / (1. / (C_a * C_a * C_a * C_a) + 1. / (C_b * C_b * C_b * C_b)))
                .sqrt()
                .sqrt();
    }

    let C_0;
    {
        // for C_0, the shape is independent of hue, so ST are constant. Values picked to roughly be the average values of ST.
        let C_a = L * 0.4;
        let C_b = (1. - L) * 0.8;

        // Use a soft minimum function, instead of a sharp triangle shape to get a smooth value for chroma.
        C_0 = (1. / (1. / (C_a * C_a) + 1. / (C_b * C_b))).sqrt();
    }

    return Cs { C_0, C_mid, C_max };
}

// Returns a smooth approximation of the location of the cusp
// This polynomial was created by an optimization process
// It has been designed so that S_mid < S_max and T_mid < T_max
fn get_ST_mid(a_: f32, b_: f32) -> ST {
    let S = 0.11516993
        + 1. / (7.44778970
            + 4.15901240 * b_
            + a_ * (-2.19557347
                + 1.75198401 * b_
                + a_ * (-2.13704948 - 10.02301043 * b_
                    + a_ * (-4.24894561 + 5.38770819 * b_ + 4.69891013 * a_))));

    let T = 0.11239642
        + 1. / (1.61320320 - 0.68124379 * b_
            + a_ * (0.40370612
                + 0.90148123 * b_
                + a_ * (-0.27087943
                    + 0.61223990 * b_
                    + a_ * (0.00299215 - 0.45399568 * b_ - 0.14661872 * a_))));

    return ST { S, T };
}

pub(crate) fn toe(x: f32) -> f32 {
    let k_1: f32 = 0.206;
    let k_2: f32 = 0.03;
    let k_3: f32 = (1. + k_1) / (1. + k_2);
    return 0.5 * (k_3 * x - k_1 + ((k_3 * x - k_1) * (k_3 * x - k_1) + 4. * k_2 * k_3 * x).sqrt());
}

pub(crate) fn toe_inv(x: f32) -> f32 {
    let k_1 = 0.206;
    let k_2 = 0.03;
    let k_3 = (1. + k_1) / (1. + k_2);
    return (x * x + k_1 * x) / (k_3 * (x + k_2));
}

pub(crate) fn oklab_to_okhsl(value: Oklaba) -> Okhsla {
    let Oklaba {
        lightness: lab_l,
        a: lab_a,
        b: lab_b,
        alpha,
    } = value;
    let C = (lab_a * lab_a + lab_b * lab_b).sqrt();
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

    let s;
    if (C < C_mid) {
        let k_1 = mid * C_0;
        let k_2 = (1. - k_1 / C_mid);

        let t = C / (k_1 + k_2 * C);
        s = t * mid;
    } else {
        let k_0 = C_mid;
        let k_1 = (1. - mid) * C_mid * C_mid * mid_inv * mid_inv / C_0;
        let k_2 = (1. - (k_1) / (C_max - C_mid));

        let t = (C - k_0) / (k_1 + k_2 * (C - k_0));
        s = mid + (1. - mid) * t;
    }

    let l = toe(L);
    return Okhsla {
        hue: h * 360.,
        saturation: s,
        lightness: l,
        alpha,
    };
}

pub(crate) fn okhsl_to_oklab(value: Okhsla) -> Oklaba {
    let Okhsla {
        hue: h,
        saturation: s,
        lightness: l,
        alpha,
    } = value;
    let h = h / 360.;

    if (l == 1.) {
        return LinearRgba::new(1., 1., 1., alpha).into();
    } else if (l == 0.) {
        return LinearRgba::new(0., 0., 0., alpha).into();
    }

    let a_ = (2. * core::f32::consts::PI * h).cos();
    let b_ = (2. * core::f32::consts::PI * h).sin();
    let L = toe_inv(l);

    let cs = get_Cs(L, a_, b_);
    let C_0 = cs.C_0;
    let C_mid = cs.C_mid;
    let C_max = cs.C_max;

    let mid = 0.8;
    let mid_inv = 1.25;

    let (C, t, k_0, k_1, k_2);

    if (s < mid) {
        t = mid_inv * s;

        k_1 = mid * C_0;
        k_2 = (1. - k_1 / C_mid);

        C = t * k_1 / (1. - k_2 * t);
    } else {
        t = (s - mid) / (1. - mid);

        k_0 = C_mid;
        k_1 = (1. - mid) * C_mid * C_mid * mid_inv * mid_inv / C_0;
        k_2 = (1. - (k_1) / (C_max - C_mid));

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
    let mut C = (lab_a * lab_a + lab_b * lab_b).sqrt();
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
        ops::cbrt(1. / ((rgb_scale.red.max(rgb_scale.green)).max(rgb_scale.blue.max(0.))));

    L = L / scale_L;
    C = C / scale_L;

    C = C * toe(L) / L;
    L = toe(L);

    // we can now compute v and s:

    let v = L / L_v;
    let s = (S_0 + T_max) * C_v / ((T_max * S_0) + T_max * k * C_v);

    return Okhsva {
        hue: h * 360.,
        saturation: s,
        value: v,
        alpha,
    };
}

pub(crate) fn okhsv_to_oklab(value: Okhsva) -> Oklaba {
    let Okhsva {
        hue: h,
        saturation: s,
        value: v,
        alpha,
    } = value;
    let h = h / 360.;

    if (v == 0.) {
        return LinearRgba::new(0., 0., 0., alpha).into();
    }

    let a_ = (2. * core::f32::consts::PI * h).cos();
    let b_ = (2. * core::f32::consts::PI * h).sin();

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
        ops::cbrt(1. / ((rgb_scale.red.max(rgb_scale.green)).max((rgb_scale.blue.max(0.)))));

    L = L * scale_L;
    C = C * scale_L;

    Oklaba::new(L, C * a_, C * b_, alpha)
}
