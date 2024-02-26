use crate::{Alpha, Hsva, Hwba, Lcha, LinearRgba, Luminance, Mix, Oklaba, Srgba, StandardColor};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

/// Color in Hue-Saturation-Lightness (HSL) color space with alpha.
/// Further information on this color model can be found on [Wikipedia](https://en.wikipedia.org/wiki/HSL_and_HSV).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Hsla {
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The saturation channel. [0.0, 1.0]
    pub saturation: f32,
    /// The lightness channel. [0.0, 1.0]
    pub lightness: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Hsla {}

impl Hsla {
    /// Construct a new [`Hsla`] color from components.
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
    pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> Self {
        Self::new(hue, saturation, lightness, 1.0)
    }

    /// Return a copy of this color with the hue channel set to the given value.
    pub const fn with_hue(self, hue: f32) -> Self {
        Self { hue, ..self }
    }

    /// Return a copy of this color with the saturation channel set to the given value.
    pub const fn with_saturation(self, saturation: f32) -> Self {
        Self { saturation, ..self }
    }

    /// Return a copy of this color with the lightness channel set to the given value.
    pub const fn with_lightness(self, lightness: f32) -> Self {
        Self { lightness, ..self }
    }
}

impl Default for Hsla {
    fn default() -> Self {
        Self::new(0., 0., 1., 1.)
    }
}

impl Mix for Hsla {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        // TODO: Refactor this into EuclideanModulo::lerp_modulo
        let shortest_angle = ((((other.hue - self.hue) % 360.) + 540.) % 360.) - 180.;
        let mut hue = self.hue + shortest_angle * factor;
        if hue < 0. {
            hue += 360.;
        } else if hue >= 360. {
            hue -= 360.;
        }
        Self {
            hue,
            saturation: self.saturation * n_factor + other.saturation * factor,
            lightness: self.lightness * n_factor + other.lightness * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Alpha for Hsla {
    #[inline]
    fn with_alpha(&self, alpha: f32) -> Self {
        Self { alpha, ..*self }
    }

    #[inline]
    fn alpha(&self) -> f32 {
        self.alpha
    }
}

impl Luminance for Hsla {
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

impl From<Hsla> for Hsva {
    fn from(
        Hsla {
            hue,
            saturation,
            lightness,
            alpha,
        }: Hsla,
    ) -> Self {
        // Based on https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_HSV
        let value = lightness + saturation * lightness.min(1. - lightness);
        let saturation = if value == 0. {
            0.
        } else {
            2. * (1. - (lightness / value))
        };

        Hsva::new(hue, saturation, value, alpha)
    }
}

impl From<Hsva> for Hsla {
    fn from(
        Hsva {
            hue,
            saturation,
            value,
            alpha,
        }: Hsva,
    ) -> Self {
        // Based on https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_HSL
        let lightness = value * (1. - saturation / 2.);
        let saturation = if lightness == 0. || lightness == 1. {
            0.
        } else {
            (value - lightness) / lightness.min(1. - lightness)
        };

        Hsla::new(hue, saturation, lightness, alpha)
    }
}

impl From<Hwba> for Hsla {
    fn from(value: Hwba) -> Self {
        Hsva::from(value).into()
    }
}

impl From<Srgba> for Hsla {
    fn from(value: Srgba) -> Self {
        Hsva::from(value).into()
    }
}

impl From<Hsla> for Srgba {
    fn from(value: Hsla) -> Self {
        Hsva::from(value).into()
    }
}

impl From<Hsla> for Hwba {
    fn from(value: Hsla) -> Self {
        Hsva::from(value).into()
    }
}

impl From<LinearRgba> for Hsla {
    fn from(value: LinearRgba) -> Self {
        Hsva::from(value).into()
    }
}

impl From<Oklaba> for Hsla {
    fn from(value: Oklaba) -> Self {
        Hsva::from(value).into()
    }
}

impl From<Lcha> for Hsla {
    fn from(value: Lcha) -> Self {
        Hsva::from(value).into()
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
        let hsla = Hsla::new(0.5, 0.5, 0.5, 1.0);
        let srgba: Srgba = hsla.into();
        let hsla2: Hsla = srgba.into();
        assert_approx_eq!(hsla.hue, hsla2.hue, 0.001);
        assert_approx_eq!(hsla.saturation, hsla2.saturation, 0.001);
        assert_approx_eq!(hsla.lightness, hsla2.lightness, 0.001);
        assert_approx_eq!(hsla.alpha, hsla2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.hsl).into();
            let hsl2: Hsla = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.000001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            assert_approx_eq!(color.hsl.hue, hsl2.hue, 0.001);
            assert_approx_eq!(color.hsl.saturation, hsl2.saturation, 0.001);
            assert_approx_eq!(color.hsl.lightness, hsl2.lightness, 0.001);
            assert_approx_eq!(color.hsl.alpha, hsl2.alpha, 0.001);
        }
    }

    #[test]
    fn test_to_from_linear() {
        let hsla = Hsla::new(0.5, 0.5, 0.5, 1.0);
        let linear: LinearRgba = hsla.into();
        let hsla2: Hsla = linear.into();
        assert_approx_eq!(hsla.hue, hsla2.hue, 0.001);
        assert_approx_eq!(hsla.saturation, hsla2.saturation, 0.001);
        assert_approx_eq!(hsla.lightness, hsla2.lightness, 0.001);
        assert_approx_eq!(hsla.alpha, hsla2.alpha, 0.001);
    }

    #[test]
    fn test_mix_wrap() {
        let hsla0 = Hsla::new(10., 0.5, 0.5, 1.0);
        let hsla1 = Hsla::new(20., 0.5, 0.5, 1.0);
        let hsla2 = Hsla::new(350., 0.5, 0.5, 1.0);
        assert_approx_eq!(hsla0.mix(&hsla1, 0.25).hue, 12.5, 0.001);
        assert_approx_eq!(hsla0.mix(&hsla1, 0.5).hue, 15., 0.001);
        assert_approx_eq!(hsla0.mix(&hsla1, 0.75).hue, 17.5, 0.001);

        assert_approx_eq!(hsla1.mix(&hsla0, 0.25).hue, 17.5, 0.001);
        assert_approx_eq!(hsla1.mix(&hsla0, 0.5).hue, 15., 0.001);
        assert_approx_eq!(hsla1.mix(&hsla0, 0.75).hue, 12.5, 0.001);

        assert_approx_eq!(hsla0.mix(&hsla2, 0.25).hue, 5., 0.001);
        assert_approx_eq!(hsla0.mix(&hsla2, 0.5).hue, 0., 0.001);
        assert_approx_eq!(hsla0.mix(&hsla2, 0.75).hue, 355., 0.001);

        assert_approx_eq!(hsla2.mix(&hsla0, 0.25).hue, 355., 0.001);
        assert_approx_eq!(hsla2.mix(&hsla0, 0.5).hue, 0., 0.001);
        assert_approx_eq!(hsla2.mix(&hsla0, 0.75).hue, 5., 0.001);
    }
}
