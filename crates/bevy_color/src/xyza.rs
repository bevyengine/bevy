use crate::{
    impl_componentwise_vector_space, Alpha, ColorToComponents, Gray, LinearRgba, Luminance, Mix,
    StandardColor,
};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::prelude::*;

/// [CIE 1931](https://en.wikipedia.org/wiki/CIE_1931_color_space) color space, also known as XYZ, with an alpha channel.
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

impl_componentwise_vector_space!(Xyza, [x, y, z, alpha]);

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
    pub const fn xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            alpha: 1.0,
        }
    }

    /// Return a copy of this color with the 'x' channel set to the given value.
    pub const fn with_x(self, x: f32) -> Self {
        Self { x, ..self }
    }

    /// Return a copy of this color with the 'y' channel set to the given value.
    pub const fn with_y(self, y: f32) -> Self {
        Self { y, ..self }
    }

    /// Return a copy of this color with the 'z' channel set to the given value.
    pub const fn with_z(self, z: f32) -> Self {
        Self { z, ..self }
    }

    /// [D65 White Point](https://en.wikipedia.org/wiki/Illuminant_D65#Definition)
    pub const D65_WHITE: Self = Self::xyz(0.95047, 1.0, 1.08883);
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

    #[inline]
    fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
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

impl Gray for Xyza {
    const BLACK: Self = Self::new(0., 0., 0., 1.);
    const WHITE: Self = Self::new(0.95047, 1.0, 1.08883, 1.0);
}

impl ColorToComponents for Xyza {
    fn to_f32_array(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            x: color[0],
            y: color[1],
            z: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            x: color[0],
            y: color[1],
            z: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            x: color[0],
            y: color[1],
            z: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            x: color[0],
            y: color[1],
            z: color[2],
            alpha: 1.0,
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
