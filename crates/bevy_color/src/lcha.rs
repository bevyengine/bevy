use crate::{Alpha, LinearRgba, Luminance, Mix, Srgba};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::color::{Color, LchRepresentation};
use serde::{Deserialize, Serialize};

/// Color in LCH color space, with alpha
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Lcha {
    /// The lightness channel. [0.0, 1.5]
    pub lightness: f32,
    /// The chroma channel. [0.0, 1.5]
    pub chroma: f32,
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl Lcha {
    /// Construct a new [`Lcha`] color from components.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Self {
        Self {
            lightness,
            chroma,
            hue,
            alpha,
        }
    }

    /// Construct a new [`Lcha`] color from (h, s, l) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    pub const fn lch(lightness: f32, chroma: f32, hue: f32) -> Self {
        Self {
            lightness,
            chroma,
            hue,
            alpha: 1.0,
        }
    }
}

impl Default for Lcha {
    fn default() -> Self {
        Self::new(0., 0., 0., 1.)
    }
}

impl Mix for Lcha {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            lightness: self.lightness * n_factor + other.lightness * factor,
            chroma: self.chroma * n_factor + other.chroma * factor,
            hue: self.hue * n_factor + other.hue * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Alpha for Lcha {
    #[inline]
    fn with_alpha(&self, alpha: f32) -> Self {
        Self { alpha, ..*self }
    }
}

impl Luminance for Lcha {
    #[inline]
    fn with_luminance(&self, lightness: f32) -> Self {
        Self { lightness, ..*self }
    }

    fn luminance(&self) -> f32 {
        self.lightness
    }

    fn darker(&self, amount: f32) -> Self {
        Self::new(
            (self.lightness - amount).max(0.),
            self.chroma,
            self.hue,
            self.alpha,
        )
    }

    fn lighter(&self, amount: f32) -> Self {
        Self::new(
            (self.lightness + amount).min(1.),
            self.chroma,
            self.hue,
            self.alpha,
        )
    }
}

impl From<Srgba> for Lcha {
    fn from(value: Srgba) -> Self {
        let (l, c, h) =
            LchRepresentation::nonlinear_srgb_to_lch([value.red, value.green, value.blue]);
        Lcha::new(l, c, h, value.alpha)
    }
}

impl From<Lcha> for Srgba {
    fn from(value: Lcha) -> Self {
        let [r, g, b] =
            LchRepresentation::lch_to_nonlinear_srgb(value.lightness, value.chroma, value.hue);
        Srgba::new(r, g, b, value.alpha)
    }
}

impl From<LinearRgba> for Lcha {
    fn from(value: LinearRgba) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Lcha> for LinearRgba {
    fn from(value: Lcha) -> Self {
        LinearRgba::from(Srgba::from(value))
    }
}

impl From<Lcha> for Color {
    fn from(value: Lcha) -> Self {
        Color::Lcha {
            hue: value.hue,
            chroma: value.chroma,
            lightness: value.lightness,
            alpha: value.alpha,
        }
    }
}

impl From<Color> for Lcha {
    fn from(value: Color) -> Self {
        match value.as_lcha() {
            Color::Lcha {
                hue,
                chroma,
                lightness,
                alpha,
            } => Lcha::new(hue, chroma, lightness, alpha),
            _ => unreachable!(),
        }
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
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.lch).into();
            let lcha: Lcha = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.0001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            assert_approx_eq!(color.lch.lightness, lcha.lightness, 0.001);
            if lcha.lightness > 0.01 {
                assert_approx_eq!(color.lch.chroma, lcha.chroma, 0.1);
            }
            if lcha.lightness > 0.01 && lcha.chroma > 0.01 {
                assert!(
                    (color.lch.hue - lcha.hue).abs() < 1.7,
                    "{:?} != {:?}",
                    color.lch,
                    lcha
                );
            }
            assert_approx_eq!(color.lch.alpha, lcha.alpha, 0.001);
        }
    }

    #[test]
    fn test_to_from_linear() {
        for color in TEST_COLORS.iter() {
            let rgb2: LinearRgba = (color.lch).into();
            let lcha: Lcha = (color.linear_rgb).into();
            assert!(
                color.linear_rgb.distance(&rgb2) < 0.0001,
                "{}: {:?} != {:?}",
                color.name,
                color.linear_rgb,
                rgb2
            );
            assert_approx_eq!(color.lch.lightness, lcha.lightness, 0.001);
            if lcha.lightness > 0.01 {
                assert_approx_eq!(color.lch.chroma, lcha.chroma, 0.1);
            }
            if lcha.lightness > 0.01 && lcha.chroma > 0.01 {
                assert!(
                    (color.lch.hue - lcha.hue).abs() < 1.7,
                    "{:?} != {:?}",
                    color.lch,
                    lcha
                );
            }
            assert_approx_eq!(color.lch.alpha, lcha.alpha, 0.001);
        }
    }
}
