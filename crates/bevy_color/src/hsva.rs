use crate::{Alpha, ClampColor, Convert, Hue, Hwba, Lcha, LinearRgba, Mix, Srgba, StandardColor, Xyza};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::prelude::*;

/// Color in Hue-Saturation-Value (HSV) color space with alpha.
/// Further information on this color model can be found on [Wikipedia](https://en.wikipedia.org/wiki/HSL_and_HSV).
#[doc = include_str!("../docs/conversion.md")]
/// <div>
#[doc = include_str!("../docs/diagrams/model_graph.svg")]
/// </div>
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(PartialEq, Default)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Hsva {
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The saturation channel. [0.0, 1.0]
    pub saturation: f32,
    /// The value channel. [0.0, 1.0]
    pub value: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Hsva {}

impl Hsva {
    /// Construct a new [`Hsva`] color from components.
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

    /// Construct a new [`Hsva`] color from (h, s, v) components, with the default alpha (1.0).
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

impl Default for Hsva {
    fn default() -> Self {
        Self::new(0., 0., 1., 1.)
    }
}

impl Mix for Hsva {
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

impl Alpha for Hsva {
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

impl Hue for Hsva {
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

impl ClampColor for Hsva {
    fn clamped(&self) -> Self {
        Self {
            hue: self.hue.rem_euclid(360.),
            saturation: self.saturation.clamp(0., 1.),
            value: self.value.clamp(0., 1.),
            alpha: self.alpha.clamp(0., 1.),
        }
    }

    fn is_within_bounds(&self) -> bool {
        (0. ..=360.).contains(&self.hue)
            && (0. ..=1.).contains(&self.saturation)
            && (0. ..=1.).contains(&self.value)
            && (0. ..=1.).contains(&self.alpha)
    }
}

impl From<Hsva> for Hwba {
    fn from(
        Hsva {
            hue,
            saturation,
            value,
            alpha,
        }: Hsva,
    ) -> Self {
        // Based on https://en.wikipedia.org/wiki/HWB_color_model#Conversion
        let whiteness = (1. - saturation) * value;
        let blackness = 1. - value;

        Hwba::new(hue, whiteness, blackness, alpha)
    }
}

impl From<Hwba> for Hsva {
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
        let saturation = 1. - (whiteness / value);

        Hsva::new(hue, saturation, value, alpha)
    }
}

impl Convert for Hsva {
    fn to_f32_array(self) -> [f32; 4] {
        [self.hue, self.saturation, self.value, self.alpha]
    }

    fn to_alphaless_array(self) -> [f32; 3] {
        [self.hue, self.saturation, self.value]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.hue, self.saturation, self.value, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.hue, self.saturation, self.value)
    }

    fn from_array(color: [f32; 4]) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            value: color[2],
            alpha: color[3],
        }
    }

    fn from_alphaless_array(color: [f32; 3]) -> Self {
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

// Derived Conversions

impl From<Srgba> for Hsva {
    fn from(value: Srgba) -> Self {
        Hwba::from(value).into()
    }
}

impl From<Hsva> for Srgba {
    fn from(value: Hsva) -> Self {
        Hwba::from(value).into()
    }
}

impl From<LinearRgba> for Hsva {
    fn from(value: LinearRgba) -> Self {
        Hwba::from(value).into()
    }
}

impl From<Hsva> for LinearRgba {
    fn from(value: Hsva) -> Self {
        Hwba::from(value).into()
    }
}

impl From<Lcha> for Hsva {
    fn from(value: Lcha) -> Self {
        Hwba::from(value).into()
    }
}

impl From<Hsva> for Lcha {
    fn from(value: Hsva) -> Self {
        Hwba::from(value).into()
    }
}

impl From<Xyza> for Hsva {
    fn from(value: Xyza) -> Self {
        Hwba::from(value).into()
    }
}

impl From<Hsva> for Xyza {
    fn from(value: Hsva) -> Self {
        Hwba::from(value).into()
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
        let hsva = Hsva::new(180., 0.5, 0.5, 1.0);
        let srgba: Srgba = hsva.into();
        let hsva2: Hsva = srgba.into();
        assert_approx_eq!(hsva.hue, hsva2.hue, 0.001);
        assert_approx_eq!(hsva.saturation, hsva2.saturation, 0.001);
        assert_approx_eq!(hsva.value, hsva2.value, 0.001);
        assert_approx_eq!(hsva.alpha, hsva2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.hsv).into();
            let hsv2: Hsva = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.00001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            assert_approx_eq!(color.hsv.hue, hsv2.hue, 0.001);
            assert_approx_eq!(color.hsv.saturation, hsv2.saturation, 0.001);
            assert_approx_eq!(color.hsv.value, hsv2.value, 0.001);
            assert_approx_eq!(color.hsv.alpha, hsv2.alpha, 0.001);
        }
    }

    #[test]
    fn test_clamp() {
        let color_1 = Hsva::hsv(361., 2., -1.);
        let color_2 = Hsva::hsv(250.2762, 1., 0.67);
        let mut color_3 = Hsva::hsv(-50., 1., 1.);

        assert!(!color_1.is_within_bounds());
        assert_eq!(color_1.clamped(), Hsva::hsv(1., 1., 0.));

        assert!(color_2.is_within_bounds());
        assert_eq!(color_2, color_2.clamped());

        color_3.clamp();
        assert!(color_3.is_within_bounds());
        assert_eq!(color_3, Hsva::hsv(310., 1., 1.));
    }
}
