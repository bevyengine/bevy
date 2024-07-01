use crate::color_difference::EuclideanDistance;
use crate::{
    impl_componentwise_vector_space, Alpha, ColorToComponents, ColorToPacked, Gray, LinearRgba,
    Luminance, Mix, StandardColor, Xyza,
};
use bevy_math::{Vec3, Vec4};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;
use thiserror::Error;

/// Non-linear standard RGB with alpha.
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
pub struct Srgba {
    /// The red channel. [0.0, 1.0]
    pub red: f32,
    /// The green channel. [0.0, 1.0]
    pub green: f32,
    /// The blue channel. [0.0, 1.0]
    pub blue: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for Srgba {}

impl_componentwise_vector_space!(Srgba, [red, green, blue, alpha]);

impl Srgba {
    // The standard VGA colors, with alpha set to 1.0.
    // https://en.wikipedia.org/wiki/Web_colors#Basic_colors

    /// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: Srgba = Srgba::new(0.0, 0.0, 0.0, 1.0);
    /// <div style="background-color:rgba(0%, 0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    #[doc(alias = "transparent")]
    pub const NONE: Srgba = Srgba::new(0.0, 0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: Srgba = Srgba::new(1.0, 1.0, 1.0, 1.0);

    /// A fully red color with full alpha.
    pub const RED: Self = Self {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    /// A fully green color with full alpha.
    pub const GREEN: Self = Self {
        red: 0.0,
        green: 1.0,
        blue: 0.0,
        alpha: 1.0,
    };

    /// A fully blue color with full alpha.
    pub const BLUE: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 1.0,
        alpha: 1.0,
    };

    /// Construct a new [`Srgba`] color from components.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Construct a new [`Srgba`] color from (r, g, b) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
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

    /// New `Srgba` from a CSS-style hexadecimal string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_color::Srgba;
    /// let color = Srgba::hex("FF00FF").unwrap(); // fuchsia
    /// let color = Srgba::hex("FF00FF7F").unwrap(); // partially transparent fuchsia
    ///
    /// // A standard hex color notation is also available
    /// assert_eq!(Srgba::hex("#FFFFFF").unwrap(), Srgba::new(1.0, 1.0, 1.0, 1.0));
    /// ```
    pub fn hex<T: AsRef<str>>(hex: T) -> Result<Self, HexColorError> {
        let hex = hex.as_ref();
        let hex = hex.strip_prefix('#').unwrap_or(hex);

        match hex.len() {
            // RGB
            3 => {
                let [l, b] = u16::from_str_radix(hex, 16)?.to_be_bytes();
                let (r, g, b) = (l & 0x0F, (b & 0xF0) >> 4, b & 0x0F);
                Ok(Self::rgb_u8(r << 4 | r, g << 4 | g, b << 4 | b))
            }
            // RGBA
            4 => {
                let [l, b] = u16::from_str_radix(hex, 16)?.to_be_bytes();
                let (r, g, b, a) = ((l & 0xF0) >> 4, l & 0xF, (b & 0xF0) >> 4, b & 0x0F);
                Ok(Self::rgba_u8(
                    r << 4 | r,
                    g << 4 | g,
                    b << 4 | b,
                    a << 4 | a,
                ))
            }
            // RRGGBB
            6 => {
                let [_, r, g, b] = u32::from_str_radix(hex, 16)?.to_be_bytes();
                Ok(Self::rgb_u8(r, g, b))
            }
            // RRGGBBAA
            8 => {
                let [r, g, b, a] = u32::from_str_radix(hex, 16)?.to_be_bytes();
                Ok(Self::rgba_u8(r, g, b, a))
            }
            _ => Err(HexColorError::Length),
        }
    }

    /// Convert this color to CSS-style hexadecimal notation.
    pub fn to_hex(&self) -> String {
        let [r, g, b, a] = self.to_u8_array();
        match a {
            255 => format!("#{:02X}{:02X}{:02X}", r, g, b),
            _ => format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a),
        }
    }

    /// New `Srgba` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0, 255]
    /// * `g` - Green channel. [0, 255]
    /// * `b` - Blue channel. [0, 255]
    ///
    /// See also [`Srgba::new`], [`Srgba::rgba_u8`], [`Srgba::hex`].
    ///
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::from_u8_array_no_alpha([r, g, b])
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
    /// New `Srgba` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0, 255]
    /// * `g` - Green channel. [0, 255]
    /// * `b` - Blue channel. [0, 255]
    /// * `a` - Alpha channel. [0, 255]
    ///
    /// See also [`Srgba::new`], [`Srgba::rgb_u8`], [`Srgba::hex`].
    ///
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::from_u8_array([r, g, b, a])
    }

