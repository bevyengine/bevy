use crate::{Alpha, LinearRgba, Luminance, Mix, Srgba};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::color::HslRepresentation;
use serde::{Deserialize, Serialize};

/// Color in Hue-Saturation-Lightness color space with alpha
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

impl From<Srgba> for Hsla {
    fn from(value: Srgba) -> Self {
        let (h, s, l) =
            HslRepresentation::nonlinear_srgb_to_hsl([value.red, value.green, value.blue]);
        Self::new(h, s, l, value.alpha)
    }
}

impl From<LinearRgba> for Hsla {
    fn from(value: LinearRgba) -> Self {
        Hsla::from(Srgba::from(value))
    }
}

impl From<Hsla> for bevy_render::color::Color {
    fn from(value: Hsla) -> Self {
        bevy_render::color::Color::Hsla {
            hue: value.hue,
            saturation: value.saturation,
            lightness: value.lightness,
            alpha: value.alpha,
        }
    }
}

impl From<bevy_render::color::Color> for Hsla {
    fn from(value: bevy_render::color::Color) -> Self {
        match value.as_hsla() {
            bevy_render::color::Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Hsla::new(hue, saturation, lightness, alpha),
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
