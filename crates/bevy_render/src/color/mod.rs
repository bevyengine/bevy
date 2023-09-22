mod colorspace;

pub use colorspace::*;

use bevy_math::{Vec3, Vec4};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, MulAssign};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Color {
    /// sRGBA color
    Rgba {
        /// Red channel. [0.0, 1.0]
        red: f32,
        /// Green channel. [0.0, 1.0]
        green: f32,
        /// Blue channel. [0.0, 1.0]
        blue: f32,
        /// Alpha channel. [0.0, 1.0]
        alpha: f32,
    },
    /// RGBA color in the Linear sRGB colorspace (often colloquially referred to as "linear", "RGB", or "linear RGB").
    RgbaLinear {
        /// Red channel. [0.0, 1.0]
        red: f32,
        /// Green channel. [0.0, 1.0]
        green: f32,
        /// Blue channel. [0.0, 1.0]
        blue: f32,
        /// Alpha channel. [0.0, 1.0]
        alpha: f32,
    },
    /// HSL (hue, saturation, lightness) color with an alpha channel
    Hsla {
        /// Hue channel. [0.0, 360.0]
        hue: f32,
        /// Saturation channel. [0.0, 1.0]
        saturation: f32,
        /// Lightness channel. [0.0, 1.0]
        lightness: f32,
        /// Alpha channel. [0.0, 1.0]
        alpha: f32,
    },
    /// LCH(ab) (lightness, chroma, hue) color with an alpha channel
    Lcha {
        /// Lightness channel. [0.0, 1.5]
        lightness: f32,
        /// Chroma channel. [0.0, 1.5]
        chroma: f32,
        /// Hue channel. [0.0, 360.0]
        hue: f32,
        /// Alpha channel. [0.0, 1.0]
        alpha: f32,
    },
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
    ///
    /// See also [`Color::rgba`], [`Color::rgb_u8`], [`Color::hex`].
    ///
    pub const fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color::Rgba {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
    }

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
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color::Rgba {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
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
    pub const fn rgb_linear(r: f32, g: f32, b: f32) -> Color {
        Color::RgbaLinear {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
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
    pub const fn rgba_linear(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color::RgbaLinear {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
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
    pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
        Color::Hsla {
            hue,
            saturation,
            lightness,
            alpha: 1.0,
        }
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
    pub const fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Color {
        Color::Hsla {
            hue,
            saturation,
            lightness,
            alpha,
        }
    }

    /// New `Color` with LCH representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    ///
    /// See also [`Color::lcha`].
    pub const fn lch(lightness: f32, chroma: f32, hue: f32) -> Color {
        Color::Lcha {
            lightness,
            chroma,
            hue,
            alpha: 1.0,
        }
    }

    /// New `Color` with LCH representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::lch`].
    pub const fn lcha(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Color {
        Color::Lcha {
            lightness,
            chroma,
            hue,
            alpha,
        }
    }

    /// New `Color` from sRGB colorspace.
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
    ///
    pub fn hex<T: AsRef<str>>(hex: T) -> Result<Color, HexColorError> {
        let hex = hex.as_ref();
        let hex = hex.strip_prefix('#').unwrap_or(hex);

        match *hex.as_bytes() {
            // RGB
            [r, g, b] => {
                let [r, g, b, ..] = decode_hex([r, r, g, g, b, b])?;
                Ok(Color::rgb_u8(r, g, b))
            }
            // RGBA
            [r, g, b, a] => {
                let [r, g, b, a, ..] = decode_hex([r, r, g, g, b, b, a, a])?;
                Ok(Color::rgba_u8(r, g, b, a))
            }
            // RRGGBB
            [r1, r2, g1, g2, b1, b2] => {
                let [r, g, b, ..] = decode_hex([r1, r2, g1, g2, b1, b2])?;
                Ok(Color::rgb_u8(r, g, b))
            }
            // RRGGBBAA
            [r1, r2, g1, g2, b1, b2, a1, a2] => {
                let [r, g, b, a, ..] = decode_hex([r1, r2, g1, g2, b1, b2, a1, a2])?;
                Ok(Color::rgba_u8(r, g, b, a))
            }
            _ => Err(HexColorError::Length),
        }
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
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Color {
        Color::rgba_u8(r, g, b, u8::MAX)
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
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
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::rgba(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    /// Converts a Color to variant [`Color::Rgba`] and return red in sRGB colorspace
    pub fn r(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { red, .. } => red,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Rgba`] and return green in sRGB colorspace
    pub fn g(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { green, .. } => green,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Rgba`] and return blue in sRGB colorspace
    pub fn b(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { blue, .. } => blue,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Rgba`] and set red
    pub fn set_r(&mut self, r: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { red, .. } => *red = r,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and return this color with red set to a new value
    #[must_use]
    pub fn with_r(mut self, r: f32) -> Self {
        self.set_r(r);
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and set green
    pub fn set_g(&mut self, g: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { green, .. } => *green = g,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and return this color with green set to a new value
    #[must_use]
    pub fn with_g(mut self, g: f32) -> Self {
        self.set_g(g);
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and set blue
    pub fn set_b(&mut self, b: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { blue, .. } => *blue = b,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and return this color with blue set to a new value
    #[must_use]
    pub fn with_b(mut self, b: f32) -> Self {
        self.set_b(b);
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return hue
    pub fn h(&self) -> f32 {
        match self.as_hsla() {
            Color::Hsla { hue, .. } => hue,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Hsla`] and return saturation
    pub fn s(&self) -> f32 {
        match self.as_hsla() {
            Color::Hsla { saturation, .. } => saturation,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Hsla`] and return lightness
    pub fn l(&self) -> f32 {
        match self.as_hsla() {
            Color::Hsla { lightness, .. } => lightness,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Hsla`] and set hue
    pub fn set_h(&mut self, h: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            Color::Hsla { hue, .. } => *hue = h,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return this color with hue set to a new value
    #[must_use]
    pub fn with_h(mut self, h: f32) -> Self {
        self.set_h(h);
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and set saturation
    pub fn set_s(&mut self, s: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            Color::Hsla { saturation, .. } => *saturation = s,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return this color with saturation set to a new value
    #[must_use]
    pub fn with_s(mut self, s: f32) -> Self {
        self.set_s(s);
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and set lightness
    pub fn set_l(&mut self, l: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            Color::Hsla { lightness, .. } => *lightness = l,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return this color with lightness set to a new value
    #[must_use]
    pub fn with_l(mut self, l: f32) -> Self {
        self.set_l(l);
        self
    }

    /// Get alpha.
    #[inline(always)]
    pub fn a(&self) -> f32 {
        match self {
            Color::Rgba { alpha, .. }
            | Color::RgbaLinear { alpha, .. }
            | Color::Hsla { alpha, .. }
            | Color::Lcha { alpha, .. } => *alpha,
        }
    }

    /// Set alpha.
    pub fn set_a(&mut self, a: f32) -> &mut Self {
        match self {
            Color::Rgba { alpha, .. }
            | Color::RgbaLinear { alpha, .. }
            | Color::Hsla { alpha, .. }
            | Color::Lcha { alpha, .. } => {
                *alpha = a;
            }
        }
        self
    }

    /// Returns this color with a new alpha value.
    #[must_use]
    pub fn with_a(mut self, a: f32) -> Self {
        self.set_a(a);
        self
    }

    /// Converts a `Color` to variant `Color::Rgba`
    pub fn as_rgba(self: &Color) -> Color {
        match self {
            Color::Rgba { .. } => *self,
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red.linear_to_nonlinear_srgb(),
                green: green.linear_to_nonlinear_srgb(),
                blue: blue.linear_to_nonlinear_srgb(),
                alpha: *alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(*hue, *saturation, *lightness);
                Color::Rgba {
                    red,
                    green,
                    blue,
                    alpha: *alpha,
                }
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(*lightness, *chroma, *hue);

                Color::Rgba {
                    red,
                    green,
                    blue,
                    alpha: *alpha,
                }
            }
        }
    }

    /// Converts a `Color` to variant `Color::RgbaLinear`
    pub fn as_rgba_linear(self: &Color) -> Color {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red.nonlinear_to_linear_srgb(),
                green: green.nonlinear_to_linear_srgb(),
                blue: blue.nonlinear_to_linear_srgb(),
                alpha: *alpha,
            },
            Color::RgbaLinear { .. } => *self,
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(*hue, *saturation, *lightness);
                Color::RgbaLinear {
                    red: red.nonlinear_to_linear_srgb(),
                    green: green.nonlinear_to_linear_srgb(),
                    blue: blue.nonlinear_to_linear_srgb(),
                    alpha: *alpha,
                }
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(*lightness, *chroma, *hue);

                Color::RgbaLinear {
                    red: red.nonlinear_to_linear_srgb(),
                    green: green.nonlinear_to_linear_srgb(),
                    blue: blue.nonlinear_to_linear_srgb(),
                    alpha: *alpha,
                }
            }
        }
    }

    /// Converts a `Color` to variant `Color::Hsla`
    pub fn as_hsla(self: &Color) -> Color {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) =
                    HslRepresentation::nonlinear_srgb_to_hsl([*red, *green, *blue]);
                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha: *alpha,
                }
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha: *alpha,
                }
            }
            Color::Hsla { .. } => *self,
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rgb = LchRepresentation::lch_to_nonlinear_srgb(*lightness, *chroma, *hue);
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl(rgb);

                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha: *alpha,
                }
            }
        }
    }

    /// Converts a `Color` to variant `Color::Lcha`
    pub fn as_lcha(self: &Color) -> Color {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) =
                    LchRepresentation::nonlinear_srgb_to_lch([*red, *green, *blue]);
                Color::Lcha {
                    lightness,
                    chroma,
                    hue,
                    alpha: *alpha,
                }
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                Color::Lcha {
                    lightness,
                    chroma,
                    hue,
                    alpha: *alpha,
                }
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rgb = HslRepresentation::hsl_to_nonlinear_srgb(*hue, *saturation, *lightness);
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch(rgb);
                Color::Lcha {
                    lightness,
                    chroma,
                    hue,
                    alpha: *alpha,
                }
            }
            Color::Lcha { .. } => *self,
        }
    }

    /// Converts a `Color` to a `[u8; 4]` from sRGB colorspace
    pub fn as_rgba_u8(&self) -> [u8; 4] {
        let [r, g, b, a] = self.as_rgba_f32();
        [
            (r * u8::MAX as f32) as u8,
            (g * u8::MAX as f32) as u8,
            (b * u8::MAX as f32) as u8,
            (a * u8::MAX as f32) as u8,
        ]
    }

    /// Converts a `Color` to a `[f32; 4]` from sRGB colorspace
    pub fn as_rgba_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => [red, green, blue, alpha],
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => [
                red.linear_to_nonlinear_srgb(),
                green.linear_to_nonlinear_srgb(),
                blue.linear_to_nonlinear_srgb(),
                alpha,
            ],
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                [red, green, blue, alpha]
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

                [red, green, blue, alpha]
            }
        }
    }

    /// Converts a `Color` to a `[f32; 4]` from linear RGB colorspace
    #[inline]
    pub fn as_linear_rgba_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => [
                red.nonlinear_to_linear_srgb(),
                green.nonlinear_to_linear_srgb(),
                blue.nonlinear_to_linear_srgb(),
                alpha,
            ],
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => [red, green, blue, alpha],
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                [
                    red.nonlinear_to_linear_srgb(),
                    green.nonlinear_to_linear_srgb(),
                    blue.nonlinear_to_linear_srgb(),
                    alpha,
                ]
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

                [
                    red.nonlinear_to_linear_srgb(),
                    green.nonlinear_to_linear_srgb(),
                    blue.nonlinear_to_linear_srgb(),
                    alpha,
                ]
            }
        }
    }

    /// Converts a `Color` to a `[f32; 4]` from HSL colorspace
    pub fn as_hsla_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) =
                    HslRepresentation::nonlinear_srgb_to_hsl([red, green, blue]);
                [hue, saturation, lightness, alpha]
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                [hue, saturation, lightness, alpha]
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => [hue, saturation, lightness, alpha],
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rgb = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl(rgb);

                [hue, saturation, lightness, alpha]
            }
        }
    }

    /// Converts a `Color` to a `[f32; 4]` from LCH colorspace
    pub fn as_lcha_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) =
                    LchRepresentation::nonlinear_srgb_to_lch([red, green, blue]);
                [lightness, chroma, hue, alpha]
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                [lightness, chroma, hue, alpha]
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rgb = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch(rgb);

                [lightness, chroma, hue, alpha]
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => [lightness, chroma, hue, alpha],
        }
    }

    /// Converts `Color` to a `u32` from sRGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    pub fn as_rgba_u32(self: Color) -> u32 {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red * 255.0) as u8,
                (green * 255.0) as u8,
                (blue * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red.linear_to_nonlinear_srgb() * 255.0) as u8,
                (green.linear_to_nonlinear_srgb() * 255.0) as u8,
                (blue.linear_to_nonlinear_srgb() * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                u32::from_le_bytes([
                    (red * 255.0) as u8,
                    (green * 255.0) as u8,
                    (blue * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ])
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

                u32::from_le_bytes([
                    (red * 255.0) as u8,
                    (green * 255.0) as u8,
                    (blue * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ])
            }
        }
    }

    /// Converts Color to a u32 from linear RGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    pub fn as_linear_rgba_u32(self: Color) -> u32 {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red.nonlinear_to_linear_srgb() * 255.0) as u8,
                (green.nonlinear_to_linear_srgb() * 255.0) as u8,
                (blue.nonlinear_to_linear_srgb() * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red * 255.0) as u8,
                (green * 255.0) as u8,
                (blue * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                u32::from_le_bytes([
                    (red.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (green.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (blue.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ])
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

                u32::from_le_bytes([
                    (red.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (green.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (blue.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ])
            }
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

impl AddAssign<Color> for Color {
    fn add_assign(&mut self, rhs: Color) {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_rgba_f32();
                *red += rhs[0];
                *green += rhs[1];
                *blue += rhs[2];
                *alpha += rhs[3];
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_linear_rgba_f32();
                *red += rhs[0];
                *green += rhs[1];
                *blue += rhs[2];
                *alpha += rhs[3];
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rhs = rhs.as_hsla_f32();
                *hue += rhs[0];
                *saturation += rhs[1];
                *lightness += rhs[2];
                *alpha += rhs[3];
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rhs = rhs.as_lcha_f32();
                *lightness += rhs[0];
                *chroma += rhs[1];
                *hue += rhs[2];
                *alpha += rhs[3];
            }
        }
    }
}

impl Add<Color> for Color {
    type Output = Color;

    fn add(self, rhs: Color) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_rgba_f32();
                Color::Rgba {
                    red: red + rhs[0],
                    green: green + rhs[1],
                    blue: blue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_linear_rgba_f32();
                Color::RgbaLinear {
                    red: red + rhs[0],
                    green: green + rhs[1],
                    blue: blue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rhs = rhs.as_hsla_f32();
                Color::Hsla {
                    hue: hue + rhs[0],
                    saturation: saturation + rhs[1],
                    lightness: lightness + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rhs = rhs.as_lcha_f32();

                Color::Lcha {
                    lightness: lightness + rhs[0],
                    chroma: chroma + rhs[1],
                    hue: hue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
        }
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

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        color.as_rgba_f32()
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

impl From<Color> for Vec4 {
    fn from(color: Color) -> Self {
        let color: [f32; 4] = color.into();
        Vec4::new(color[0], color[1], color[2], color[3])
    }
}

impl From<Vec4> for Color {
    fn from(vec4: Vec4) -> Self {
        Color::rgba(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        if let Color::RgbaLinear {
            red,
            green,
            blue,
            alpha,
        } = color.as_rgba_linear()
        {
            wgpu::Color {
                r: red as f64,
                g: green as f64,
                b: blue as f64,
                a: alpha as f64,
            }
        } else {
            unreachable!()
        }
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs,
                green: green * rhs,
                blue: blue * rhs,
                alpha,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs,
                green: green * rhs,
                blue: blue * rhs,
                alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs,
                saturation: saturation * rhs,
                lightness: lightness * rhs,
                alpha,
            },
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs,
                chroma: chroma * rhs,
                hue: hue * rhs,
                alpha,
            },
        }
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            Color::Rgba {
                red, green, blue, ..
            }
            | Color::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs;
                *green *= rhs;
                *blue *= rhs;
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs;
                *saturation *= rhs;
                *lightness *= rhs;
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => {
                *lightness *= rhs;
                *chroma *= rhs;
                *hue *= rhs;
            }
        }
    }
}

impl Mul<Vec4> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec4) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha: alpha * rhs.w,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha: alpha * rhs.w,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs.x,
                saturation: saturation * rhs.y,
                lightness: lightness * rhs.z,
                alpha: alpha * rhs.w,
            },
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs.x,
                chroma: chroma * rhs.y,
                hue: hue * rhs.z,
                alpha: alpha * rhs.w,
            },
        }
    }
}

