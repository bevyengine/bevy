use crate::{
    impl_componentwise_vector_space,
    okcolor_convert::{okhsl_to_oklab, oklab_to_okhsl},
    Alpha, ColorToComponents, Gray, Hsla, Hsva, Hue, Hwba, Laba, Lcha, LinearRgba, Luminance, Mix,
    Oklaba, Oklcha, Saturation, Srgba, StandardColor, Xyza,
};
use bevy_math::{Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Color in Okhsl color space with alpha
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
pub struct Okhsla {
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The saturation channel. [0.0, 1.0]
    pub saturation: f32,
    /// The lightness channel. [0.0, 1.0]
    pub lightness: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Okhsla {}

impl_componentwise_vector_space!(Okhsla, [hue, saturation, lightness, alpha]);

impl Okhsla {
    /// Construct a new [`Okhsla`] color from components.
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

    /// Construct a new [`Okhsla`] color from (h, s, l) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> Self {
        Self::new(hue, saturation, lightness, 1.0)
    }

    /// Return a copy of this color with the saturation channel set to the given value.
    pub const fn with_saturation(self, saturation: f32) -> Self {
        Self { saturation, ..self }
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
    /// # use bevy_color::Okhsla;
    /// // Unique color for an entity
    /// # let entity_index = 123;
    /// // let entity_index = entity.index();
    /// let color = Okhsla::sequential_dispersed(entity_index);
    ///
    /// // Palette with 5 distinct hues
    /// let palette = (0..5).map(Okhsla::sequential_dispersed).collect::<Vec<_>>();
    /// ```
    pub const fn sequential_dispersed(index: u32) -> Self {
        const FRAC_U32MAX_GOLDEN_RATIO: u32 = 2654435769; // (u32::MAX / Φ) rounded up
        const RATIO_360: f32 = 360.0 / u32::MAX as f32;

        // from https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
        //
        // Map a sequence of integers (eg: 154, 155, 156, 157, 158) into the [0.0..1.0] range,
        // so that the closer the numbers are, the larger the difference of their image.
        let hue = index.wrapping_mul(FRAC_U32MAX_GOLDEN_RATIO) as f32 * RATIO_360;
        Self::hsl(hue, 1., 0.5)
    }
}

impl Default for Okhsla {
    fn default() -> Self {
        Self::new(0., 0., 1., 1.)
    }
}

impl Mix for Okhsla {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            hue: crate::color_ops::lerp_hue(self.hue, other.hue, factor),
            saturation: self.saturation * n_factor + other.saturation * factor,
            lightness: self.lightness * n_factor + other.lightness * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for Okhsla {
    const BLACK: Self = Self::new(0., 0., 0., 1.);
    const WHITE: Self = Self::new(0., 0., 1., 1.);
}

impl Alpha for Okhsla {
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

impl Hue for Okhsla {
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

impl Saturation for Okhsla {
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

impl Luminance for Okhsla {
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

impl ColorToComponents for Okhsla {
    fn to_f32_array(self) -> [f32; 4] {
        [self.hue, self.saturation, self.lightness, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.hue, self.saturation, self.lightness]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.hue, self.saturation, self.lightness, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.hue, self.saturation, self.lightness)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            hue: color[0],
            saturation: color[1],
            lightness: color[2],
            alpha: 1.0,
        }
    }
}

#[cfg(feature = "wgpu-types")]
impl From<Okhsla> for wgpu_types::Color {
    fn from(color: Okhsla) -> Self {
        wgpu_types::Color {
            r: color.hue as f64,
            g: color.saturation as f64,
            b: color.lightness as f64,
            a: color.alpha as f64,
        }
    }
}

impl From<Oklaba> for Okhsla {
    fn from(value: Oklaba) -> Self {
        oklab_to_okhsl(value)
    }
}

impl From<Okhsla> for Oklaba {
    fn from(value: Okhsla) -> Self {
        okhsl_to_oklab(value)
    }
}

// Derived Conversions

impl From<LinearRgba> for Okhsla {
    fn from(value: LinearRgba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for LinearRgba {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hsla> for Okhsla {
    fn from(value: Hsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Hsla {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hsva> for Okhsla {
    fn from(value: Hsva) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Hsva {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Hwba> for Okhsla {
    fn from(value: Hwba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Hwba {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Lcha> for Okhsla {
    fn from(value: Lcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Lcha {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Srgba> for Okhsla {
    fn from(value: Srgba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Srgba {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Xyza> for Okhsla {
    fn from(value: Xyza) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Xyza {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Laba> for Okhsla {
    fn from(value: Laba) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Laba {
    fn from(value: Okhsla) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Oklcha> for Okhsla {
    fn from(value: Oklcha) -> Self {
        Oklaba::from(value).into()
    }
}

impl From<Okhsla> for Oklcha {
    fn from(value: Okhsla) -> Self {
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
        let okhsla = Okhsla::new(180.0, 0.5, 0.5, 1.0);
        let srgba: Srgba = okhsla.into();
        let okhsla2: Okhsla = srgba.into();
        assert_approx_eq!(okhsla.hue, okhsla2.hue, 0.001);
        assert_approx_eq!(okhsla.saturation, okhsla2.saturation, 0.001);
        assert_approx_eq!(okhsla.lightness, okhsla2.lightness, 0.001);
        assert_approx_eq!(okhsla.alpha, okhsla2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.okhsl).into();
            let okhsl: Okhsla = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            let msg = alloc::format!(
                "{}: expected {:?}, got {:?}",
                color.name,
                color.okhsl,
                okhsl
            );
            assert_approx_eq!(color.okhsl.hue, okhsl.hue, 0.001, msg);
            assert_approx_eq!(color.okhsl.saturation, okhsl.saturation, 0.001, msg);
            assert_approx_eq!(color.okhsl.lightness, okhsl.lightness, 0.001, msg);
            assert_approx_eq!(color.okhsl.alpha, okhsl.alpha, 0.001, msg);
        }
    }

    #[test]
    fn test_to_from_linear() {
        let okhsla = Okhsla::new(180.0, 0.5, 0.5, 1.0);
        let linear: LinearRgba = okhsla.into();
        let okhsla2: Okhsla = linear.into();
        assert_approx_eq!(okhsla.hue, okhsla2.hue, 0.001);
        assert_approx_eq!(okhsla.saturation, okhsla2.saturation, 0.001);
        assert_approx_eq!(okhsla.lightness, okhsla2.lightness, 0.001);
        assert_approx_eq!(okhsla.alpha, okhsla2.alpha, 0.001);
    }
}
