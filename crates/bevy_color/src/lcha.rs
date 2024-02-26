use crate::{Alpha, Hsla, LinearRgba, Luminance, Mix, Oklaba, Srgba, StandardColor, Xyza};
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

    /// CIE Epsilon Constant
    ///
    /// See [Continuity (16) (17)](http://brucelindbloom.com/index.html?LContinuity.html)
    pub const CIE_EPSILON: f32 = 216.0 / 24389.0;

    /// CIE Kappa Constant
    ///
    /// See [Continuity (16) (17)](http://brucelindbloom.com/index.html?LContinuity.html)
    pub const CIE_KAPPA: f32 = 24389.0 / 27.0;
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

impl From<Lcha> for Xyza {
    fn from(
        Lcha {
            lightness,
            chroma,
            hue,
            alpha,
        }: Lcha,
    ) -> Self {
        let lightness = lightness * 100.0;
        let chroma = chroma * 100.0;

        // convert LCH to Lab
        // http://www.brucelindbloom.com/index.html?Eqn_LCH_to_Lab.html
        let l = lightness;
        let a = chroma * hue.to_radians().cos();
        let b = chroma * hue.to_radians().sin();

        // convert Lab to XYZ
        // http://www.brucelindbloom.com/index.html?Eqn_Lab_to_XYZ.html
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b / 200.0;
        let xr = {
            let fx3 = fx.powf(3.0);

            if fx3 > Lcha::CIE_EPSILON {
                fx3
            } else {
                (116.0 * fx - 16.0) / Lcha::CIE_KAPPA
            }
        };
        let yr = if l > Lcha::CIE_EPSILON * Lcha::CIE_KAPPA {
            ((l + 16.0) / 116.0).powf(3.0)
        } else {
            l / Lcha::CIE_KAPPA
        };
        let zr = {
            let fz3 = fz.powf(3.0);

            if fz3 > Lcha::CIE_EPSILON {
                fz3
            } else {
                (116.0 * fz - 16.0) / Lcha::CIE_KAPPA
            }
        };
        let x = xr * Xyza::D65_WHITE.x;
        let y = yr * Xyza::D65_WHITE.y;
        let z = zr * Xyza::D65_WHITE.z;

        Xyza::new(x, y, z, alpha)
    }
}

impl From<Xyza> for Lcha {
    fn from(Xyza { x, y, z, alpha }: Xyza) -> Self {
        // XYZ to Lab
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_Lab.html
        let xr = x / Xyza::D65_WHITE.x;
        let yr = y / Xyza::D65_WHITE.y;
        let zr = z / Xyza::D65_WHITE.z;
        let fx = if xr > Lcha::CIE_EPSILON {
            xr.cbrt()
        } else {
            (Lcha::CIE_KAPPA * xr + 16.0) / 116.0
        };
        let fy = if yr > Lcha::CIE_EPSILON {
            yr.cbrt()
        } else {
            (Lcha::CIE_KAPPA * yr + 16.0) / 116.0
        };
        let fz = if yr > Lcha::CIE_EPSILON {
            zr.cbrt()
        } else {
            (Lcha::CIE_KAPPA * zr + 16.0) / 116.0
        };
        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);

        // Lab to LCH
        // http://www.brucelindbloom.com/index.html?Eqn_Lab_to_LCH.html
        let c = (a.powf(2.0) + b.powf(2.0)).sqrt();
        let h = {
            let h = b.to_radians().atan2(a.to_radians()).to_degrees();

            if h < 0.0 {
                h + 360.0
            } else {
                h
            }
        };

        let lightness = (l / 100.0).clamp(0.0, 1.5);
        let chroma = (c / 100.0).clamp(0.0, 1.5);
        let hue = h;

        Lcha::new(lightness, chroma, hue, alpha)
    }
}

impl From<Srgba> for Lcha {
    fn from(value: Srgba) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Lcha> for Srgba {
    fn from(value: Lcha) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRgba> for Lcha {
    fn from(value: LinearRgba) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Lcha> for LinearRgba {
    fn from(value: Lcha) -> Self {
        LinearRgba::from(Srgba::from(value))
    }
}

impl From<Oklaba> for Lcha {
    fn from(value: Oklaba) -> Self {
        Srgba::from(value).into()
    }
}

impl From<Hsla> for Lcha {
    fn from(value: Hsla) -> Self {
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
