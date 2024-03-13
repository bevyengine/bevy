use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use crate::{
    add_alpha_blend, sub_alpha_blend, Alpha, Hwba, Lcha, LinearRgba, Srgba, StandardColor, Xyza,
};
use bevy_math::cubic_splines::Point;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

/// Color in Hue-Saturation-Value (HSV) color space with alpha.
/// Further information on this color model can be found on [Wikipedia](https://en.wikipedia.org/wiki/HSL_and_HSV).
#[doc = include_str!("../docs/conversion.md")]
/// <div>
#[doc = include_str!("../docs/diagrams/model_graph.svg")]
/// </div>
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
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

    /// Return a copy of this color with the hue channel set to the given value.
    pub const fn with_hue(self, hue: f32) -> Self {
        Self { hue, ..self }
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

/// All color channels are added directly
/// but alpha is blended
///
/// Values are not clamped
/// but hue is in `0..360`
impl Add<Hsva> for Hsva {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            hue: (self.hue + rhs.hue).rem_euclid(360.),
            value: self.value + rhs.value,
            saturation: self.saturation + rhs.saturation,
            alpha: add_alpha_blend(self.alpha, rhs.alpha),
        }
    }
}

/// All color channels are added directly
/// but alpha is blended
///
/// Values are not clamped
/// but hue is in `0..360`
impl AddAssign<Self> for Hsva {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

/// All color channels are subtracted directly
/// but alpha is blended
///
/// Values are not clamped
/// but hue is in `0..360`
impl Sub<Hsva> for Hsva {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            hue: (self.hue - rhs.hue).rem_euclid(360.),
            value: self.value - rhs.value,
            saturation: self.saturation - rhs.saturation,
            alpha: sub_alpha_blend(self.alpha, rhs.alpha),
        }
    }
}

/// All color channels are subtracted directly
/// but alpha is blended
///
/// Values are not clamped
/// but hue is in `0..360`
impl SubAssign<Self> for Hsva {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

/// All color channels are scaled directly,
/// but alpha is unchanged.
///
/// Values are not clamped.
impl Mul<f32> for Hsva {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::Output {
            hue: (self.hue * rhs).rem_euclid(360.),
            value: self.value * rhs,
            saturation: self.saturation * rhs,
            alpha: self.alpha,
        }
    }
}

/// All color channels are scaled directly,
/// but alpha is unchanged.
///
/// Values are not clamped.
impl Mul<Hsva> for f32 {
    type Output = Hsva;

    fn mul(self, rhs: Hsva) -> Self::Output {
        rhs * self
    }
}

/// All color channels are scaled directly,
/// but alpha is unchanged.
///
/// Values are not clamped.
impl Div<f32> for Hsva {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::Output {
            hue: (self.hue / rhs).rem_euclid(360.),
            value: self.value / rhs,
            saturation: self.saturation / rhs,
            alpha: self.alpha,
        }
    }
}

/// All color channels are negated directly,
/// but alpha is unchanged.
///
/// Values are not clamped
impl Neg for Hsva {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::Output {
            hue: 360. - self.hue,
            saturation: -self.saturation,
            value: -self.value,
            alpha: self.alpha,
        }
    }
}

impl Point for Hsva {}

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
        Srgba,
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
}
