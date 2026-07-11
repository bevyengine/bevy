use crate::{
    okhsva::Okhsva, Alpha, ColorToComponents, Gray, Hsla, Hsva, Hue, Hwba, Laba, Lcha, LinearRgba,
    Mix, Okhsla, Oklaba, Oklcha, Srgba, StandardColor, Xyza,
};
use bevy_math::{Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Color in Okhwb color space with alpha.
/// Further information on this color model can be found on <https://bottosson.github.io/posts/colorpicker>.
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
pub struct Okhwba {
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The whiteness channel. [0.0, 1.0]
    pub whiteness: f32,
    /// The blackness channel. [0.0, 1.0]
    pub blackness: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Okhwba {}

impl Okhwba {
    /// Construct a new [`Okhwba`] color from components.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `whiteness` - Whiteness channel. [0.0, 1.0]
    /// * `blackness` - Blackness channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(hue: f32, whiteness: f32, blackness: f32, alpha: f32) -> Self {
        Self {
            hue,
            whiteness,
            blackness,
            alpha,
        }
    }

    /// Construct a new [`Okhwba`] color from (h, w, b) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `whiteness` - Whiteness channel. [0.0, 1.0]
    /// * `blackness` - Blackness channel. [0.0, 1.0]
    pub const fn hwb(hue: f32, whiteness: f32, blackness: f32) -> Self {
        Self::new(hue, whiteness, blackness, 1.0)
    }

    /// Return a copy of this color with the whiteness channel set to the given value.
    pub const fn with_whiteness(self, whiteness: f32) -> Self {
        Self { whiteness, ..self }
    }

    /// Return a copy of this color with the blackness channel set to the given value.
    pub const fn with_blackness(self, blackness: f32) -> Self {
        Self { blackness, ..self }
    }
}

impl Default for Okhwba {
    fn default() -> Self {
        Self::new(0., 0., 0., 1.)
    }
}

impl Mix for Okhwba {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            hue: crate::color_ops::lerp_hue(self.hue, other.hue, factor),
            whiteness: self.whiteness * n_factor + other.whiteness * factor,
            blackness: self.blackness * n_factor + other.blackness * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for Okhwba {
    const BLACK: Self = Self::new(0., 0., 1., 1.);
    const WHITE: Self = Self::new(0., 1., 0., 1.);
}

impl Alpha for Okhwba {
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

impl Hue for Okhwba {
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

impl ColorToComponents for Okhwba {
    fn to_f32_array(self) -> [f32; 4] {
        [self.hue, self.whiteness, self.blackness, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.hue, self.whiteness, self.blackness]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.hue, self.whiteness, self.blackness, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.hue, self.whiteness, self.blackness)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            hue: color[0],
            whiteness: color[1],
            blackness: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            hue: color[0],
            whiteness: color[1],
            blackness: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            hue: color[0],
            whiteness: color[1],
            blackness: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            hue: color[0],
            whiteness: color[1],
            blackness: color[2],
            alpha: 1.0,
        }
    }
}

impl From<Okhsva> for Okhwba {
    fn from(
        Okhsva {
            hue,
            saturation,
            value,
            alpha,
        }: Okhsva,
    ) -> Self {
        // Based on https://bottosson.github.io/posts/colorpicker/#okhwb
        let whiteness = (1. - saturation) * value;
        let blackness = 1. - value;

        Okhwba::new(hue, whiteness, blackness, alpha)
    }
}

impl From<Okhwba> for Okhsva {
    fn from(
        Okhwba {
            hue,
            whiteness,
            blackness,
            alpha,
        }: Okhwba,
    ) -> Self {
        // Based on https://bottosson.github.io/posts/colorpicker/#okhwb
        let value = 1. - blackness;
        let saturation = if value != 0. {
            1. - (whiteness / value)
        } else {
            0.
        };

        Okhsva::new(hue, saturation, value, alpha)
    }
}

// Derived Conversions

impl From<LinearRgba> for Okhwba {
    fn from(value: LinearRgba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for LinearRgba {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Srgba> for Okhwba {
    fn from(value: Srgba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Srgba {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Hwba> for Okhwba {
    fn from(value: Hwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Hwba {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Lcha> for Okhwba {
    fn from(value: Lcha) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Lcha {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Xyza> for Okhwba {
    fn from(value: Xyza) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Xyza {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhsla> for Okhwba {
    fn from(value: Okhsla) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Okhsla {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Hsla> for Okhwba {
    fn from(value: Hsla) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Hsla {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Hsva> for Okhwba {
    fn from(value: Hsva) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Hsva {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Laba> for Okhwba {
    fn from(value: Laba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Laba {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Oklaba> for Okhwba {
    fn from(value: Oklaba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Oklaba {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Oklcha> for Okhwba {
    fn from(value: Oklcha) -> Self {
        Okhsva::from(value).into()
    }
}

impl From<Okhwba> for Oklcha {
    fn from(value: Okhwba) -> Self {
        Okhsva::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_approx_eq;

    #[test]
    fn test_from_oklaba() {
        // Test `oklab_l == 0.0`
        let oklaba = Oklaba::new(0.0, 0.5, 0.5, 1.0);
        let okhwba: Okhwba = oklaba.into();
        let oklaba2: Oklaba = okhwba.into();
        assert_approx_eq!(okhwba.hue, 0.0, 0.001);
        assert_approx_eq!(okhwba.whiteness, 0.0, 0.001);
        assert_approx_eq!(okhwba.blackness, 1.0, 0.001);
        assert_approx_eq!(okhwba.alpha, 1.0, 0.001);

        assert_approx_eq!(oklaba.lightness, oklaba2.lightness, 0.001);
        assert_approx_eq!(0.0, oklaba2.a, 0.001);
        assert_approx_eq!(0.0, oklaba2.b, 0.001);
        assert_approx_eq!(oklaba.alpha, oklaba2.alpha, 0.001);

        // Test `oklab_l == 1.0`
        let oklaba = Oklaba::new(1.0, 0.5, 0.5, 1.0);
        let okhwba: Okhwba = oklaba.into();
        let oklaba2: Oklaba = okhwba.into();
        assert_approx_eq!(okhwba.hue, 0.0, 0.001);
        assert_approx_eq!(okhwba.whiteness, 1.0, 0.001);
        assert_approx_eq!(okhwba.blackness, 0.0, 0.001);
        assert_approx_eq!(okhwba.alpha, 1.0, 0.001);

        assert_approx_eq!(oklaba.lightness, oklaba2.lightness, 0.001);
        assert_approx_eq!(0.0, oklaba2.a, 0.001);
        assert_approx_eq!(0.0, oklaba2.b, 0.001);
        assert_approx_eq!(oklaba.alpha, oklaba2.alpha, 0.001);

        // Test `oklab_a == 0.0 && oklab_b ==0.0` (C == 0.0)
        let oklaba = Oklaba::new(0.5, 0.0, 0.0, 1.0);
        let okhwba: Okhwba = oklaba.into();
        let oklaba2: Oklaba = okhwba.into();
        assert_approx_eq!(okhwba.hue, 0.0, 0.001);
        assert_approx_eq!(okhwba.whiteness, 0.42114055, 0.001);
        assert_approx_eq!(okhwba.blackness, 0.57885945, 0.001);
        assert_approx_eq!(okhwba.alpha, 1.0, 0.001);

        assert_approx_eq!(oklaba.lightness, oklaba2.lightness, 0.001);
        assert_approx_eq!(0.0, oklaba2.a, 0.001);
        assert_approx_eq!(0.0, oklaba2.b, 0.001);
        assert_approx_eq!(oklaba.alpha, oklaba2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba() {
        let okhwba = Okhwba::new(180.0, 0.5, 0.5, 1.0);
        let srgba: Srgba = okhwba.into();
        let okhwba2: Okhwba = srgba.into();
        assert_approx_eq!(okhwba.hue, okhwba2.hue, 0.001);
        assert_approx_eq!(okhwba.whiteness, okhwba2.whiteness, 0.001);
        assert_approx_eq!(okhwba.blackness, okhwba2.blackness, 0.001);
        assert_approx_eq!(okhwba.alpha, okhwba2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_linear() {
        let okhwba = Okhwba::new(180.0, 0.5, 0.5, 1.0);
        let linear: LinearRgba = okhwba.into();
        let okhwba2: Okhwba = linear.into();
        assert_approx_eq!(okhwba.hue, okhwba2.hue, 0.001);
        assert_approx_eq!(okhwba.whiteness, okhwba2.whiteness, 0.001);
        assert_approx_eq!(okhwba.blackness, okhwba2.blackness, 0.001);
        assert_approx_eq!(okhwba.alpha, okhwba2.alpha, 0.001);
    }
}
