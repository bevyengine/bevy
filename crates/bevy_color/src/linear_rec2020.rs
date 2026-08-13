use crate::{
    color_difference::EuclideanDistance, impl_componentwise_vector_space, Alpha, ColorToComponents,
    Gray, Hsla, Hsva, Hwba, Laba, Lcha, LinearRgba, Luminance, Mix, Okhsla, Okhsva, Okhwba, Oklaba,
    Oklcha, Srgba, StandardColor, Xyza,
};
use bevy_math::{Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// Linear RGB color with [ITU-R BT.2020](https://registry.color.org/rgb-registry/bt2020)
/// (Rec. 2020) wide-gamut primaries, a D65 white point, and an alpha channel.
///
/// All Rec. 709 colors and almost all Display P3 colors fit inside this gamut with
/// non-negative components. `(1.0, 1.0, 1.0)` is the D65 reference white, the same
/// white as [`LinearRgba::WHITE`]. Values above `1.0` are HDR intensities, and
/// negative values are outside the gamut.
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
pub struct LinearRec2020 {
    /// The red channel. [0.0, 1.0] for SDR colors.
    pub red: f32,
    /// The green channel. [0.0, 1.0] for SDR colors.
    pub green: f32,
    /// The blue channel. [0.0, 1.0] for SDR colors.
    pub blue: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for LinearRec2020 {}

impl_componentwise_vector_space!(LinearRec2020, [red, green, blue, alpha]);

impl LinearRec2020 {
    /// Linear Rec. 2020 to CIE 1931 XYZ matrix, derived from the ITU-R BT.2020-2
    /// chromaticities with the same
    /// [Lindbloom method](http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html)
    /// and reference white as the crate's sRGB to XYZ matrices.
    ///
    /// Row-major rows: X, Y, Z. The first entry of the Z row is zero because the
    /// BT.2020 red primary lies on the `x + y = 1` line.
    const RGB_TO_XYZ: [[f32; 3]; 3] = [
        [0.637_010_2, 0.144_615_02, 0.168_844_77],
        [0.262_721_72, 0.677_989_3, 0.059_289_01],
        [0.0, 0.028_072_33, 1.060_757_6],
    ];

    /// CIE 1931 XYZ to linear Rec. 2020 matrix; the inverse of
    /// [`Self::RGB_TO_XYZ`]. Row-major rows: red, green, blue.
    const XYZ_TO_RGB: [[f32; 3]; 3] = [
        [1.716_510_7, -0.355_641_66, -0.253_345_55],
        [-0.666_693, 1.616_502_2, 0.015_768_75],
        [0.017_643_638, -0.042_779_78, 0.942_305_1],
    ];

    /// A fully black color with full alpha.
    pub const BLACK: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    /// A fully white color with full alpha.
    pub const WHITE: Self = Self {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
        alpha: 1.0,
    };

    /// A fully transparent color.
    pub const NONE: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    };

    /// Construct a new [`LinearRec2020`] color from components.
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Construct a new [`LinearRec2020`] color from (r, g, b) components, with the
    /// default alpha (1.0).
    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    /// Return a copy of this color with the red channel set to the given value.
    pub const fn with_red(self, red: f32) -> Self {
        Self { red, ..self }
    }

    /// Return a copy of this color with the green channel set to the given value.
    pub const fn with_green(self, green: f32) -> Self {
        Self { green, ..self }
    }

    /// Return a copy of this color with the blue channel set to the given value.
    pub const fn with_blue(self, blue: f32) -> Self {
        Self { blue, ..self }
    }

    /// Make the color lighter or darker by some amount. SDR colors clamp to black and
    /// white. HDR colors have no upper clamp: they scale and keep their chromaticity.
    fn adjust_lightness(&mut self, amount: f32) {
        let luminance = self.luminance();
        // A saturated color can have channels above 1.0 while its luminance stays below 1.0.
        let is_sdr = luminance <= 1.0 && self.red <= 1.0 && self.green <= 1.0 && self.blue <= 1.0;
        let target_luminance = if is_sdr {
            (luminance + amount).clamp(0.0, 1.0)
        } else {
            (luminance + amount).max(0.0)
        };
        if target_luminance < luminance {
            let adjustment = (luminance - target_luminance) / luminance;
            self.mix_assign(Self::new(0.0, 0.0, 0.0, self.alpha), adjustment);
        } else if target_luminance > luminance {
            if is_sdr {
                let adjustment = (target_luminance - luminance) / (1. - luminance);
                self.mix_assign(Self::new(1.0, 1.0, 1.0, self.alpha), adjustment);
            } else {
                let scale = target_luminance / luminance;
                self.red *= scale;
                self.green *= scale;
                self.blue *= scale;
            }
        }
    }
}

impl Default for LinearRec2020 {
    /// Construct a new [`LinearRec2020`] color with the default values (white with full alpha).
    fn default() -> Self {
        Self::WHITE
    }
}

impl Luminance for LinearRec2020 {
    /// Relative luminance using the Rec. 2020 weights: the Y row of the RGB to XYZ
    /// matrix, approximately `0.2627 R + 0.6780 G + 0.0593 B` per ITU-R BT.2020.
    #[inline]
    fn luminance(&self) -> f32 {
        let [lr, lg, lb] = Self::RGB_TO_XYZ[1];
        self.red * lr + self.green * lg + self.blue * lb
    }

    /// Scales the color to the target luminance, preserving its chromaticity.
    ///
    /// If the components are all within `[0.0, 1.0]` and the target is at most `1.0`,
    /// the result is clamped to that range.
    #[inline]
    fn with_luminance(&self, luminance: f32) -> Self {
        let current_luminance = self.luminance();
        let adjustment = luminance / current_luminance;
        let (red, green, blue) = (
            self.red * adjustment,
            self.green * adjustment,
            self.blue * adjustment,
        );
        let sdr = |c: f32| (0.0..=1.0).contains(&c);
        // A negative target takes this branch, so an SDR color clamps to black.
        // A NaN target fails the check and passes through unclamped.
        if sdr(self.red) && sdr(self.green) && sdr(self.blue) && luminance <= 1.0 {
            Self {
                red: red.clamp(0., 1.),
                green: green.clamp(0., 1.),
                blue: blue.clamp(0., 1.),
                alpha: self.alpha,
            }
        } else {
            Self {
                red,
                green,
                blue,
                alpha: self.alpha,
            }
        }
    }

    #[inline]
    fn darker(&self, amount: f32) -> Self {
        let mut result = *self;
        result.adjust_lightness(-amount);
        result
    }

    #[inline]
    fn lighter(&self, amount: f32) -> Self {
        let mut result = *self;
        result.adjust_lightness(amount);
        result
    }
}

impl Mix for LinearRec2020 {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            red: self.red * n_factor + other.red * factor,
            green: self.green * n_factor + other.green * factor,
            blue: self.blue * n_factor + other.blue * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Gray for LinearRec2020 {
    const BLACK: Self = Self::BLACK;
    const WHITE: Self = Self::WHITE;
}

impl Alpha for LinearRec2020 {
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

impl EuclideanDistance for LinearRec2020 {
    #[inline]
    fn distance_squared(&self, other: &Self) -> f32 {
        let dr = self.red - other.red;
        let dg = self.green - other.green;
        let db = self.blue - other.blue;
        dr * dr + dg * dg + db * db
    }
}

impl ColorToComponents for LinearRec2020 {
    fn to_f32_array(self) -> [f32; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }

    fn to_f32_array_no_alpha(self) -> [f32; 3] {
        [self.red, self.green, self.blue]
    }

    fn to_vec4(self) -> Vec4 {
        Vec4::new(self.red, self.green, self.blue, self.alpha)
    }

    fn to_vec3(self) -> Vec3 {
        Vec3::new(self.red, self.green, self.blue)
    }

    fn from_f32_array(color: [f32; 4]) -> Self {
        Self {
            red: color[0],
            green: color[1],
            blue: color[2],
            alpha: color[3],
        }
    }

    fn from_f32_array_no_alpha(color: [f32; 3]) -> Self {
        Self {
            red: color[0],
            green: color[1],
            blue: color[2],
            alpha: 1.0,
        }
    }

    fn from_vec4(color: Vec4) -> Self {
        Self {
            red: color[0],
            green: color[1],
            blue: color[2],
            alpha: color[3],
        }
    }

    fn from_vec3(color: Vec3) -> Self {
        Self {
            red: color[0],
            green: color[1],
            blue: color[2],
            alpha: 1.0,
        }
    }
}

impl From<LinearRec2020> for Xyza {
    fn from(
        LinearRec2020 {
            red,
            green,
            blue,
            alpha,
        }: LinearRec2020,
    ) -> Self {
        let m = LinearRec2020::RGB_TO_XYZ;
        let x = red * m[0][0] + green * m[0][1] + blue * m[0][2];
        let y = red * m[1][0] + green * m[1][1] + blue * m[1][2];
        let z = red * m[2][0] + green * m[2][1] + blue * m[2][2];

        Xyza::new(x, y, z, alpha)
    }
}

impl From<Xyza> for LinearRec2020 {
    fn from(Xyza { x, y, z, alpha }: Xyza) -> Self {
        let m = LinearRec2020::XYZ_TO_RGB;
        let red = x * m[0][0] + y * m[0][1] + z * m[0][2];
        let green = x * m[1][0] + y * m[1][1] + z * m[1][2];
        let blue = x * m[2][0] + y * m[2][1] + z * m[2][2];

        LinearRec2020::new(red, green, blue, alpha)
    }
}

// Derived Conversions

impl From<Srgba> for LinearRec2020 {
    fn from(value: Srgba) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Srgba {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRgba> for LinearRec2020 {
    fn from(value: LinearRgba) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for LinearRgba {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Hsla> for LinearRec2020 {
    fn from(value: Hsla) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Hsla {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Hsva> for LinearRec2020 {
    fn from(value: Hsva) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Hsva {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Hwba> for LinearRec2020 {
    fn from(value: Hwba) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Hwba {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Laba> for LinearRec2020 {
    fn from(value: Laba) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Laba {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Lcha> for LinearRec2020 {
    fn from(value: Lcha) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Lcha {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Oklaba> for LinearRec2020 {
    fn from(value: Oklaba) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Oklaba {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Oklcha> for LinearRec2020 {
    fn from(value: Oklcha) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Oklcha {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Okhsla> for LinearRec2020 {
    fn from(value: Okhsla) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Okhsla {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Okhsva> for LinearRec2020 {
    fn from(value: Okhsva) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Okhsva {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

impl From<Okhwba> for LinearRec2020 {
    fn from(value: Okhwba) -> Self {
        Xyza::from(value).into()
    }
}

impl From<LinearRec2020> for Okhwba {
    fn from(value: LinearRec2020) -> Self {
        Xyza::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_approx_eq;

    #[test]
    fn xyza_round_trip() {
        let colors = [
            LinearRec2020::new(0.0, 0.0, 0.0, 1.0),
            LinearRec2020::new(1.0, 1.0, 1.0, 1.0),
            LinearRec2020::new(0.5, 0.25, 0.75, 0.5),
            LinearRec2020::new(1.0, 0.0, 0.0, 1.0),
            LinearRec2020::new(0.0, 1.0, 0.0, 1.0),
            LinearRec2020::new(0.0, 0.0, 1.0, 1.0),
            LinearRec2020::new(4.5, -0.2, 1.5, 1.0),
        ];
        for color in colors {
            let xyza: Xyza = color.into();
            let back: LinearRec2020 = xyza.into();
            assert_approx_eq!(color.red, back.red, 1e-5);
            assert_approx_eq!(color.green, back.green, 1e-5);
            assert_approx_eq!(color.blue, back.blue, 1e-5);
            assert_approx_eq!(color.alpha, back.alpha, 1e-5);
        }
    }

    #[test]
    fn linear_rgba_round_trip() {
        let colors = [
            LinearRec2020::new(0.5, 0.25, 0.75, 0.5),
            LinearRec2020::new(1.0, 0.0, 0.0, 1.0),
            LinearRec2020::new(2.0, 1.5, 0.25, 1.0),
        ];
        for color in colors {
            let rgba: LinearRgba = color.into();
            let back: LinearRec2020 = rgba.into();
            assert_approx_eq!(color.red, back.red, 1e-5);
            assert_approx_eq!(color.green, back.green, 1e-5);
            assert_approx_eq!(color.blue, back.blue, 1e-5);
            assert_approx_eq!(color.alpha, back.alpha, 1e-5);
        }
    }

    #[test]
    fn primaries_map_to_bt2020_chromaticities() {
        // Each pure primary must land on its ITU-R BT.2020-2 chromaticity.
        let cases = [
            (LinearRec2020::rgb(1.0, 0.0, 0.0), (0.708, 0.292)),
            (LinearRec2020::rgb(0.0, 1.0, 0.0), (0.170, 0.797)),
            (LinearRec2020::rgb(0.0, 0.0, 1.0), (0.131, 0.046)),
        ];
        for (color, (expected_x, expected_y)) in cases {
            let xyza: Xyza = color.into();
            let sum = xyza.x + xyza.y + xyza.z;
            assert_approx_eq!(xyza.x / sum, expected_x, 1e-4);
            assert_approx_eq!(xyza.y / sum, expected_y, 1e-4);
        }
    }

    #[test]
    fn white_maps_to_d65() {
        let white: Xyza = LinearRec2020::WHITE.into();
        assert_approx_eq!(white.x, Xyza::D65_WHITE.x, 1e-5);
        assert_approx_eq!(white.y, Xyza::D65_WHITE.y, 1e-5);
        assert_approx_eq!(white.z, Xyza::D65_WHITE.z, 1e-5);

        // Both spaces share D65, so white is also linear sRGB white.
        let srgb_white: LinearRgba = LinearRec2020::WHITE.into();
        assert_approx_eq!(srgb_white.red, 1.0, 1e-5);
        assert_approx_eq!(srgb_white.green, 1.0, 1e-5);
        assert_approx_eq!(srgb_white.blue, 1.0, 1e-5);
    }

    #[test]
    fn rec709_red_matches_bt2087() {
        // Pure Rec.709 red in Rec.2020 must match the first column of the published
        // ITU-R BT.2087 matrix.
        let red: LinearRec2020 = LinearRgba::RED.into();
        assert_approx_eq!(red.red, 0.6274, 1e-3);
        assert_approx_eq!(red.green, 0.0691, 1e-3);
        assert_approx_eq!(red.blue, 0.0164, 1e-3);
    }

    #[test]
    fn luminance_weights() {
        assert_approx_eq!(LinearRec2020::WHITE.luminance(), 1.0, 1e-6);
        // ITU-R BT.2020 luma weights.
        assert_approx_eq!(LinearRec2020::rgb(1.0, 0.0, 0.0).luminance(), 0.2627, 1e-4);
        assert_approx_eq!(LinearRec2020::rgb(0.0, 1.0, 0.0).luminance(), 0.6780, 1e-4);
        assert_approx_eq!(LinearRec2020::rgb(0.0, 0.0, 1.0).luminance(), 0.0593, 1e-4);
    }

    #[test]
    fn hdr_lighter_darker() {
        let sdr = LinearRec2020::rgb(0.9, 0.9, 0.9);
        let lighter = sdr.lighter(0.5);
        assert_approx_eq!(lighter.luminance(), 1.0, 1e-5);

        let hdr = LinearRec2020::rgb(2.0, 2.0, 2.0);
        let lighter = hdr.lighter(0.5);
        assert_approx_eq!(lighter.luminance(), 2.5, 1e-4);
        let darker = hdr.darker(0.5);
        assert_approx_eq!(darker.luminance(), 1.5, 1e-4);

        // A saturated HDR color (channel above 1.0, luminance below 1.0) counts as HDR:
        // `lighter` scales it instead of crushing it to SDR white.
        let saturated_hdr = LinearRec2020::rgb(3.0, 0.0, 0.0);
        let luminance = saturated_hdr.luminance();
        assert!(luminance < 1.0);
        let lighter = saturated_hdr.lighter(0.3);
        assert_approx_eq!(lighter.luminance(), luminance + 0.3, 1e-4);
        assert!(lighter.red > 3.0);
        assert_approx_eq!(lighter.green, 0.0, 1e-6);
        assert_approx_eq!(lighter.blue, 0.0, 1e-6);
        let via_with_luminance = saturated_hdr.with_luminance(luminance + 0.3);
        assert_approx_eq!(lighter.red, via_with_luminance.red, 1e-4);
    }
}
