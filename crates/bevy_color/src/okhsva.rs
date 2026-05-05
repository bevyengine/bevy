use crate::{
    okhsla::{find_cusp, to_ST, toe, toe_inv, Okhsla},
    Alpha, ColorToComponents, Gray, Hsla, Hsva, Hue, Hwba, Laba, Lcha, LinearRgba, Mix, Oklaba,
    Oklcha, Saturation, Srgba, StandardColor, Xyza,
};
use bevy_math::{ops, Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Color in Okhsv color space with alpha.
/// Further information on this color model can be found on [Wikipedia](https://en.wikipedia.org/wiki/HSL_and_HSV).
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
pub struct Okhsva {
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The saturation channel. [0.0, 1.0]
    pub saturation: f32,
    /// The value channel. [0.0, 1.0]
    pub value: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Okhsva {}

impl Okhsva {
    /// Construct a new [`Okhsva`] color from components.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `value` - Value channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(hue: f32, saturation: f32, value: f32, alpha: f32) -> Self {
        Self {
            hue,
            saturation,
            value,
            alpha,
        }
    }

    /// Construct a new [`Okhsva`] color from (h, s, v) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `value` - Value channel. [0.0, 1.0]
    pub const fn hsv(hue: f32, saturation: f32, value: f32) -> Self {
        Self::new(hue, saturation, value, 1.0)
    }

    /// Return a copy of this color with the saturation channel set to the given value.
    pub const fn with_saturation(self, saturation: f32) -> Self {
        Self { saturation, ..self }
    }

    /// Return a copy of this color with the value channel set to the given value.
    pub const fn with_value(self, value: f32) -> Self {
        Self { value, ..self }
    }
}

impl Default for Okhsva {
    fn default() -> Self {
        Self::new(0., 0., 1., 1.)
    }
}

impl Mix for Okhsva {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            hue: crate::color_ops::lerp_hue(self.hue, other.hue, factor),
            saturation: self.saturation * n_factor + other.saturation * factor,
            value: self.value * n_factor + other.value * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for Okhsva {
    const BLACK: Self = Self::new(0., 0., 0., 1.);
    const WHITE: Self = Self::new(0., 0., 1., 1.);
}

impl Alpha for Okhsva {
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

impl Hue for Okhsva {
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

impl Saturation for Okhsva {
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

impl From<Okhsva> for Hwba {
    fn from(
        Okhsva {
            hue,
            saturation,
            value,
            alpha,
        }: Okhsva,
    ) -> Self {
        // Based on https://en.wikipedia.org/wiki/HWB_color_model#Conversion
        let whiteness = (1. - saturation) * value;
        let blackness = 1. - value;

        Hwba::new(hue, whiteness, blackness, alpha)
    }
}

impl From<Hwba> for Okhsva {
    fn from(
        Hwba {
            hue,
            whiteness,
            blackness,
            alpha,
        }: Hwba,
    ) -> Self {
        // Based on https://en.wikipedia.org/wiki/HWB_color_model#Conversion
        let value = 1. - blackness;
        let saturation = if value != 0. {
            1. - (whiteness / value)
        } else {
            0.
        };

        Okhsva::new(hue, saturation, value, alpha)
    }
}

impl ColorToComponents for Okhsva {
    fn to_f32_array(self) -> [f32; 4] {
        [self.hue, self.saturation, self.value, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.hue, self.saturation, self.value]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.hue, self.saturation, self.value, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.hue, self.saturation, self.value)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            value: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            value: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            value: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            value: color[2],
            alpha: 1.0,
        }
    }
}

impl From<Oklaba> for Okhsva {
    fn from(value: Oklaba) -> Self {
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
            hue: h,
            saturation: s,
            value: v,
            alpha,
        };
    }
}

impl From<Okhsva> for Oklaba {
    fn from(value: Okhsva) -> Self {
        let Okhsva {
            hue: h,
            saturation: s,
            value: v,
            alpha,
        } = value;

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
}

// Derived Conversions

impl From<LinearRgba> for Okhsva {
    fn from(value: LinearRgba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for LinearRgba {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Srgba> for Okhsva {
    fn from(value: Srgba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Srgba {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Lcha> for Okhsva {
    fn from(value: Lcha) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Lcha {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Okhsva {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Xyza {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsla> for Okhsva {
    fn from(value: Okhsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Okhsla {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hsla> for Okhsva {
    fn from(value: Hsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Hsla {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hsva> for Okhsva {
    fn from(value: Hsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Hsva {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Laba> for Okhsva {
    fn from(value: Laba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Laba {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklcha> for Okhsva {
    fn from(value: Oklcha) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Okhsva> for Oklcha {
    fn from(value: Okhsva) -> Self {
        LinearRgba::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        color_difference::EuclideanDistance, test_colors::TEST_COLORS, testing::assert_approx_eq,
    };
}
