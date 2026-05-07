use crate::{
    okcolor_convert::{okhsv_to_oklab, oklab_to_okhsv},
    okhsla::Okhsla,
    Alpha, ColorToComponents, Gray, Hsla, Hsva, Hue, Hwba, Laba, Lcha, LinearRgba, Mix, Oklaba,
    Oklcha, Saturation, Srgba, StandardColor, Xyza,
};
use bevy_math::{Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Color in Okhsv color space with alpha.
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
        oklab_to_okhsv(value)
    }
}

impl From<Okhsva> for Oklaba {
    fn from(value: Okhsva) -> Self {
        okhsv_to_oklab(value)
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
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Srgba {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hwba> for Okhsva {
    fn from(value: Hwba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Hwba {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Lcha> for Okhsva {
    fn from(value: Lcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Lcha {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Xyza> for Okhsva {
    fn from(value: Xyza) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Xyza {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Okhsva {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Okhsla {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hsla> for Okhsva {
    fn from(value: Hsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Hsla {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hsva> for Okhsva {
    fn from(value: Hsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Hsva {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Laba> for Okhsva {
    fn from(value: Laba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Laba {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Okhsva {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsva> for Oklcha {
    fn from(value: Okhsva) -> Self {
        Oklaba::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        color_difference::EuclideanDistance, test_colors::TEST_COLORS, testing::assert_approx_eq,
    };

    #[test]
    fn test_to_from_srgba() {
        let okhsva = Okhsva::new(180.0, 0.5, 0.5, 1.0);
        let srgba: Srgba = okhsva.into();
        let okhsva2: Okhsva = srgba.into();
        assert_approx_eq!(okhsva.hue, okhsva2.hue, 0.001);
        assert_approx_eq!(okhsva.saturation, okhsva2.saturation, 0.001);
        assert_approx_eq!(okhsva.value, okhsva2.value, 0.001);
        assert_approx_eq!(okhsva.alpha, okhsva2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.okhsv).into();
            let okhsv: Okhsva = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2,
            );
            let msg = alloc::format!(
                "{}: expected {:?}, got {:?}",
                color.name,
                color.okhsv,
                okhsv
            );
            assert_approx_eq!(color.okhsv.hue, okhsv.hue, 0.001, msg);
            assert_approx_eq!(color.okhsv.saturation, okhsv.saturation, 0.001, msg);
            assert_approx_eq!(color.okhsv.value, okhsv.value, 0.001, msg);
            assert_approx_eq!(color.okhsv.alpha, okhsv.alpha, 0.001, msg);
        }
    }

    #[test]
    fn test_to_from_linear() {
        let okhsva = Okhsva::new(0.5, 0.5, 0.5, 1.0);
        let linear: LinearRgba = okhsva.into();
        let okhsva2: Okhsva = linear.into();
        assert_approx_eq!(okhsva.hue, okhsva2.hue, 0.001);
        assert_approx_eq!(okhsva.saturation, okhsva2.saturation, 0.001);
        assert_approx_eq!(okhsva.value, okhsva2.value, 0.001);
        assert_approx_eq!(okhsva.alpha, okhsva2.alpha, 0.001);
    }
}