impl MulAssign<Vec4> for Color {
    fn mul_assign(&mut self, rhs: Vec4) {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            }
            | Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                *red *= rhs.x;
                *green *= rhs.y;
                *blue *= rhs.z;
                *alpha *= rhs.w;
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                *hue *= rhs.x;
                *saturation *= rhs.y;
                *lightness *= rhs.z;
                *alpha *= rhs.w;
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                *lightness *= rhs.x;
                *chroma *= rhs.y;
                *hue *= rhs.z;
                *alpha *= rhs.w;
            }
        }
    }
}

impl Mul<Vec3> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec3) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs.x,
                saturation: saturation * rhs.y,
                lightness: lightness * rhs.z,
                alpha,
            },
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs.x,
                chroma: chroma * rhs.y,
                hue: hue * rhs.z,
                alpha,
            },
        }
    }
}

impl MulAssign<Vec3> for Color {
    fn mul_assign(&mut self, rhs: Vec3) {
        match self {
            Color::Rgba {
                red, green, blue, ..
            }
            | Color::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs.x;
                *green *= rhs.y;
                *blue *= rhs.z;
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs.x;
                *saturation *= rhs.y;
                *lightness *= rhs.z;
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => {
                *lightness *= rhs.x;
                *chroma *= rhs.y;
                *hue *= rhs.z;
            }
        }
    }
}

