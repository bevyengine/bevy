use crate::{
    impl_componentwise_vector_space, Alpha, ColorToComponents, Gray, Hsla, Hsva, Hue, Hwba, Laba,
    Lcha, LinearRgba, Luminance, Mix, Oklaba, Oklcha, Saturation, Srgba, StandardColor, Xyza,
};
use bevy_math::{ops, Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Color in Okhsl color space, with alpha
#[doc = include_str!("../docs/conversion.md")]
/// <div>
#[doc = include_str!("../docs/diagrams/model_graph.svg")]
/// </div>
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, PartialEq, Default)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Okhsla {
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The saturation channel. [0.0, 1.0]
    pub saturation: f32,
    /// The lightness channel. [0.0, 1.0]
    pub lightness: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Okhsla {}

impl_componentwise_vector_space!(Okhsla, [hue, saturation, lightness, alpha]);

impl Okhsla {
    /// Construct a new [`Okhsla`] color from components.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Self {
        Self {
            hue,
            saturation,
            lightness,
            alpha,
        }
    }

    /// Construct a new [`Hsla`] color from (h, s, l) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    pub const fn okhsl(hue: f32, saturation: f32, lightness: f32) -> Self {
        Self::new(hue, saturation, lightness, 1.0)
    }

    /// Return a copy of this color with the saturation channel set to the given value.
    pub const fn with_saturation(self, saturation: f32) -> Self {
        Self { saturation, ..self }
    }

    /// Return a copy of this color with the lightness channel set to the given value.
    pub const fn with_lightness(self, lightness: f32) -> Self {
        Self { lightness, ..self }
    }

    /// Generate a deterministic but [quasi-randomly distributed](https://en.wikipedia.org/wiki/Low-discrepancy_sequence)
    /// color from a provided `index`.
    ///
    /// This can be helpful for generating debug colors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_color::Hsla;
    /// // Unique color for an entity
    /// # let entity_index = 123;
    /// // let entity_index = entity.index();
    /// let color = Hsla::sequential_dispersed(entity_index);
    ///
    /// // Palette with 5 distinct hues
    /// let palette = (0..5).map(Hsla::sequential_dispersed).collect::<Vec<_>>();
    /// ```
    pub const fn sequential_dispersed(index: u32) -> Self {
        const FRAC_U32MAX_GOLDEN_RATIO: u32 = 2654435769; // (u32::MAX / Φ) rounded up
        const RATIO_360: f32 = 360.0 / u32::MAX as f32;

        // from https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
        //
        // Map a sequence of integers (eg: 154, 155, 156, 157, 158) into the [0.0..1.0] range,
        // so that the closer the numbers are, the larger the difference of their image.
        let hue = index.wrapping_mul(FRAC_U32MAX_GOLDEN_RATIO) as f32 * RATIO_360;
        Self::okhsl(hue, 1., 0.5)
    }
}

impl Default for Okhsla {
    fn default() -> Self {
        Self::new(0., 0., 1., 1.)
    }
}

impl Mix for Okhsla {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            hue: crate::color_ops::lerp_hue(self.hue, other.hue, factor),
            saturation: self.saturation * n_factor + other.saturation * factor,
            lightness: self.lightness * n_factor + other.lightness * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for Okhsla {
    const BLACK: Self = Self::new(0., 0., 0., 1.);
    const WHITE: Self = Self::new(0., 0., 1., 1.);
}

impl Alpha for Okhsla {
    #[inline]
    fn with_alpha(&self, alpha: f32) -> Self {
        Self { alpha, ..*self }
    }

    #[inline]
    fn alpha(&self) -> f32 {
        self.alpha
    }

    #[inline]
    fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }
}

impl Hue for Okhsla {
    #[inline]
    fn with_hue(&self, hue: f32) -> Self {
        Self { hue, ..*self }
    }

    #[inline]
    fn hue(&self) -> f32 {
        self.hue
    }

    #[inline]
    fn set_hue(&mut self, hue: f32) {
        self.hue = hue;
    }
}

impl Saturation for Okhsla {
    #[inline]
    fn with_saturation(&self, saturation: f32) -> Self {
        Self {
            saturation,
            ..*self
        }
    }

    #[inline]
    fn saturation(&self) -> f32 {
        self.saturation
    }

    #[inline]
    fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation;
    }
}

impl Luminance for Okhsla {
    #[inline]
    fn with_luminance(&self, lightness: f32) -> Self {
        Self { lightness, ..*self }
    }

    fn luminance(&self) -> f32 {
        self.lightness
    }

    fn darker(&self, amount: f32) -> Self {
        Self {
            lightness: (self.lightness - amount).clamp(0., 1.),
            ..*self
        }
    }

    fn lighter(&self, amount: f32) -> Self {
        Self {
            lightness: (self.lightness + amount).min(1.),
            ..*self
        }
    }
}

impl ColorToComponents for Okhsla {
    fn to_f32_array(self) -> [f32; 4] {
        [self.hue, self.saturation, self.lightness, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.hue, self.saturation, self.lightness]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.hue, self.saturation, self.lightness, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.hue, self.saturation, self.lightness)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: 1.0,
        }
    }
}

#[cfg(feature = "wgpu-types")]
impl From<Okhsla> for wgpu_types::Color {
    fn from(color: Okhsla) -> Self {
        wgpu_types::Color {
            r: color.hue as f64,
            g: color.saturation as f64,
            b: color.lightness as f64,
            a: color.alpha as f64,
        }
    }
}

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

impl From<Oklaba> for Okhsla {
    fn from(value: Oklaba) -> Self {
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
            hue: h,
            saturation: s,
            lightness: l,
            alpha,
        };
    }
}

impl From<Okhsla> for Oklaba {
    fn from(value: Okhsla) -> Self {
        let Okhsla {
            hue: h,
            saturation: s,
            lightness: l,
            alpha,
        } = value;

        if (l == 1.) {
            return Oklaba::new(1., 1., 1., alpha);
        } else if (l == 0.) {
            return Oklaba::new(0., 0., 0., alpha);
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
}

// Derived Conversions

impl From<LinearRgba> for Okhsla {
    fn from(value: LinearRgba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for LinearRgba {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hsla> for Okhsla {
    fn from(value: Hsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Hsla {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hsva> for Okhsla {
    fn from(value: Hsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Hsva {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hwba> for Okhsla {
    fn from(value: Hwba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Hwba {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Lcha> for Okhsla {
    fn from(value: Lcha) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Lcha {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Srgba> for Okhsla {
    fn from(value: Srgba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Srgba {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Okhsla {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Xyza {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Laba> for Okhsla {
    fn from(value: Laba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Laba {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklcha> for Okhsla {
    fn from(value: Oklcha) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Oklcha {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_colors::TEST_COLORS, testing::assert_approx_eq};
}
