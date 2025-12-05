use crate::{
    color_difference::EuclideanDistance, impl_componentwise_vector_space, Alpha, ColorToComponents,
    Gray, Hsla, Hsva, Hwba, Lcha, LinearRgba, Luminance, Mix, Srgba, StandardColor, Xyza,
};
use bevy_math::{ops, FloatPow, Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Color in Oklab color space, with alpha
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
        (self.lightness - other.lightness).squared()
            + (self.a - other.a).squared()
            + (self.b - other.b).squared()
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

impl From<LinearRgba> for Oklaba {
    fn from(value: LinearRgba) -> Self {
        let LinearRgba {
            red,
            green,
            blue,
            alpha,
        } = value;
        // From https://bottosson.github.io/posts/oklab/#converting-from-linear-srgb-to-oklab
        // Float literals are truncated to avoid excessive precision.
        let l = 0.41222146 * red + 0.53633255 * green + 0.051445995 * blue;
        let m = 0.2119035 * red + 0.6806995 * green + 0.10739696 * blue;
        let s = 0.08830246 * red + 0.28171885 * green + 0.6299787 * blue;
        let l_ = ops::cbrt(l);
        let m_ = ops::cbrt(m);
        let s_ = ops::cbrt(s);
        let l = 0.21045426 * l_ + 0.7936178 * m_ - 0.004072047 * s_;
        let a = 1.9779985 * l_ - 2.4285922 * m_ + 0.4505937 * s_;
        let b = 0.025904037 * l_ + 0.78277177 * m_ - 0.80867577 * s_;
        Oklaba::new(l, a, b, alpha)
    }
}

impl From<Oklaba> for LinearRgba {
    fn from(value: Oklaba) -> Self {
        let Oklaba {
            lightness,
            a,
            b,
            alpha,
        } = value;

        // From https://bottosson.github.io/posts/oklab/#converting-from-linear-srgb-to-oklab
        // Float literals are truncated to avoid excessive precision.
        let l_ = lightness + 0.39633778 * a + 0.21580376 * b;
        let m_ = lightness - 0.105561346 * a - 0.06385417 * b;
        let s_ = lightness - 0.08948418 * a - 1.2914855 * b;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let red = 4.0767417 * l - 3.3077116 * m + 0.23096994 * s;
        let green = -1.268438 * l + 2.6097574 * m - 0.34131938 * s;
        let blue = -0.0041960863 * l - 0.7034186 * m + 1.7076147 * s;

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