    /// Converts a non-linear sRGB value to a linear one via [gamma correction](https://en.wikipedia.org/wiki/Gamma_correction).
    pub fn gamma_function(value: f32) -> f32 {
        if value <= 0.0 {
            return value;
        }
        if value <= 0.04045 {
            value / 12.92 // linear falloff in dark values
        } else {
            ((value + 0.055) / 1.055).powf(2.4) // gamma curve in other area
        }
    }

    /// Converts a linear sRGB value to a non-linear one via [gamma correction](https://en.wikipedia.org/wiki/Gamma_correction).
    pub fn gamma_function_inverse(value: f32) -> f32 {
        if value <= 0.0 {
            return value;
        }

        if value <= 0.0031308 {
            value * 12.92 // linear falloff in dark values
        } else {
            (1.055 * value.powf(1.0 / 2.4)) - 0.055 // gamma curve in other area
        }
    }
}

impl Default for Srgba {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Luminance for Srgba {
    #[inline]
    fn luminance(&self) -> f32 {
        let linear: LinearRgba = (*self).into();
        linear.luminance()
    }

    #[inline]
    fn with_luminance(&self, luminance: f32) -> Self {
        let linear: LinearRgba = (*self).into();
        linear
            .with_luminance(Srgba::gamma_function(luminance))
            .into()
    }

    #[inline]
    fn darker(&self, amount: f32) -> Self {
        let linear: LinearRgba = (*self).into();
        linear.darker(amount).into()
    }

