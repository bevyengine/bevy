use crate::{
    color_difference::EuclideanDistance, Alpha, Hsla, Lcha, LinearRgba, Luminance, Mix, Srgba,
    StandardColor,
};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

/// Color in Oklaba color space, with alpha
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Oklaba {
    /// The 'l' channel. [0.0, 1.0]
    pub l: f32,
    /// The 'a' channel. [-1.0, 1.0]
    pub a: f32,
    /// The 'b' channel. [-1.0, 1.0]
    pub b: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Oklaba {}

impl Oklaba {
    /// Construct a new [`Oklaba`] color from components.
    ///
    /// # Arguments
    ///
    /// * `l` - Lightness channel. [0.0, 1.0]
    /// * `a` - Green-red channel. [-1.0, 1.0]
    /// * `b` - Blue-yellow channel. [-1.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(l: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self { l, a, b, alpha }
    }

    /// Construct a new [`Oklaba`] color from (l, a, b) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `l` - Lightness channel. [0.0, 1.0]
    /// * `a` - Green-red channel. [-1.0, 1.0]
    /// * `b` - Blue-yellow channel. [-1.0, 1.0]
    pub const fn lch(l: f32, a: f32, b: f32) -> Self {
        Self {
            l,
            a,
            b,
            alpha: 1.0,
        }
    }

    /// Return a copy of this color with the 'l' channel set to the given value.
    pub const fn with_l(self, l: f32) -> Self {
        Self { l, ..self }
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
            l: self.l * n_factor + other.l * factor,
            a: self.a * n_factor + other.a * factor,
            b: self.b * n_factor + other.b * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
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
}

impl Luminance for Oklaba {
    #[inline]
    fn with_luminance(&self, l: f32) -> Self {
        Self { l, ..*self }
    }

    fn luminance(&self) -> f32 {
        self.l
    }

    fn darker(&self, amount: f32) -> Self {
        Self::new((self.l - amount).max(0.), self.a, self.b, self.alpha)
    }

    fn lighter(&self, amount: f32) -> Self {
        Self::new((self.l + amount).min(1.), self.a, self.b, self.alpha)
    }
}

impl EuclideanDistance for Oklaba {
    #[inline]
    fn distance_squared(&self, other: &Self) -> f32 {
        (self.l - other.l).powi(2) + (self.a - other.a).powi(2) + (self.b - other.b).powi(2)
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

impl From<Srgba> for Oklaba {
    fn from(value: Srgba) -> Self {
        Oklaba::from(LinearRgba::from(value))
    }
}

impl From<Lcha> for Oklaba {
    fn from(value: Lcha) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hsla> for Oklaba {
    fn from(value: Hsla) -> Self {
        LinearRgba::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_colors::TEST_COLORS, testing::assert_approx_eq, Srgba};

    #[test]
    fn test_to_from_srgba() {
        let oklaba = Oklaba::new(0.5, 0.5, 0.5, 1.0);
        let srgba: Srgba = oklaba.into();
        let oklaba2: Oklaba = srgba.into();
        assert_approx_eq!(oklaba.l, oklaba2.l, 0.001);
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
        assert_approx_eq!(oklaba.l, oklaba2.l, 0.001);
        assert_approx_eq!(oklaba.a, oklaba2.a, 0.001);
        assert_approx_eq!(oklaba.b, oklaba2.b, 0.001);
        assert_approx_eq!(oklaba.alpha, oklaba2.alpha, 0.001);
    }
}
