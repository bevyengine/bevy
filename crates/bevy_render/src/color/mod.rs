use bevy_math::{Vec3, Vec4};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, MulAssign};
use thiserror::Error;

pub use palette;

use palette::{convert::FromColorUnclamped, encoding, rgb::Rgb, Clamp, IntoColor, Srgb, WithAlpha};

/// Concrete [`Color`] which is used to unify the various color-interacting systems
/// across this crate. The core of this type is its interop with [`palette`], [`encase`],
/// [`wgpu`], [`serde`], and [`bevy_reflect`].
///
/// To perform color manipulation, it is recommended to treat [`Color`] as an at-rest type, and
/// then use [`palette`] methods to enter and exit from a colorspace that best matches your
/// desired operations.
#[derive(
    FromColorUnclamped, WithAlpha, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect,
)]
#[reflect(PartialEq, Serialize, Deserialize)]
#[palette(skip_derives(Rgb), rgb_standard = "encoding::Srgb")]
pub struct Color {
    /// Red channel. [0.0, 1.0]
    r: f32,
    /// Green channel. [0.0, 1.0]
    g: f32,
    /// Blue channel. [0.0, 1.0]
    b: f32,
    /// Alpha channel. [0.0, 1.0]
    #[palette(alpha)]
    a: f32,
}

impl Color {
    /// <div style="background-color:rgb(94%, 97%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ALICE_BLUE: Color = Color::rgb(0.94, 0.97, 1.0);
    /// <div style="background-color:rgb(98%, 92%, 84%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ANTIQUE_WHITE: Color = Color::rgb(0.98, 0.92, 0.84);
    /// <div style="background-color:rgb(49%, 100%, 83%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AQUAMARINE: Color = Color::rgb(0.49, 1.0, 0.83);
    /// <div style="background-color:rgb(94%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AZURE: Color = Color::rgb(0.94, 1.0, 1.0);
    /// <div style="background-color:rgb(96%, 96%, 86%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BEIGE: Color = Color::rgb(0.96, 0.96, 0.86);
    /// <div style="background-color:rgb(100%, 89%, 77%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BISQUE: Color = Color::rgb(1.0, 0.89, 0.77);
    /// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(0%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    /// <div style="background-color:rgb(86%, 8%, 24%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CRIMSON: Color = Color::rgb(0.86, 0.08, 0.24);
    /// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CYAN: Color = Color::rgb(0.0, 1.0, 1.0);
    /// <div style="background-color:rgb(25%, 25%, 25%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GRAY: Color = Color::rgb(0.25, 0.25, 0.25);
    /// <div style="background-color:rgb(0%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GREEN: Color = Color::rgb(0.0, 0.5, 0.0);
    /// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const FUCHSIA: Color = Color::rgb(1.0, 0.0, 1.0);
    /// <div style="background-color:rgb(100%, 84%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD: Color = Color::rgb(1.0, 0.84, 0.0);
    /// <div style="background-color:rgb(50%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GRAY: Color = Color::rgb(0.5, 0.5, 0.5);
    /// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    /// <div style="background-color:rgb(28%, 0%, 51%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const INDIGO: Color = Color::rgb(0.29, 0.0, 0.51);
    /// <div style="background-color:rgb(20%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIME_GREEN: Color = Color::rgb(0.2, 0.8, 0.2);
    /// <div style="background-color:rgb(50%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON: Color = Color::rgb(0.5, 0.0, 0.0);
    /// <div style="background-color:rgb(10%, 10%, 44%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MIDNIGHT_BLUE: Color = Color::rgb(0.1, 0.1, 0.44);
    /// <div style="background-color:rgb(0%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const NAVY: Color = Color::rgb(0.0, 0.0, 0.5);
    /// <div style="background-color:rgba(0%, 0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    #[doc(alias = "transparent")]
    pub const NONE: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(50%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const OLIVE: Color = Color::rgb(0.5, 0.5, 0.0);
    /// <div style="background-color:rgb(100%, 65%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE: Color = Color::rgb(1.0, 0.65, 0.0);
    /// <div style="background-color:rgb(100%, 27%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE_RED: Color = Color::rgb(1.0, 0.27, 0.0);
    /// <div style="background-color:rgb(100%, 8%, 57%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PINK: Color = Color::rgb(1.0, 0.08, 0.58);
    /// <div style="background-color:rgb(50%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE: Color = Color::rgb(0.5, 0.0, 0.5);
    /// <div style="background-color:rgb(100%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    /// <div style="background-color:rgb(98%, 50%, 45%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SALMON: Color = Color::rgb(0.98, 0.5, 0.45);
    /// <div style="background-color:rgb(18%, 55%, 34%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SEA_GREEN: Color = Color::rgb(0.18, 0.55, 0.34);
    /// <div style="background-color:rgb(75%, 75%, 75%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SILVER: Color = Color::rgb(0.75, 0.75, 0.75);
    /// <div style="background-color:rgb(0%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL: Color = Color::rgb(0.0, 0.5, 0.5);
    /// <div style="background-color:rgb(100%, 39%, 28%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TOMATO: Color = Color::rgb(1.0, 0.39, 0.28);
    /// <div style="background-color:rgb(25%, 88%, 82%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TURQUOISE: Color = Color::rgb(0.25, 0.88, 0.82);
    /// <div style="background-color:rgb(93%, 51%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const VIOLET: Color = Color::rgb(0.93, 0.51, 0.93);
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    /// <div style="background-color:rgb(100%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW: Color = Color::rgb(1.0, 1.0, 0.0);
    /// <div style="background-color:rgb(60%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_GREEN: Color = Color::rgb(0.6, 0.8, 0.2);

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    /// * `a` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgb`], [`Color::rgba_u8`], [`Color::hex`].
    ///
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Converts this `Color` into `sRGBA`
    #[must_use]
    pub fn as_rgba(self) -> palette::Srgba {
        self.in_space()
    }

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgba`], [`Color::rgb_u8`], [`Color::hex`].
    ///
    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.)
    }

    /// Converts this `Color` into sRGB
    #[must_use]
    pub fn as_rgb(self) -> palette::Srgb {
        self.in_space()
    }

    /// New `Color` from linear RGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    /// * `a` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgba`], [`Color::rgb_linear`].
    ///
    #[must_use]
    pub fn rgba_linear(r: f32, g: f32, b: f32, a: f32) -> Self {
        palette::LinSrgba::new(r, g, b, a).into_color()
    }

