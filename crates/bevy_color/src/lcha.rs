use crate::{impl_bi_from_via, Alpha, Laba, LinearRgba, Luminance, Mix, Srgba, StandardColor, Xyza};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
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

impl StandardColor for Lcha {}

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

    /// Return a copy of this color with the hue channel set to the given value.
    pub const fn with_hue(self, hue: f32) -> Self {
        Self { hue, ..self }
    }

    /// Return a copy of this color with the chroma channel set to the given value.
    pub const fn with_chroma(self, chroma: f32) -> Self {
        Self { chroma, ..self }
    }

    /// Return a copy of this color with the lightness channel set to the given value.
    pub const fn with_lightness(self, lightness: f32) -> Self {
        Self { lightness, ..self }
    }
}

impl Default for Lcha {
    fn default() -> Self {
        Self::new(1., 0., 0., 1.)
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

    #[inline]
    fn alpha(&self) -> f32 {
        self.alpha
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

impl From<Lcha> for Laba {
    fn from(
        Lcha {
            lightness,
            chroma,
            hue,
            alpha,
        }: Lcha,
    ) -> Self {
        // Based on http://www.brucelindbloom.com/index.html?Eqn_LCH_to_Lab.html
        let l = lightness;
        let a = chroma * hue.to_radians().cos();
        let b = chroma * hue.to_radians().sin();

        Laba::new(l, a, b, alpha)
    }
}

impl From<Laba> for Lcha {
    fn from(Laba { lightness, a, b, alpha }: Laba) -> Self {
        // Based on http://www.brucelindbloom.com/index.html?Eqn_Lab_to_LCH.html
        let c = (a.powf(2.0) + b.powf(2.0)).sqrt();
        let h = {
            let h = b.to_radians().atan2(a.to_radians()).to_degrees();

            if h < 0.0 {
                h + 360.0
            } else {
                h
            }
        };

        let chroma = c.clamp(0.0, 1.5);
        let hue = h;

        Lcha::new(lightness, chroma, hue, alpha)
    }
}

impl_bi_from_via! {
    impl From<Srgba> for Lcha via Laba {}
    impl From<LinearRgba> for Lcha via Laba {}
    impl From<Xyza> for Lcha via Laba {}
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
