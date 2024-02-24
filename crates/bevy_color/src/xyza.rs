use crate::{Alpha, Hsla, Lcha, LinearRgba, Luminance, Mix, Oklaba, Srgba, StandardColor};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::color::Color;
use serde::{Deserialize, Serialize};

/// [CIE 1931](https://en.wikipedia.org/wiki/CIE_1931_color_space) color space, also known as XYZ, with an alpha channel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Xyza {
    /// The x-axis. [0.0, 1.0]
    pub x: f32,
    /// The y-axis, intended to represent luminance. [0.0, 1.0]
    pub y: f32,
    /// The z-axis. [0.0, 1.0]
    pub z: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Xyza {}

impl Xyza {
    /// Construct a new [`Xyza`] color from components.
    ///
    /// # Arguments
    ///
    /// * `x` - x-axis. [0.0, 1.0]
    /// * `y` - y-axis. [0.0, 1.0]
    /// * `z` - z-axis. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(x: f32, y: f32, z: f32, alpha: f32) -> Self {
        Self { x, y, z, alpha }
    }

    /// Construct a new [`Xyza`] color from (x, y, z) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `x` - x-axis. [0.0, 1.0]
    /// * `y` - y-axis. [0.0, 1.0]
    /// * `z` - z-axis. [0.0, 1.0]
    pub const fn rgb(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            alpha: 1.0,
        }
    }
}

impl Default for Xyza {
    fn default() -> Self {
        Self::new(0., 0., 0., 1.)
    }
}

impl Alpha for Xyza {
    #[inline]
    fn with_alpha(&self, alpha: f32) -> Self {
        Self { alpha, ..*self }
    }

    #[inline]
    fn alpha(&self) -> f32 {
        self.alpha
    }
}

impl Luminance for Xyza {
    #[inline]
    fn with_luminance(&self, lightness: f32) -> Self {
        Self {
            y: lightness,
            ..*self
        }
    }

    fn luminance(&self) -> f32 {
        self.y
    }

    fn darker(&self, amount: f32) -> Self {
        Self {
            y: (self.y - amount).clamp(0., 1.),
            ..*self
        }
    }

    fn lighter(&self, amount: f32) -> Self {
        Self {
            y: (self.y + amount).min(1.),
            ..*self
        }
    }
}

impl Mix for Xyza {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            x: self.x * n_factor + other.x * factor,
            y: self.y * n_factor + other.y * factor,
            z: self.z * n_factor + other.z * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl From<LinearRgba> for Xyza {
    fn from(
        LinearRgba {
            red,
            green,
            blue,
            alpha,
        }: LinearRgba,
    ) -> Self {
        // Linear sRGB to XYZ
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_RGB.html
        // http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html (sRGB, RGB to XYZ [M])
        let r = red;
        let g = green;
        let b = blue;

        let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
        let y = r * 0.2126729 + g * 0.7151522 + b * 0.072175;
        let z = r * 0.0193339 + g * 0.119192 + b * 0.9503041;

        Xyza::new(x, y, z, alpha)
    }
}

impl From<Xyza> for LinearRgba {
    fn from(Xyza { x, y, z, alpha }: Xyza) -> Self {
        // XYZ to Linear sRGB
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_RGB.html
        // http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html (sRGB, XYZ to RGB [M]-1)
        let r = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
        let g = x * -0.969266 + y * 1.8760108 + z * 0.041556;
        let b = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;

        LinearRgba::new(r, g, b, alpha)
    }
}

impl From<Srgba> for Xyza {
    fn from(value: Srgba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Srgba {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Hsla> for Xyza {
    fn from(value: Hsla) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Hsla {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Lcha> for Xyza {
    fn from(value: Lcha) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Lcha {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for Xyza {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Oklaba {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Color> for Xyza {
    fn from(value: Color) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for Color {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
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
        let xyza = Xyza::new(0.5, 0.5, 0.5, 1.0);
        let srgba: Srgba = xyza.into();
        let xyza2: Xyza = srgba.into();
        assert_approx_eq!(xyza.x, xyza2.x, 0.001);
        assert_approx_eq!(xyza.y, xyza2.y, 0.001);
        assert_approx_eq!(xyza.z, xyza2.z, 0.001);
        assert_approx_eq!(xyza.alpha, xyza2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_srgba_2() {
        for color in TEST_COLORS.iter() {
            let rgb2: Srgba = (color.xyz).into();
            let xyz2: Xyza = (color.rgb).into();
            assert!(
                color.rgb.distance(&rgb2) < 0.00001,
                "{}: {:?} != {:?}",
                color.name,
                color.rgb,
                rgb2
            );
            assert_approx_eq!(color.xyz.x, xyz2.x, 0.001);
            assert_approx_eq!(color.xyz.y, xyz2.y, 0.001);
            assert_approx_eq!(color.xyz.z, xyz2.z, 0.001);
            assert_approx_eq!(color.xyz.alpha, xyz2.alpha, 0.001);
        }
    }
}