    /// Converts this `Color` into Linear `sRGBA`
    #[must_use]
    pub fn as_rgba_linear(self) -> palette::LinSrgba {
        self.in_space()
    }

    /// New `Color` from linear RGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgb`], [`Color::rgba_linear`].
    ///
    #[must_use]
    pub fn rgb_linear(r: f32, g: f32, b: f32) -> Self {
        Self::rgba_linear(r, g, b, 1.)
    }

    /// Converts this `Color` into Linear sRGB
    #[must_use]
    pub fn as_rgb_linear(self) -> palette::LinSrgb {
        self.in_space()
    }

    /// New `Color` with HSL representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::hsl`].
    ///
    #[must_use]
    pub fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Self {
        palette::Hsla::new(hue, saturation, lightness, alpha).into_color()
    }

    /// Converts this `Color` into HSLA
    #[must_use]
    pub fn as_hsla(self) -> palette::Hsla {
        self.in_space()
    }

    /// New `Color` with HSL representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    ///
    /// See also [`Color::hsla`].
    ///
    #[must_use]
    pub fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
        Self::hsla(hue, saturation, lightness, 1.)
    }

    /// Converts this `Color` into HSL
    #[must_use]
    pub fn as_hsl(self) -> palette::Hsl {
        self.in_space()
    }

    /// New `Color` with HSV representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `value` - Value channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::hsv`].
    ///
    #[must_use]
    pub fn hsva(hue: f32, saturation: f32, value: f32, alpha: f32) -> Self {
        palette::Hsva::new(hue, saturation, value, alpha).into_color()
    }

    /// Converts this `Color` into HSVA
    #[must_use]
    pub fn as_hsva(self) -> palette::Hsva {
        self.in_space()
    }

    /// New `Color` with HSV representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Value channel. [0.0, 1.0]
    ///
    /// See also [`Color::hsva`].
    ///
    #[must_use]
    pub fn hsv(hue: f32, saturation: f32, value: f32) -> Color {
        Self::hsva(hue, saturation, value, 1.)
    }

    /// Converts this `Color` into HSV
    #[must_use]
    pub fn as_hsv(self) -> palette::Hsv {
        self.in_space()
    }

    /// New `Color` with LCH representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 150.0]
    /// * `chroma` - Chroma channel. [0.0, 150.0]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::lch`].
    #[must_use]
    pub fn lcha(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Color {
        palette::Lcha::new(lightness, chroma, hue, alpha).into_color()
    }

    /// Converts this `Color` into LCHA
    #[must_use]
    pub fn as_lcha(self) -> palette::Lcha {
        self.in_space()
    }

    /// New `Color` with LCH representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 150.0]
    /// * `chroma` - Chroma channel. [0.0, 150.0]
    /// * `hue` - Hue channel. [0.0, 360.0]
    ///
    /// See also [`Color::lcha`].
    #[must_use]
    pub fn lch(lightness: f32, chroma: f32, hue: f32) -> Color {
        Self::lcha(lightness, chroma, hue, 1.)
    }

    /// Converts this `Color` into LCH
    #[must_use]
    pub fn as_lch(self) -> palette::Lch {
        self.in_space()
    }

    /// New `Color` with Oklcha representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::oklch`].
    #[must_use]
    pub fn oklcha(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Color {
        palette::Oklcha::new(lightness, chroma, hue, alpha).into_color()
    }

    /// Converts this `Color` into LCHA
    #[must_use]
    pub fn as_oklcha(self) -> palette::Oklcha {
        self.in_space()
    }

    /// New `Color` with Oklch representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    ///
    /// See also [`Color::lcha`].
    #[must_use]
    pub fn oklch(lightness: f32, chroma: f32, hue: f32) -> Color {
        Self::oklcha(lightness, chroma, hue, 1.)
    }

    /// Converts this `Color` into Oklch
    #[must_use]
    pub fn as_oklch(self) -> palette::Oklch {
        self.in_space()
    }

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0, 255]
    /// * `g` - Green channel. [0, 255]
    /// * `b` - Blue channel. [0, 255]
    /// * `a` - Alpha channel. [0, 255]
    ///
    /// See also [`Color::rgba`], [`Color::rgb_u8`], [`Color::hex`].
    ///
    #[must_use]
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        palette::Srgba::<u8>::new(r, g, b, a)
            .into_format()
            .into_color()
    }

    /// Converts this `Color` into `sRGBA` u8
    #[must_use]
    pub fn as_rgba_u8(self) -> palette::rgb::PackedRgba {
        palette::rgb::PackedRgba::pack(palette::Srgba::from_color_unclamped(self).into_format())
    }

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0, 255]
    /// * `g` - Green channel. [0, 255]
    /// * `b` - Blue channel. [0, 255]
    ///
    /// See also [`Color::rgb`], [`Color::rgba_u8`], [`Color::hex`].
    ///
    #[must_use]
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Color {
        palette::Srgb::<u8>::new(r, g, b).into_format().into_color()
    }

    /// Converts this `Color` into RGB u8
    #[must_use]
    pub fn as_rgb_u8(self) -> palette::rgb::PackedArgb {
        palette::rgb::PackedArgb::pack(palette::Srgba::from_color_unclamped(self).into_format())
    }

    /// Converts `Color` to a `u32` from sRGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    #[must_use]
    pub fn as_rgba_u32(self) -> u32 {
        self.as_rgba_u8().into()
    }

    /// Converts this `Color` into Linear RGB u8
    #[must_use]
    pub fn as_linear_rgba_u8(self) -> palette::rgb::PackedRgba {
        palette::rgb::PackedRgba::pack(palette::LinSrgba::from_color_unclamped(self).into_format())
    }

    /// Converts `Color` to a `u32` from Linear sRGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    #[must_use]
    pub fn as_linear_rgba_u32(self) -> u32 {
        self.as_linear_rgba_u8().into()
    }

    /// Converts this `Color` into 4 `sRGBA` channels in an array.
    #[must_use]
    pub fn as_rgba_f32(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Converts this `Color` into 3 `sRGB` channels in an array.
    #[must_use]
    pub fn as_rgb_f32(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }

    /// Converts this `Color` into 4 Linear `sRGBA` channels in an array.
    #[must_use]
    pub fn as_linear_rgba_f32(self) -> [f32; 4] {
        self.as_rgba_linear().into()
    }

    /// Converts this `Color` into 3 Linear `sRGB` channels in an array.
    #[must_use]
    pub fn as_linear_rgb_f32(self) -> [f32; 3] {
        self.as_rgb_linear().into()
    }

    /// New `Color` from sRGB(A) colorspace.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_render::color::Color;
    /// let color = Color::hex("FF00FF").unwrap(); // fuchsia
    /// let color = Color::hex("FF00FF7F").unwrap(); // partially transparent fuchsia
    ///
    /// // A standard hex color notation is also available
    /// assert_eq!(Color::hex("#FFFFFF").unwrap(), Color::rgb(1.0, 1.0, 1.0));
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

    /// Returns red in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// assert_eq!(Color::RED.r(), 1.0);
    /// ```
    #[must_use]
    pub const fn r(&self) -> f32 {
        self.r
    }

    /// Returns green in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// assert_eq!(Color::GREEN.g(), 1.0);
    /// ```
    #[must_use]
    pub const fn g(&self) -> f32 {
        self.g
    }

    /// Returns blue in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// assert_eq!(Color::BLUE.b(), 1.0);
    /// ```
    #[must_use]
    pub const fn b(&self) -> f32 {
        self.b
    }

    /// Returns alpha in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// assert_eq!(Color::RED.a(), 1.0);
    /// ```
    #[must_use]
    pub const fn a(&self) -> f32 {
        self.a
    }

    /// Replaces the red channel with the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let blue = Color::BLUE;
    /// # assert_eq!(blue.r(), 0.0);
    /// # assert_eq!(blue.b(), 1.0);
    /// let purple = blue.with_r(1.0);
    /// # assert_eq!(purple.r(), 1.0);
    /// # assert_eq!(purple.b(), 1.0);
    /// ```
    #[must_use]
    pub const fn with_r(self, r: f32) -> Self {
        Self { r, ..self }
    }

    /// Replaces the green channel with the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let blue = Color::BLUE;
    /// # assert_eq!(blue.b(), 1.0);
    /// # assert_eq!(blue.g(), 0.0);
    /// let cyan = blue.with_g(1.0);
    /// # assert_eq!(cyan.b(), 1.0);
    /// # assert_eq!(cyan.g(), 1.0);
    /// ```
    #[must_use]
    pub const fn with_g(self, g: f32) -> Self {
        Self { g, ..self }
    }

    /// Replaces the blue channel with the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let red = Color::RED;
    /// # assert_eq!(red.r(), 1.0);
    /// # assert_eq!(red.b(), 0.0);
    /// let purple = red.with_b(1.0);
    /// # assert_eq!(purple.r(), 1.0);
    /// # assert_eq!(purple.b(), 1.0);
    /// ```
    #[must_use]
    pub const fn with_b(self, b: f32) -> Self {
        Self { b, ..self }
    }

    /// Replaces the alpha channel with the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let red = Color::RED;
    /// # assert_eq!(red.r(), 1.0);
    /// let transparent_red = red.with_r(0.5);
    /// # assert_eq!(transparent_red.r(), 0.5);
    /// ```
    #[must_use]
    pub const fn with_a(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Sets the red channel to the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let mut red = Color::RED;
    /// # assert_eq!(red.r(), 1.0);
    /// red.set_r(0.5);
    /// # assert_eq!(red.r(), 0.5);
    /// ```
    pub fn set_r(&mut self, r: f32) -> &mut Self {
        *self = self.with_r(r);
        self
    }

    /// Sets the green channel to the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let mut green = Color::GREEN;
    /// # assert_eq!(green.g(), 1.0);
    /// green.set_g(0.5);
    /// # assert_eq!(green.g(), 0.5);
    /// ```
    pub fn set_g(&mut self, g: f32) -> &mut Self {
        *self = self.with_g(g);
        self
    }

    /// Sets the blue channel to the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let mut blue = Color::BLUE;
    /// # assert_eq!(blue.b(), 1.0);
    /// blue.set_b(0.5);
    /// # assert_eq!(blue.b(), 0.5);
    /// ```
    pub fn set_b(&mut self, b: f32) -> &mut Self {
        *self = self.with_b(b);
        self
    }

    /// Sets the alpha channel to the provided value in `sRGBA`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::Color;
    /// #
    /// let mut red = Color::RED;
    /// # assert_eq!(red.a(), 1.0);
    /// red.set_a(0.5);
    /// # assert_eq!(red.a(), 0.5);
    /// ```
    pub fn set_a(&mut self, a: f32) -> &mut Self {
        *self = self.with_a(a);
        self
    }

    /// Convert this `Color` into a specific colorspace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::Oklab};
    /// #
    /// let color = Color::RED.in_space::<Oklab>();
    /// // ...
    /// let color = Color::from_space(color);
    /// ```
    ///
    /// See also [`from_space`](`Color::from_space`)
    ///
    #[must_use]
    pub fn in_space<C>(self) -> C
    where
        C: FromColorUnclamped<Self> + palette::Clamp,
    {
        self.into_color()
    }

    /// Convert a provided arbitrary color into a `Color`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::Oklab};
    /// #
    /// let color = Color::RED.in_space::<Oklab>();
    /// // ...
    /// let color = Color::from_space(color);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`)
    ///
    #[must_use]
    pub fn from_space<C>(color: C) -> Self
    where
        Self: FromColorUnclamped<C> + palette::Clamp,
    {
        color.into_color()
    }

    /// Mix this `Color` with another at a certain ratio, within a specific colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::{Srgba, Lab}};
    /// #
    /// let start = Color::RED;
    /// let end = Color::BLUE;
    ///
    /// // Purple
    /// let middle = start.mix_in::<Srgba>(end, 0.5);
    ///
    /// assert!(middle.difference_in::<Lab>(Color::PURPLE).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`)
    ///
    #[must_use]
    pub fn mix_in<C>(self, rhs: Self, amount: <C as palette::Mix>::Scalar) -> Self
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::Mix,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>()
            .mix(rhs.in_space(), amount)
            .into_color()
    }

    /// Calculates the hue of this `Color` in the specified colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::Hsla};
    /// #
    /// let red = Color::RED;
    /// let hue = red.hue_in::<Hsla>().into_positive_degrees();
    /// assert_eq!(hue, 0.);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`), [`shift_hue_in`](`Color::shift_hue_in`), [`with_hue_in`](`Color::with_hue_in`)
    ///
    #[must_use]
    pub fn hue_in<C>(self) -> <C as palette::GetHue>::Hue
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::GetHue,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().get_hue()
    }

    /// Sets the hue of this `Color` to a provided value within the specified colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::{Hsla, Lab}};
    /// #
    /// let red = Color::RED;
    /// let green = red.with_hue_in::<Hsla>(120.);
    /// let blue = red.with_hue_in::<Hsla>(240.);
    ///
    /// assert!(green.difference_in::<Lab>(Color::GREEN).abs() < 1e-3);
    /// assert!(blue.difference_in::<Lab>(Color::BLUE).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`), [`shift_hue_in`](`Color::shift_hue_in`), [`hue_in`](`Color::hue_in`)
    ///
    #[must_use]
    pub fn with_hue_in<C>(self, hue: f32) -> Self
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::WithHue<f32>,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().with_hue(hue).into_color()
    }

    /// Shift the hue of this `Color` by a provided amount, within a specific colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::{Hsla, Lab}};
    /// #
    /// // Dark Yellow
    /// let primary = Color::hsl(45., 1.0, 0.5);
    ///
    /// // Light Blue
    /// let compliment = primary.shift_hue_in::<Hsla>(180.);
    ///
    /// assert!(compliment.difference_in::<Lab>(Color::hsl(45. + 180., 1.0, 0.5)).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`), [`with_hue_in`](`Color::with_hue_in`), [`hue_in`](`Color::hue_in`)
    ///
    #[must_use]
    pub fn shift_hue_in<C>(self, amount: <C as palette::ShiftHue>::Scalar) -> Self
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::ShiftHue,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().shift_hue(amount).into_color()
    }

    /// Saturate this `Color` by a provided amount, within a specific colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::{Hsla, Lab}};
    /// #
    /// // 50% Saturation
    /// let primary = Color::hsl(45., 0.5, 0.5);
    ///
    /// // 75% Saturation
    /// let primary_saturated = primary.saturate_in::<Hsla>(0.5);
    ///
    /// assert!(primary_saturated.difference_in::<Lab>(Color::hsl(45., 0.75, 0.5)).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`), [`desaturate_in`](`Color::desaturate_in`)
    ///
    #[must_use]
    pub fn saturate_in<C>(self, amount: <C as palette::Saturate>::Scalar) -> Self
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::Saturate,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().saturate(amount).into_color()
    }

    /// Desaturate this `Color` by a provided amount, within a specific colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::{Hsla, Lab}};
    /// #
    /// // 50% Saturation
    /// let primary = Color::hsl(45., 0.5, 0.5);
    ///
    /// // 25% Saturation
    /// let primary_desaturated = primary.desaturate_in::<Hsla>(0.5);
    ///
    /// assert!(primary_desaturated.difference_in::<Lab>(Color::hsl(45., 0.25, 0.5)).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`), [`saturate_in`](`Color::saturate_in`)
    ///
    #[must_use]
    pub fn desaturate_in<C>(self, amount: <C as palette::Desaturate>::Scalar) -> Self
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::Desaturate,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().desaturate(amount).into_color()
    }

    /// Lighten this `Color` by a provided amount, within a specific colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::{Hsla, Lab}};
    /// #
    /// // 50% Lightness
    /// let primary = Color::hsl(45., 0.5, 0.5);
    ///
    /// // 75% Saturation
    /// let primary_light = primary.lighten_in::<Hsla>(0.5);
    ///
    /// assert!(primary_light.difference_in::<Lab>(Color::hsl(45., 0.5, 0.75)).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`), [`darken_in`](`Color::darken_in`)
    ///
    #[must_use]
    pub fn lighten_in<C>(self, amount: <C as palette::Lighten>::Scalar) -> Self
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::Lighten,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().lighten(amount).into_color()
    }

    /// Darken this `Color` by a provided amount, within a specific colorspace.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::{Hsla, Lab}};
    /// #
    /// // 50% Lightness
    /// let primary = Color::hsl(45., 0.5, 0.5);
    ///
    /// // 25% Lightness
    /// let primary_dark = primary.darken_in::<Hsla>(0.5);
    ///
    /// assert!(primary_dark.difference_in::<Lab>(Color::hsl(45., 0.5, 0.25)).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`), [`lighten_in`](`Color::lighten_in`)
    ///
    #[must_use]
    pub fn darken_in<C>(self, amount: <C as palette::Darken>::Scalar) -> Self
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::Darken,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().darken(amount).into_color()
    }

    /// Compare this `Color` with another, within a specific colorspace, using the
    /// CIEDE2000 color difference metric.
    ///
    /// If you intend on performing multiple operations in an alternate colorspace, you
    /// should use [`in_space`](`Self::in_space`) to explicitly transform into that space first,
    /// perform your changes, then return back into this space using [`from_space`](`Self::from_space`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_render::color::{Color, palette::Lch};
    /// #
    /// let a = Color::RED;
    /// let b = Color::hsl(0., 1.0, 0.5);
    ///
    /// assert!(a.difference_in::<Lch>(b).abs() < 1e-3);
    /// ```
    ///
    /// See also [`in_space`](`Color::in_space`), [`from_space`](`Color::from_space`)
    ///
    #[must_use]
    pub fn difference_in<C>(self, rhs: Self) -> <C as palette::color_difference::Ciede2000>::Scalar
    where
        C: FromColorUnclamped<Self> + palette::Clamp + palette::color_difference::Ciede2000,
        Self: FromColorUnclamped<C>,
    {
        self.in_space::<C>().difference(rhs.in_space::<C>())
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

// Convert from any kind of f32 sRGB.
impl<S> FromColorUnclamped<Rgb<S, f32>> for Color
where
    Srgb: FromColorUnclamped<Rgb<S, f32>>,
{
    fn from_color_unclamped(color: Rgb<S, f32>) -> Color {
        let srgb = Srgb::from_color_unclamped(color);
        Color {
            r: srgb.red,
            g: srgb.green,
            b: srgb.blue,
            a: 1.0,
        }
    }
}

// Convert into any kind of f32 sRGB.
impl<S> FromColorUnclamped<Color> for Rgb<S, f32>
where
    Rgb<S, f32>: FromColorUnclamped<Srgb>,
{
    fn from_color_unclamped(color: Color) -> Self {
        let srgb = Srgb::new(color.r, color.g, color.b);
        Self::from_color_unclamped(srgb)
    }
}

// There's no blanket implementation for Self -> Self, unlike the From trait.
// This is to better allow cases like Self<A> -> Self<B>.
impl FromColorUnclamped<Color> for Color {
    fn from_color_unclamped(color: Color) -> Color {
        color
    }
}

// Add the required clamping.
impl Clamp for Color {
    fn clamp(self) -> Self {
        Color {
            r: self.r.clamp(0., 1.),
            g: self.g.clamp(0., 1.),
            b: self.b.clamp(0., 1.),
            a: self.a.clamp(0., 1.),
        }
    }
}

impl AddAssign<Color> for Color {
    fn add_assign(&mut self, rhs: Color) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
        self.a += rhs.a;
    }
}

