use crate::color_difference::EuclideanDistance;
use crate::oklaba::Oklaba;
use crate::{Alpha, Hsla, LinearRgba, Luminance, Mix};
use bevy_math::Vec4;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::color::{Color, HexColorError, HslRepresentation, SrgbColorSpace};
use serde::{Deserialize, Serialize};

/// Non-linear standard RGB with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
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

impl Srgba {
    /// <div style="background-color:rgb(94%, 97%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ALICE_BLUE: Srgba = Srgba::new(0.94, 0.97, 1.0, 1.0);
    /// <div style="background-color:rgb(98%, 92%, 84%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ANTIQUE_WHITE: Srgba = Srgba::new(0.98, 0.92, 0.84, 1.0);
    /// <div style="background-color:rgb(49%, 100%, 83%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AQUAMARINE: Srgba = Srgba::new(0.49, 1.0, 0.83, 1.0);
    /// <div style="background-color:rgb(94%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AZURE: Srgba = Srgba::new(0.94, 1.0, 1.0, 1.0);
    /// <div style="background-color:rgb(96%, 96%, 86%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BEIGE: Srgba = Srgba::new(0.96, 0.96, 0.86, 1.0);
    /// <div style="background-color:rgb(100%, 89%, 77%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BISQUE: Srgba = Srgba::new(1.0, 0.89, 0.77, 1.0);
    /// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: Srgba = Srgba::new(0.0, 0.0, 0.0, 1.0);
    /// <div style="background-color:rgb(0%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE: Srgba = Srgba::new(0.0, 0.0, 1.0, 1.0);
    /// <div style="background-color:rgb(86%, 8%, 24%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CRIMSON: Srgba = Srgba::new(0.86, 0.08, 0.24, 1.0);
    /// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CYAN: Srgba = Srgba::new(0.0, 1.0, 1.0, 1.0);
    /// <div style="background-color:rgb(25%, 25%, 25%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GRAY: Srgba = Srgba::new(0.25, 0.25, 0.25, 1.0);
    /// <div style="background-color:rgb(0%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GREEN: Srgba = Srgba::new(0.0, 0.5, 0.0, 1.0);
    /// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const FUCHSIA: Srgba = Srgba::new(1.0, 0.0, 1.0, 1.0);
    /// <div style="background-color:rgb(100%, 84%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD: Srgba = Srgba::new(1.0, 0.84, 0.0, 1.0);
    /// <div style="background-color:rgb(50%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GRAY: Srgba = Srgba::new(0.5, 0.5, 0.5, 1.0);
    /// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN: Srgba = Srgba::new(0.0, 1.0, 0.0, 1.0);
    /// <div style="background-color:rgb(28%, 0%, 51%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const INDIGO: Srgba = Srgba::new(0.29, 0.0, 0.51, 1.0);
    /// <div style="background-color:rgb(20%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIME_GREEN: Srgba = Srgba::new(0.2, 0.8, 0.2, 1.0);
    /// <div style="background-color:rgb(50%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON: Srgba = Srgba::new(0.5, 0.0, 0.0, 1.0);
    /// <div style="background-color:rgb(10%, 10%, 44%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MIDNIGHT_BLUE: Srgba = Srgba::new(0.1, 0.1, 0.44, 1.0);
    /// <div style="background-color:rgb(0%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const NAVY: Srgba = Srgba::new(0.0, 0.0, 0.5, 1.0);
    /// <div style="background-color:rgba(0%, 0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    #[doc(alias = "transparent")]
    pub const NONE: Srgba = Srgba::new(0.0, 0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(50%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const OLIVE: Srgba = Srgba::new(0.5, 0.5, 0.0, 1.0);
    /// <div style="background-color:rgb(100%, 65%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE: Srgba = Srgba::new(1.0, 0.65, 0.0, 1.0);
    /// <div style="background-color:rgb(100%, 27%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE_RED: Srgba = Srgba::new(1.0, 0.27, 0.0, 1.0);
    /// <div style="background-color:rgb(100%, 8%, 57%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PINK: Srgba = Srgba::new(1.0, 0.08, 0.58, 1.0);
    /// <div style="background-color:rgb(50%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE: Srgba = Srgba::new(0.5, 0.0, 0.5, 1.0);
    /// <div style="background-color:rgb(100%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED: Srgba = Srgba::new(1.0, 0.0, 0.0, 1.0);
    /// <div style="background-color:rgb(98%, 50%, 45%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SALMON: Srgba = Srgba::new(0.98, 0.5, 0.45, 1.0);
    /// <div style="background-color:rgb(18%, 55%, 34%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SEA_GREEN: Srgba = Srgba::new(0.18, 0.55, 0.34, 1.0);
    /// <div style="background-color:rgb(75%, 75%, 75%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SILVER: Srgba = Srgba::new(0.75, 0.75, 0.75, 1.0);
    /// <div style="background-color:rgb(0%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL: Srgba = Srgba::new(0.0, 0.5, 0.5, 1.0);
    /// <div style="background-color:rgb(100%, 39%, 28%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TOMATO: Srgba = Srgba::new(1.0, 0.39, 0.28, 1.0);
    /// <div style="background-color:rgb(25%, 88%, 82%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TURQUOISE: Srgba = Srgba::new(0.25, 0.88, 0.82, 1.0);
    /// <div style="background-color:rgb(93%, 51%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const VIOLET: Srgba = Srgba::new(0.93, 0.51, 0.93, 1.0);
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: Srgba = Srgba::new(1.0, 1.0, 1.0, 1.0);
    /// <div style="background-color:rgb(100%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW: Srgba = Srgba::new(1.0, 1.0, 0.0, 1.0);
    /// <div style="background-color:rgb(60%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_GREEN: Srgba = Srgba::new(0.6, 0.8, 0.2, 1.0);

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

        match *hex.as_bytes() {
            // RGB
            [r, g, b] => {
                let [r, g, b, ..] = decode_hex([r, r, g, g, b, b])?;
                Ok(Self::rgb_u8(r, g, b))
            }
            // RGBA
            [r, g, b, a] => {
                let [r, g, b, a, ..] = decode_hex([r, r, g, g, b, b, a, a])?;
                Ok(Self::rgba_u8(r, g, b, a))
            }
            // RRGGBB
            [r1, r2, g1, g2, b1, b2] => {
                let [r, g, b, ..] = decode_hex([r1, r2, g1, g2, b1, b2])?;
                Ok(Self::rgb_u8(r, g, b))
            }
            // RRGGBBAA
            [r1, r2, g1, g2, b1, b2, a1, a2] => {
                let [r, g, b, a, ..] = decode_hex([r1, r2, g1, g2, b1, b2, a1, a2])?;
                Ok(Self::rgba_u8(r, g, b, a))
            }
            _ => Err(HexColorError::Length),
        }
    }

    /// Convert this color to CSS-style hexadecimal notation.
    pub fn to_hex(&self) -> String {
        let r = (self.red * 255.0).round() as u8;
        let g = (self.green * 255.0).round() as u8;
        let b = (self.blue * 255.0).round() as u8;
        let a = (self.alpha * 255.0).round() as u8;
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
        Self::rgba_u8(r, g, b, u8::MAX)
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
        Self::new(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
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
            .with_luminance(luminance.nonlinear_to_linear_srgb())
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

impl From<LinearRgba> for Srgba {
    #[inline]
    fn from(value: LinearRgba) -> Self {
        Self {
            red: value.red.linear_to_nonlinear_srgb(),
            green: value.green.linear_to_nonlinear_srgb(),
            blue: value.blue.linear_to_nonlinear_srgb(),
            alpha: value.alpha,
        }
    }
}

impl From<Hsla> for Srgba {
    fn from(value: Hsla) -> Self {
        let [r, g, b] =
            HslRepresentation::hsl_to_nonlinear_srgb(value.hue, value.saturation, value.lightness);
        Self::new(r, g, b, value.alpha)
    }
}

impl From<Oklaba> for Srgba {
    fn from(value: Oklaba) -> Self {
        Srgba::from(LinearRgba::from(value))
    }
}

impl From<Srgba> for Color {
    fn from(value: Srgba) -> Self {
        Color::Rgba {
            red: value.red,
            green: value.green,
            blue: value.blue,
            alpha: value.alpha,
        }
    }
}

impl From<Color> for Srgba {
    fn from(value: Color) -> Self {
        match value.as_rgba() {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Srgba::new(red, green, blue, alpha),
            _ => unreachable!(),
        }
    }
}

impl From<Srgba> for [f32; 4] {
    fn from(color: Srgba) -> Self {
        [color.red, color.green, color.blue, color.alpha]
    }
}

impl From<Srgba> for Vec4 {
    fn from(color: Srgba) -> Self {
        Vec4::new(color.red, color.green, color.blue, color.alpha)
    }
}

/// Converts hex bytes to an array of RGB\[A\] components
///
/// # Example
/// For RGB: *b"ffffff" -> [255, 255, 255, ..]
/// For RGBA: *b"E2E2E2FF" -> [226, 226, 226, 255, ..]
const fn decode_hex<const N: usize>(mut bytes: [u8; N]) -> Result<[u8; N], HexColorError> {
    let mut i = 0;
    while i < bytes.len() {
        // Convert single hex digit to u8
        let val = match hex_value(bytes[i]) {
            Ok(val) => val,
            Err(byte) => return Err(HexColorError::Char(byte as char)),
        };
        bytes[i] = val;
        i += 1;
    }
    // Modify the original bytes to give an `N / 2` length result
    i = 0;
    while i < bytes.len() / 2 {
        // Convert pairs of u8 to R/G/B/A
        // e.g `ff` -> [102, 102] -> [15, 15] = 255
        bytes[i] = bytes[i * 2] * 16 + bytes[i * 2 + 1];
        i += 1;
    }
    Ok(bytes)
}

/// Parse a single hex digit (a-f/A-F/0-9) as a `u8`
const fn hex_value(b: u8) -> Result<u8, u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        // Wrong hex digit
        _ => Err(b),
    }
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
        assert_eq!(Srgba::hex("yyy"), Err(HexColorError::Char('y')));
        assert_eq!(Srgba::hex("#f2a"), Ok(Srgba::rgb_u8(255, 34, 170)));
        assert_eq!(Srgba::hex("#e23030"), Ok(Srgba::rgb_u8(226, 48, 48)));
        assert_eq!(Srgba::hex("#ff"), Err(HexColorError::Length));
        assert_eq!(Srgba::hex("##fff"), Err(HexColorError::Char('#')));
    }
}
