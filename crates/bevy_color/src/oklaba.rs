use crate::{
    color_difference::EuclideanDistance, impl_componentwise_vector_space, Alpha, ColorToComponents,
    Gray, Hsla, Hsva, Hwba, Lcha, LinearRgba, Luminance, Mix, Srgba, StandardColor, Xyza,
};
use bevy_math::{Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Color in Oklab color space, with alpha
#[doc = include_str!("../docs/conversion.md")]
/// <div>
#[doc = include_str!("../docs/diagrams/model_graph.svg")]
/// </div>
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(PartialEq, Default))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Oklaba {
    /// The 'lightness' channel. [0.0, 1.0]
    pub lightness: f32,
    /// The 'a' channel. [-1.0, 1.0]
    pub a: f32,
    /// The 'b' channel. [-1.0, 1.0]
    pub b: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Oklaba {}

impl_componentwise_vector_space!(Oklaba, [lightness, a, b, alpha]);

impl Oklaba {
    /// Construct a new [`Oklaba`] color from components.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `a` - Green-red channel. [-1.0, 1.0]
    /// * `b` - Blue-yellow channel. [-1.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(lightness: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self {
            lightness,
            a,
            b,
            alpha,
        }
    }

    /// Construct a new [`Oklaba`] color from (l, a, b) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `a` - Green-red channel. [-1.0, 1.0]
    /// * `b` - Blue-yellow channel. [-1.0, 1.0]
    pub const fn lab(lightness: f32, a: f32, b: f32) -> Self {
        Self {
            lightness,
            a,
            b,
            alpha: 1.0,
        }
    }

    /// Return a copy of this color with the 'lightness' channel set to the given value.
    pub const fn with_lightness(self, lightness: f32) -> Self {
        Self { lightness, ..self }
    }

    /// Return a copy of this color with the 'a' channel set to the given value.
    pub const fn with_a(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Return a copy of this color with the 'b' channel set to the given value.
    pub const fn with_b(self, b: f32) -> Self {
        Self { b, ..self }
    }
}

impl Default for Oklaba {
    fn default() -> Self {
        Self::new(1., 0., 0., 1.)
    }
}

impl Mix for Oklaba {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            lightness: self.lightness * n_factor + other.lightness * factor,
            a: self.a * n_factor + other.a * factor,
            b: self.b * n_factor + other.b * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for Oklaba {
    const BLACK: Self = Self::new(0., 0., 0., 1.);
    const WHITE: Self = Self::new(1.0, 0.0, 0.000000059604645, 1.0);
}

impl Alpha for Oklaba {
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

impl Luminance for Oklaba {
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
            self.a,
            self.b,
            self.alpha,
        )
    }

    fn lighter(&self, amount: f32) -> Self {
        Self::new(
            (self.lightness + amount).min(1.),
            self.a,
            self.b,
            self.alpha,
        )
    }
}

impl EuclideanDistance for Oklaba {
    #[inline]
    fn distance_squared(&self, other: &Self) -> f32 {
        (self.lightness - other.lightness).powi(2)
            + (self.a - other.a).powi(2)
            + (self.b - other.b).powi(2)
    }
}

impl ColorToComponents for Oklaba {
    fn to_f32_array(self) -> [f32; 4] {
        [self.lightness, self.a, self.b, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.lightness, self.a, self.b]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.lightness, self.a, self.b, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.lightness, self.a, self.b)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            lightness: color[0],
            a: color[1],
            b: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            lightness: color[0],
            a: color[1],
            b: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            lightness: color[0],
            a: color[1],
            b: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            lightness: color[0],
            a: color[1],
            b: color[2],
            alpha: 1.0,
        }
    }
}

#[allow(clippy::excessive_precision)]
impl From<LinearRgba> for Oklaba {
    fn from(value: LinearRgba) -> Self {
        let LinearRgba {
            red,
            green,
            blue,
            alpha,
        } = value;
        // From https://github.com/DougLau/pix
        let l = 0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue;
        let m = 0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue;
        let s = 0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue;
        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();
        let l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
        let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
        let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;
        Oklaba::new(l, a, b, alpha)
    }
}

#[allow(clippy::excessive_precision)]
impl From<Oklaba> for LinearRgba {
    fn from(value: Oklaba) -> Self {
        let Oklaba {
            lightness,
            a,
            b,
            alpha,
        } = value;

        // From https://github.com/Ogeon/palette/blob/e75eab2fb21af579353f51f6229a510d0d50a311/palette/src/oklab.rs#L312-L332
        let l_ = lightness + 0.3963377774 * a + 0.2158037573 * b;
        let m_ = lightness - 0.1055613458 * a - 0.0638541728 * b;
        let s_ = lightness - 0.0894841775 * a - 1.2914855480 * b;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let red = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
        let green = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
        let blue = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

// Derived Conversions

impl From<Hsla> for Oklaba {
    fn from(value: Hsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for Hsla {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hsva> for Oklaba {
    fn from(value: Hsva) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for Hsva {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hwba> for Oklaba {
    fn from(value: Hwba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for Hwba {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Lcha> for Oklaba {
    fn from(value: Lcha) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for Lcha {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Srgba> for Oklaba {
    fn from(value: Srgba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for Srgba {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Oklaba {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for Xyza {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_colors::TEST_COLORS, testing::assert_approx_eq};

    #[test]
    fn test_to_from_srgba() {
        let oklaba = Oklaba::new(0.5, 0.5, 0.5, 1.0);
        let srgba: Srgba = oklaba.into();
        let oklaba2: Oklaba = srgba.into();
        assert_approx_eq!(oklaba.lightness, oklaba2.lightness, 0.001);
        assert_approx_eq!(oklaba.a, oklaba2.a, 0.001);
        assert_approx_eq!(oklaba.b, oklaba2.b, 0.001);
        assert_approx_eq!(oklaba.alpha, oklaba2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.oklab).into();
            let oklab: Oklaba = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.0001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            assert!(
                color.oklab.distance(&oklab) < 0.0001,
                "{}: {:?} != {:?}",
                color.name,
                color.oklab,
                oklab
            );
        }
    }

    #[test]
    fn test_to_from_linear() {
        let oklaba = Oklaba::new(0.5, 0.5, 0.5, 1.0);
        let linear: LinearRgba = oklaba.into();
        let oklaba2: Oklaba = linear.into();
        assert_approx_eq!(oklaba.lightness, oklaba2.lightness, 0.001);
        assert_approx_eq!(oklaba.a, oklaba2.a, 0.001);
        assert_approx_eq!(oklaba.b, oklaba2.b, 0.001);
        assert_approx_eq!(oklaba.alpha, oklaba2.alpha, 0.001);
    }
}