impl Add<Color> for Color {
    type Output = Color;

    fn add(mut self, rhs: Color) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<Vec4> for Color {
    fn add_assign(&mut self, rhs: Vec4) {
        let rhs: Color = rhs.into();
        *self += rhs;
    }
}

impl Add<Vec4> for Color {
    type Output = Color;

    fn add(self, rhs: Vec4) -> Self::Output {
        let rhs: Color = rhs.into();
        self + rhs
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<[f32; 3]> for Color {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Color::rgb(r, g, b)
    }
}

impl From<Color> for Vec3 {
    fn from(color: Color) -> Self {
        color.as_rgb_f32().into()
    }
}

impl From<Color> for Vec4 {
    fn from(color: Color) -> Self {
        color.as_rgba_f32().into()
    }
}

impl From<Vec4> for Color {
    fn from(vec4: Vec4) -> Self {
        Color::rgba(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        color.as_rgba_f32()
    }
}

impl From<Color> for [f32; 3] {
    fn from(color: Color) -> Self {
        color.as_rgb_f32()
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        let Color { r, g, b, a } = color;

        wgpu::Color {
            r: r as f64,
            g: g as f64,
            b: b as f64,
            a: a as f64,
        }
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<Vec4> for Color {
    fn mul_assign(&mut self, rhs: Vec4) {
        self.r *= rhs.x;
        self.g *= rhs.y;
        self.b *= rhs.z;
        self.a *= rhs.w;
    }
}

impl Mul<Vec4> for Color {
    type Output = Color;

    fn mul(mut self, rhs: Vec4) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<Vec3> for Color {
    fn mul_assign(&mut self, rhs: Vec3) {
        self.r *= rhs.x;
        self.g *= rhs.y;
        self.b *= rhs.z;
    }
}

impl Mul<Vec3> for Color {
    type Output = Color;

    fn mul(mut self, rhs: Vec3) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<[f32; 4]> for Color {
    fn mul_assign(&mut self, rhs: [f32; 4]) {
        *self *= Vec4::from(rhs);
    }
}

impl Mul<[f32; 4]> for Color {
    type Output = Color;

    fn mul(mut self, rhs: [f32; 4]) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<[f32; 3]> for Color {
    fn mul_assign(&mut self, rhs: [f32; 3]) {
        *self *= Vec3::from(rhs);
    }
}

impl Mul<[f32; 3]> for Color {
    type Output = Color;

    fn mul(mut self, rhs: [f32; 3]) -> Self::Output {
        self *= rhs;
        self
    }
}

impl encase::ShaderType for Color {
    type ExtraMetadata = ();

    const METADATA: encase::private::Metadata<Self::ExtraMetadata> = {
        let size =
            encase::private::SizeValue::from(<f32 as encase::private::ShaderSize>::SHADER_SIZE)
                .mul(4);
        let alignment = encase::private::AlignmentValue::from_next_power_of_two_size(size);

        encase::private::Metadata {
            alignment,
            has_uniform_min_alignment: false,
            min_size: size,
            extra: (),
        }
    };

    const UNIFORM_COMPAT_ASSERT: fn() = || {};
}

impl encase::private::WriteInto for Color {
    fn write_into<B: encase::private::BufferMut>(&self, writer: &mut encase::private::Writer<B>) {
        let linear = self.as_rgba_linear();
        encase::private::WriteInto::write_into(&linear.red, writer);
        encase::private::WriteInto::write_into(&linear.green, writer);
        encase::private::WriteInto::write_into(&linear.blue, writer);
        encase::private::WriteInto::write_into(&linear.alpha, writer);
    }
}

impl encase::private::ReadFrom for Color {
    fn read_from<B: encase::private::BufferRef>(
        &mut self,
        reader: &mut encase::private::Reader<B>,
    ) {
        let mut buffer = [0.0f32; 4];
        for el in &mut buffer {
            encase::private::ReadFrom::read_from(el, reader);
        }

        *self = Color::rgba_linear(buffer[0], buffer[1], buffer[2], buffer[3]);
    }
}

impl encase::private::CreateFrom for Color {
    fn create_from<B>(reader: &mut encase::private::Reader<B>) -> Self
    where
        B: encase::private::BufferRef,
    {
        // These are intentionally not inlined in the constructor to make this
        // resilient to internal Color refactors / implicit type changes.
        let red: f32 = encase::private::CreateFrom::create_from(reader);
        let green: f32 = encase::private::CreateFrom::create_from(reader);
        let blue: f32 = encase::private::CreateFrom::create_from(reader);
        let alpha: f32 = encase::private::CreateFrom::create_from(reader);
        Color::rgba_linear(red, green, blue, alpha)
    }
}

impl encase::ShaderSize for Color {}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HexColorError {
    #[error("Unexpected length of hex string")]
    Length,
    #[error("Invalid hex char")]
    Char(char),
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
    use super::*;

    #[test]
    fn hex_color() {
        assert_eq!(Color::hex("FFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("FFFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("FFFFFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("FFFFFFFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("000"), Ok(Color::BLACK));
        assert_eq!(Color::hex("000F"), Ok(Color::BLACK));
        assert_eq!(Color::hex("000000"), Ok(Color::BLACK));
        assert_eq!(Color::hex("000000FF"), Ok(Color::BLACK));
        assert_eq!(Color::hex("03a9f4"), Ok(Color::rgb_u8(3, 169, 244)));
        assert_eq!(Color::hex("yy"), Err(HexColorError::Length));
        assert_eq!(Color::hex("yyy"), Err(HexColorError::Char('y')));
        assert_eq!(Color::hex("#f2a"), Ok(Color::rgb_u8(255, 34, 170)));
        assert_eq!(Color::hex("#e23030"), Ok(Color::rgb_u8(226, 48, 48)));
        assert_eq!(Color::hex("#ff"), Err(HexColorError::Length));
        assert_eq!(Color::hex("##fff"), Err(HexColorError::Char('#')));
    }

    #[test]
    fn conversions_vec4() {
        let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
        let starting_color = Color::from(starting_vec4);

        assert_eq!(starting_vec4, Vec4::from(starting_color),);

        let transformation = Vec4::new(0.5, 0.5, 0.5, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::from(starting_vec4 * transformation),
        );
    }

    #[test]
    fn mul_and_mulassign_f32() {
        let transformation = 0.5;
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_f32by3() {
        let transformation = [0.4, 0.5, 0.6];
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_f32by4() {
        let transformation = [0.4, 0.5, 0.6, 0.9];
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0 * 0.9),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_vec3() {
        let transformation = Vec3::new(0.2, 0.3, 0.4);
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_vec4() {
        let transformation = Vec4::new(0.2, 0.3, 0.4, 0.5);
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0 * 0.5),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn lerp() {
        let color_start = Color::BLUE;
        let color_end = Color::RED;

        let color_mix = color_start.mix_in::<palette::Srgba>(color_end, 0.5);

        assert!(color_mix.difference_in::<palette::Lch>(Color::PURPLE).abs() < 1e-3);

        let color_mix = color_start.mix_in::<palette::LinSrgba>(color_end, 0.5);

        assert!(
            color_mix
                .difference_in::<palette::Lch>(Color::rgb(0.7353569, 0., 0.7353569))
                .abs()
                < 1e-3
        );

        let color_mix = color_start.mix_in::<palette::Oklcha>(color_end, 0.5);

        assert!(
            color_mix
                .difference_in::<palette::Lch>(Color::rgb(0.7299066, 0., 0.7600829))
                .abs()
                < 1e-3
        );
    }

    #[test]
    fn hue_shift() {
        let color_start = Color::hsl(45., 0.5, 0.5);
        let color_complement = color_start.shift_hue_in::<palette::Hsla>(180.);
        let color_complement_ref = Color::hsl(45. + 180., 0.5, 0.5);

        assert!(
            color_complement
                .difference_in::<palette::Lch>(color_complement_ref)
                .abs()
                < 1e-3
        );

        use super::palette::ShiftHue;

        let color_complement = color_start.in_space::<palette::Hsla>();
        let color_complement = color_complement
            .shift_hue(60.)
            .shift_hue(60.)
            .shift_hue(60.);
        let color_complement = Color::from_space(color_complement);

        assert!(
            color_complement
                .difference_in::<palette::Lch>(color_complement_ref)
                .abs()
                < 1e-3
        );
    }

    #[test]
    fn key_colors() {
        fn assert_color_approx(a: Color, b: Color) {
            assert!(a.difference_in::<palette::Lch>(b).abs() < 1e-2);
        }

        assert_color_approx(Color::RED, Color::hsl(0., 1.0, 0.5));
        assert_color_approx(Color::RED, Color::hsv(0., 1.0, 1.0));
        assert_color_approx(Color::RED, Color::lch(53.23711, 104.55, 39.99987));
        assert_color_approx(Color::RED, Color::oklch(0.62796, 0.25768, 29.23388));

        assert_color_approx(Color::GREEN, Color::hsl(120., 1.0, 0.5));
        assert_color_approx(Color::GREEN, Color::hsv(120., 1.0, 1.0));
        assert_color_approx(Color::GREEN, Color::lch(87.73552, 119.78014, 136.01306));
        assert_color_approx(Color::GREEN, Color::oklch(0.86644, 0.29483, 142.49535));

        assert_color_approx(Color::BLUE, Color::hsl(240., 1.0, 0.5));
        assert_color_approx(Color::BLUE, Color::hsv(240., 1.0, 1.0));
        assert_color_approx(Color::BLUE, Color::lch(32.30087, 133.80843, 306.28882));
        assert_color_approx(Color::BLUE, Color::oklch(0.45201, 0.31321, 264.05203));
    }
}