impl Mul<[f32; 4]> for Color {
    type Output = Color;

    fn mul(self, rhs: [f32; 4]) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha: alpha * rhs[3],
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha: alpha * rhs[3],
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs[0],
                saturation: saturation * rhs[1],
                lightness: lightness * rhs[2],
                alpha: alpha * rhs[3],
            },
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs[0],
                chroma: chroma * rhs[1],
                hue: hue * rhs[2],
                alpha: alpha * rhs[3],
            },
        }
    }
}

impl MulAssign<[f32; 4]> for Color {
    fn mul_assign(&mut self, rhs: [f32; 4]) {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            }
            | Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                *red *= rhs[0];
                *green *= rhs[1];
                *blue *= rhs[2];
                *alpha *= rhs[3];
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                *hue *= rhs[0];
                *saturation *= rhs[1];
                *lightness *= rhs[2];
                *alpha *= rhs[3];
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                *lightness *= rhs[0];
                *chroma *= rhs[1];
                *hue *= rhs[2];
                *alpha *= rhs[3];
            }
        }
    }
}

impl Mul<[f32; 3]> for Color {
    type Output = Color;

    fn mul(self, rhs: [f32; 3]) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs[0],
                saturation: saturation * rhs[1],
                lightness: lightness * rhs[2],
                alpha,
            },
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs[0],
                chroma: chroma * rhs[1],
                hue: hue * rhs[2],
                alpha,
            },
        }
    }
}

