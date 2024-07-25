use crate::{
    Alpha, ColorToComponents, Gray, Hue, Laba, LinearRgba, Luminance, Mix, Srgba, StandardColor,
    Xyza,
};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::prelude::*;

/// Color in LCH color space, with alpha
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

    /// Return a copy of this color with the chroma channel set to the given value.
    pub const fn with_chroma(self, chroma: f32) -> Self {
        Self { chroma, ..self }
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
    /// # use bevy_color::Lcha;
    /// // Unique color for an entity
    /// # let entity_index = 123;
    /// // let entity_index = entity.index();
    /// let color = Lcha::sequential_dispersed(entity_index);
    ///
    /// // Palette with 5 distinct hues
    /// let palette = (0..5).map(Lcha::sequential_dispersed).collect::<Vec<_>>();
    /// ```
    pub fn sequential_dispersed(index: u32) -> Self {
        const FRAC_U32MAX_GOLDEN_RATIO: u32 = 2654435769; // (u32::MAX / Φ) rounded up
        const RATIO_360: f32 = 360.0 / u32::MAX as f32;

        // from https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
        //
        // Map a sequence of integers (eg: 154, 155, 156, 157, 158) into the [0.0..1.0] range,
        // so that the closer the numbers are, the larger the difference of their image.
        let hue = index.wrapping_mul(FRAC_U32MAX_GOLDEN_RATIO) as f32 * RATIO_360;
        Self::lch(0.75, 0.35, hue)
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
            hue: crate::color_ops::lerp_hue(self.hue, other.hue, factor),
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for Lcha {
    const BLACK: Self = Self::new(0.0, 0.0, 0.0000136603785, 1.0);
    const WHITE: Self = Self::new(1.0, 0.0, 0.0000136603785, 1.0);
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

    #[inline]
    fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }
}

impl Hue for Lcha {
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

impl ColorToComponents for Lcha {
    fn to_f32_array(self) -> [f32; 4] {
        [self.lightness, self.chroma, self.hue, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.lightness, self.chroma, self.hue]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.lightness, self.chroma, self.hue, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.lightness, self.chroma, self.hue)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            lightness: color[0],
            chroma: color[1],
            hue: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            lightness: color[0],
            chroma: color[1],
            hue: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            lightness: color[0],
            chroma: color[1],
            hue: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            lightness: color[0],
            chroma: color[1],
            hue: color[2],
            alpha: 1.0,
        }
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
    fn from(
        Laba {
            lightness,
            a,
            b,
            alpha,
        }: Laba,
    ) -> Self {
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

// Derived Conversions

impl From<Srgba> for Lcha {
    fn from(value: Srgba) -> Self {
        Laba::from(value).into()
    }
}

impl From<Lcha> for Srgba {
    fn from(value: Lcha) -> Self {
        Laba::from(value).into()
    }
}

impl From<LinearRgba> for Lcha {
    fn from(value: LinearRgba) -> Self {
        Laba::from(value).into()
    }
}

impl From<Lcha> for LinearRgba {
    fn from(value: Lcha) -> Self {
        Laba::from(value).into()
    }
}

impl From<Xyza> for Lcha {
    fn from(value: Xyza) -> Self {
        Laba::from(value).into()
    }
}

impl From<Lcha> for Xyza {
    fn from(value: Lcha) -> Self {
        Laba::from(value).into()
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