    #[inline]
    fn lighter(&self, amount: f32) -> Self {
        let linear: LinearRgba = (*self).into();
        linear.lighter(amount).into()
    }
}

impl Mix for Srgba {
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

impl Alpha for Srgba {
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

impl EuclideanDistance for Srgba {
    #[inline]
    fn distance_squared(&self, other: &Self) -> f32 {
        let dr = self.red - other.red;
        let dg = self.green - other.green;
        let db = self.blue - other.blue;
        dr * dr + dg * dg + db * db
    }
}

impl Gray for Srgba {
    const BLACK: Self = Self::BLACK;
    const WHITE: Self = Self::WHITE;
}

impl ColorToComponents for Srgba {
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

impl ColorToPacked for Srgba {
    fn to_u8_array(self) -> [u8; 4] {
        [self.red, self.green, self.blue, self.alpha]
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    fn to_u8_array_no_alpha(self) -> [u8; 3] {
        [self.red, self.green, self.blue].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    fn from_u8_array(color: [u8; 4]) -> Self {
        Self::from_f32_array(color.map(|u| u as f32 / 255.0))
    }

    fn from_u8_array_no_alpha(color: [u8; 3]) -> Self {
        Self::from_f32_array_no_alpha(color.map(|u| u as f32 / 255.0))
    }
}

impl From<LinearRgba> for Srgba {
    #[inline]
    fn from(value: LinearRgba) -> Self {
        Self {
            red: Srgba::gamma_function_inverse(value.red),
            green: Srgba::gamma_function_inverse(value.green),
            blue: Srgba::gamma_function_inverse(value.blue),
            alpha: value.alpha,
        }
    }
}

impl From<Srgba> for LinearRgba {
    #[inline]
    fn from(value: Srgba) -> Self {
        Self {
            red: Srgba::gamma_function(value.red),
            green: Srgba::gamma_function(value.green),
            blue: Srgba::gamma_function(value.blue),
            alpha: value.alpha,
        }
    }
}

// Derived Conversions

impl From<Xyza> for Srgba {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Srgba> for Xyza {
    fn from(value: Srgba) -> Self {
        LinearRgba::from(value).into()
    }
}

/// Error returned if a hex string could not be parsed as a color.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HexColorError {
    /// Parsing error.
    #[error("Invalid hex string")]
    Parse(#[from] std::num::ParseIntError),
    /// Invalid length.
    #[error("Unexpected length of hex string")]
    Length,
    /// Invalid character.
    #[error("Invalid hex char")]
    Char(char),
}

#[cfg(test)]
mod tests {
    use crate::testing::assert_approx_eq;

    use super::*;

    #[test]
    fn test_to_from_linear() {
        let srgba = Srgba::new(0.0, 0.5, 1.0, 1.0);
        let linear_rgba: LinearRgba = srgba.into();
        assert_eq!(linear_rgba.red, 0.0);
        assert_approx_eq!(linear_rgba.green, 0.2140, 0.0001);
        assert_approx_eq!(linear_rgba.blue, 1.0, 0.0001);
        assert_eq!(linear_rgba.alpha, 1.0);
        let srgba2: Srgba = linear_rgba.into();
        assert_eq!(srgba2.red, 0.0);
        assert_approx_eq!(srgba2.green, 0.5, 0.0001);
        assert_approx_eq!(srgba2.blue, 1.0, 0.0001);
        assert_eq!(srgba2.alpha, 1.0);
    }

    #[test]
    fn euclidean_distance() {
        // White to black
        let a = Srgba::new(0.0, 0.0, 0.0, 1.0);
        let b = Srgba::new(1.0, 1.0, 1.0, 1.0);
        assert_eq!(a.distance_squared(&b), 3.0);

        // Alpha shouldn't matter
        let a = Srgba::new(0.0, 0.0, 0.0, 1.0);
        let b = Srgba::new(1.0, 1.0, 1.0, 0.0);
        assert_eq!(a.distance_squared(&b), 3.0);

        // Red to green
        let a = Srgba::new(0.0, 0.0, 0.0, 1.0);
        let b = Srgba::new(1.0, 0.0, 0.0, 1.0);
        assert_eq!(a.distance_squared(&b), 1.0);
    }

    #[test]
    fn darker_lighter() {
        // Darker and lighter should be commutative.
        let color = Srgba::new(0.4, 0.5, 0.6, 1.0);
        let darker1 = color.darker(0.1);
        let darker2 = darker1.darker(0.1);
        let twice_as_dark = color.darker(0.2);
        assert!(darker2.distance_squared(&twice_as_dark) < 0.0001);

        let lighter1 = color.lighter(0.1);
        let lighter2 = lighter1.lighter(0.1);
        let twice_as_light = color.lighter(0.2);
        assert!(lighter2.distance_squared(&twice_as_light) < 0.0001);
    }

    #[test]
    fn hex_color() {
        assert_eq!(Srgba::hex("FFF"), Ok(Srgba::WHITE));
        assert_eq!(Srgba::hex("FFFF"), Ok(Srgba::WHITE));
        assert_eq!(Srgba::hex("FFFFFF"), Ok(Srgba::WHITE));
        assert_eq!(Srgba::hex("FFFFFFFF"), Ok(Srgba::WHITE));
        assert_eq!(Srgba::hex("000"), Ok(Srgba::BLACK));
        assert_eq!(Srgba::hex("000F"), Ok(Srgba::BLACK));
        assert_eq!(Srgba::hex("000000"), Ok(Srgba::BLACK));
        assert_eq!(Srgba::hex("000000FF"), Ok(Srgba::BLACK));
        assert_eq!(Srgba::hex("03a9f4"), Ok(Srgba::rgb_u8(3, 169, 244)));
        assert_eq!(Srgba::hex("yy"), Err(HexColorError::Length));
        assert_eq!(Srgba::hex("#f2a"), Ok(Srgba::rgb_u8(255, 34, 170)));
        assert_eq!(Srgba::hex("#e23030"), Ok(Srgba::rgb_u8(226, 48, 48)));
        assert_eq!(Srgba::hex("#ff"), Err(HexColorError::Length));
        assert_eq!(Srgba::hex("11223344"), Ok(Srgba::rgba_u8(17, 34, 51, 68)));
        assert_eq!(Srgba::hex("1234"), Ok(Srgba::rgba_u8(17, 34, 51, 68)));
        assert_eq!(Srgba::hex("12345678"), Ok(Srgba::rgba_u8(18, 52, 86, 120)));
        assert_eq!(Srgba::hex("4321"), Ok(Srgba::rgba_u8(68, 51, 34, 17)));

        assert!(matches!(Srgba::hex("yyy"), Err(HexColorError::Parse(_))));
        assert!(matches!(Srgba::hex("##fff"), Err(HexColorError::Parse(_))));
    }
}