impl MulAssign<[f32; 3]> for Color {
    fn mul_assign(&mut self, rhs: [f32; 3]) {
        match self {
            Color::Rgba {
                red, green, blue, ..
            }
            | Color::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs[0];
                *green *= rhs[1];
                *blue *= rhs[2];
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs[0];
                *saturation *= rhs[1];
                *lightness *= rhs[2];
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => {
                *lightness *= rhs[0];
                *chroma *= rhs[1];
                *hue *= rhs[2];
            }
        }
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
        let linear = self.as_linear_rgba_f32();
        for el in &linear {
            encase::private::WriteInto::write_into(el, writer);
        }
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

        *self = Color::RgbaLinear {
            red: buffer[0],
            green: buffer[1],
            blue: buffer[2],
            alpha: buffer[3],
        }
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
        Color::RgbaLinear {
            red,
            green,
            blue,
            alpha,
        }
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

        assert_eq!(starting_vec4, Vec4::from(starting_color));

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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
    }

    // regression test for https://github.com/bevyengine/bevy/pull/8040
    #[test]
    fn convert_to_rgba_linear() {
        let rgba = Color::rgba(0., 0., 0., 0.);
        let rgba_l = Color::rgba_linear(0., 0., 0., 0.);
        let hsla = Color::hsla(0., 0., 0., 0.);
        let lcha = Color::lcha(0., 0., 0., 0.);
        assert_eq!(rgba_l, rgba_l.as_rgba_linear());
        let Color::RgbaLinear { .. } = rgba.as_rgba_linear() else {
            panic!("from Rgba")
        };
        let Color::RgbaLinear { .. } = hsla.as_rgba_linear() else {
            panic!("from Hsla")
        };
        let Color::RgbaLinear { .. } = lcha.as_rgba_linear() else {
            panic!("from Lcha")
        };
    }
}
