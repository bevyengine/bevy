use crate::{
    color_difference::EuclideanDistance, Alpha, ColorToComponents, Gray, Hsla, Hsva, Hue, Hwba,
    Laba, Lcha, LinearRgba, Luminance, Mix, Oklaba, Srgba, StandardColor, Xyza,
};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::prelude::*;

/// Color in Oklch color space, with alpha
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
pub struct Oklcha {
    /// The 'lightness' channel. [0.0, 1.0]
    pub lightness: f32,
    /// The 'chroma' channel. [0.0, 1.0]
    pub chroma: f32,
    /// The 'hue' channel. [0.0, 360.0]
    pub hue: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Oklcha {}

impl Oklcha {
    /// Construct a new [`Oklcha`] color from components.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `chroma` - Chroma channel. [0.0, 1.0]
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

    /// Construct a new [`Oklcha`] color from (l, c, h) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `chroma` - Chroma channel. [0.0, 1.0]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn lch(lightness: f32, chroma: f32, hue: f32) -> Self {
        Self::new(lightness, chroma, hue, 1.0)
    }

    /// Return a copy of this color with the 'lightness' channel set to the given value.
    pub const fn with_lightness(self, lightness: f32) -> Self {
        Self { lightness, ..self }
    }

    /// Return a copy of this color with the 'chroma' channel set to the given value.
    pub const fn with_chroma(self, chroma: f32) -> Self {
        Self { chroma, ..self }
    }

    /// Generate a deterministic but [quasi-randomly distributed](https://en.wikipedia.org/wiki/Low-discrepancy_sequence)
    /// color from a provided `index`.
    ///
    /// This can be helpful for generating debug colors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_color::Oklcha;
    /// // Unique color for an entity
    /// # let entity_index = 123;
    /// // let entity_index = entity.index();
    /// let color = Oklcha::sequential_dispersed(entity_index);
    ///
    /// // Palette with 5 distinct hues
    /// let palette = (0..5).map(Oklcha::sequential_dispersed).collect::<Vec<_>>();
    /// ```
    pub fn sequential_dispersed(index: u32) -> Self {
        const FRAC_U32MAX_GOLDEN_RATIO: u32 = 2654435769; // (u32::MAX / Φ) rounded up
        const RATIO_360: f32 = 360.0 / u32::MAX as f32;

        // from https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
        //
        // Map a sequence of integers (eg: 154, 155, 156, 157, 158) into the [0.0..1.0] range,
        // so that the closer the numbers are, the larger the difference of their image.
        let hue = index.wrapping_mul(FRAC_U32MAX_GOLDEN_RATIO) as f32 * RATIO_360;
        Self::lch(0.75, 0.1, hue)
    }
}

impl Default for Oklcha {
    fn default() -> Self {
        Self::new(1., 0., 0., 1.)
    }
}

impl Mix for Oklcha {
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

impl Gray for Oklcha {
    const BLACK: Self = Self::new(0., 0., 0., 1.);
    const WHITE: Self = Self::new(1.0, 0.000000059604645, 90.0, 1.0);
}

impl Alpha for Oklcha {
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

impl Hue for Oklcha {
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

impl Luminance for Oklcha {
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

impl EuclideanDistance for Oklcha {
    #[inline]
    fn distance_squared(&self, other: &Self) -> f32 {
        (self.lightness - other.lightness).powi(2)
            + (self.chroma - other.chroma).powi(2)
            + (self.hue - other.hue).powi(2)
    }
}

impl ColorToComponents for Oklcha {
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

impl From<Oklaba> for Oklcha {
    fn from(
        Oklaba {
            lightness,
            a,
            b,
            alpha,
        }: Oklaba,
    ) -> Self {
        let chroma = a.hypot(b);
        let hue = b.atan2(a).to_degrees();

        let hue = if hue < 0.0 { hue + 360.0 } else { hue };

        Oklcha::new(lightness, chroma, hue, alpha)
    }
}

impl From<Oklcha> for Oklaba {
    fn from(
        Oklcha {
            lightness,
            chroma,
            hue,
            alpha,
        }: Oklcha,
    ) -> Self {
        let l = lightness;
        let a = chroma * hue.to_radians().cos();
        let b = chroma * hue.to_radians().sin();

        Oklaba::new(l, a, b, alpha)
    }
}

// Derived Conversions

impl From<Hsla> for Oklcha {
    fn from(value: Hsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Hsla {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hsva> for Oklcha {
    fn from(value: Hsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Hsva {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hwba> for Oklcha {
    fn from(value: Hwba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Hwba {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Laba> for Oklcha {
    fn from(value: Laba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Laba {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Lcha> for Oklcha {
    fn from(value: Lcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Lcha {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<LinearRgba> for Oklcha {
    fn from(value: LinearRgba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for LinearRgba {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Srgba> for Oklcha {
    fn from(value: Srgba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Srgba {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Xyza> for Oklcha {
    fn from(value: Xyza) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Xyza {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_colors::TEST_COLORS, testing::assert_approx_eq};

    #[test]
    fn test_to_from_srgba() {
        let oklcha = Oklcha::new(0.5, 0.5, 180.0, 1.0);
        let srgba: Srgba = oklcha.into();
        let oklcha2: Oklcha = srgba.into();
        assert_approx_eq!(oklcha.lightness, oklcha2.lightness, 0.001);
        assert_approx_eq!(oklcha.chroma, oklcha2.chroma, 0.001);
        assert_approx_eq!(oklcha.hue, oklcha2.hue, 0.001);
        assert_approx_eq!(oklcha.alpha, oklcha2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.oklch).into();
            let oklch: Oklcha = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.0001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            assert!(
                color.oklch.distance(&oklch) < 0.0001,
                "{}: {:?} != {:?}",
                color.name,
                color.oklch,
                oklch
            );
        }
    }

    #[test]
    fn test_to_from_linear() {
        let oklcha = Oklcha::new(0.5, 0.5, 0.5, 1.0);
        let linear: LinearRgba = oklcha.into();
        let oklcha2: Oklcha = linear.into();
        assert_approx_eq!(oklcha.lightness, oklcha2.lightness, 0.001);
        assert_approx_eq!(oklcha.chroma, oklcha2.chroma, 0.001);
        assert_approx_eq!(oklcha.hue, oklcha2.hue, 0.001);
        assert_approx_eq!(oklcha.alpha, oklcha2.alpha, 0.001);
    }
}
