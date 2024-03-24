//! Implementation of the Hue-Whiteness-Blackness (HWB) color model as described
//! in [_HWB - A More Intuitive Hue-Based Color Model_] by _Smith et al_.
//!
//! [_HWB - A More Intuitive Hue-Based Color Model_]: https://web.archive.org/web/20240226005220/http://alvyray.com/Papers/CG/HWB_JGTv208.pdf
use crate::{Alpha, ClampColor, Hue, Lcha, LinearRgba, Mix, Srgba, StandardColor, Xyza};
use bevy_reflect::prelude::*;

/// Color in Hue-Whiteness-Blackness (HWB) color space with alpha.
/// Further information on this color model can be found on [Wikipedia](https://en.wikipedia.org/wiki/HWB_color_model).
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
pub struct Hwba {
    /// The hue channel. [0.0, 360.0]
    pub hue: f32,
    /// The whiteness channel. [0.0, 1.0]
    pub whiteness: f32,
    /// The blackness channel. [0.0, 1.0]
    pub blackness: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Hwba {}

impl Hwba {
    /// Construct a new [`Hwba`] color from components.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `whiteness` - Whiteness channel. [0.0, 1.0]
    /// * `blackness` - Blackness channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(hue: f32, whiteness: f32, blackness: f32, alpha: f32) -> Self {
        Self {
            hue,
            whiteness,
            blackness,
            alpha,
        }
    }

    /// Construct a new [`Hwba`] color from (h, s, l) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `whiteness` - Whiteness channel. [0.0, 1.0]
    /// * `blackness` - Blackness channel. [0.0, 1.0]
    pub const fn hwb(hue: f32, whiteness: f32, blackness: f32) -> Self {
        Self::new(hue, whiteness, blackness, 1.0)
    }

    /// Return a copy of this color with the whiteness channel set to the given value.
    pub const fn with_whiteness(self, whiteness: f32) -> Self {
        Self { whiteness, ..self }
    }

    /// Return a copy of this color with the blackness channel set to the given value.
    pub const fn with_blackness(self, blackness: f32) -> Self {
        Self { blackness, ..self }
    }
}

impl Default for Hwba {
    fn default() -> Self {
        Self::new(0., 0., 1., 1.)
    }
}

impl Mix for Hwba {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            hue: crate::color_ops::lerp_hue(self.hue, other.hue, factor),
            whiteness: self.whiteness * n_factor + other.whiteness * factor,
            blackness: self.blackness * n_factor + other.blackness * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Alpha for Hwba {
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

impl Hue for Hwba {
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

impl ClampColor for Hwba {
    fn clamped(&self) -> Self {
        Self {
            hue: self.hue.rem_euclid(360.),
            whiteness: self.whiteness.clamp(0., 1.),
            blackness: self.blackness.clamp(0., 1.),
            alpha: self.alpha.clamp(0., 1.),
        }
    }

    fn is_within_bounds(&self) -> bool {
        (0. ..=360.).contains(&self.hue)
            && (0. ..=1.).contains(&self.whiteness)
            && (0. ..=1.).contains(&self.blackness)
            && (0. ..=1.).contains(&self.alpha)
    }
}

impl From<Srgba> for Hwba {
    fn from(
        Srgba {
            red,
            green,
            blue,
            alpha,
        }: Srgba,
    ) -> Self {
        // Based on "HWB - A More Intuitive Hue-Based Color Model" Appendix B
        let x_max = 0f32.max(red).max(green).max(blue);
        let x_min = 1f32.min(red).min(green).min(blue);

        let chroma = x_max - x_min;

        let hue = if chroma == 0.0 {
            0.0
        } else if red == x_max {
            60.0 * (green - blue) / chroma
        } else if green == x_max {
            60.0 * (2.0 + (blue - red) / chroma)
        } else {
            60.0 * (4.0 + (red - green) / chroma)
        };
        let hue = if hue < 0.0 { 360.0 + hue } else { hue };

        let whiteness = x_min;
        let blackness = 1.0 - x_max;

        Hwba {
            hue,
            whiteness,
            blackness,
            alpha,
        }
    }
}

impl From<Hwba> for Srgba {
    fn from(
        Hwba {
            hue,
            whiteness,
            blackness,
            alpha,
        }: Hwba,
    ) -> Self {
        // Based on "HWB - A More Intuitive Hue-Based Color Model" Appendix B
        let w = whiteness;
        let v = 1. - blackness;

        let h = (hue % 360.) / 60.;
        let i = h.floor();
        let f = h - i;

        let i = i as u8;

        let f = if i % 2 == 0 { f } else { 1. - f };

        let n = w + f * (v - w);

        let (red, green, blue) = match i {
            0 => (v, n, w),
            1 => (n, v, w),
            2 => (w, v, n),
            3 => (w, n, v),
            4 => (n, w, v),
            5 => (v, w, n),
            _ => unreachable!("i is bounded in [0, 6)"),
        };

        Srgba::new(red, green, blue, alpha)
    }
}

// Derived Conversions

impl From<LinearRgba> for Hwba {
    fn from(value: LinearRgba) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Hwba> for LinearRgba {
    fn from(value: Hwba) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Lcha> for Hwba {
    fn from(value: Lcha) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Hwba> for Lcha {
    fn from(value: Hwba) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Xyza> for Hwba {
    fn from(value: Xyza) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Hwba> for Xyza {
    fn from(value: Hwba) -> Self {
        Srgba::from(value).into()
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
        let hwba = Hwba::new(0.0, 0.5, 0.5, 1.0);
        let srgba: Srgba = hwba.into();
        let hwba2: Hwba = srgba.into();
        assert_approx_eq!(hwba.hue, hwba2.hue, 0.001);
        assert_approx_eq!(hwba.whiteness, hwba2.whiteness, 0.001);
        assert_approx_eq!(hwba.blackness, hwba2.blackness, 0.001);
        assert_approx_eq!(hwba.alpha, hwba2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.hwb).into();
            let hwb2: Hwba = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.00001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            assert_approx_eq!(color.hwb.hue, hwb2.hue, 0.001);
            assert_approx_eq!(color.hwb.whiteness, hwb2.whiteness, 0.001);
            assert_approx_eq!(color.hwb.blackness, hwb2.blackness, 0.001);
            assert_approx_eq!(color.hwb.alpha, hwb2.alpha, 0.001);
        }
    }

    #[test]
    fn test_clamp() {
        let color_1 = Hwba::hwb(361., 2., -1.);
        let color_2 = Hwba::hwb(250.2762, 1., 0.67);
        let mut color_3 = Hwba::hwb(-50., 1., 1.);

        assert!(!color_1.is_within_bounds());
        assert_eq!(color_1.clamped(), Hwba::hwb(1., 1., 0.));

        assert!(color_2.is_within_bounds());
        assert_eq!(color_2, color_2.clamped());

        color_3.clamp();
        assert!(color_3.is_within_bounds());
        assert_eq!(color_3, Hwba::hwb(310., 1., 1.));
    }
}
