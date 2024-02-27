use bevy_color::{
    Color, HexColorError, Hsla, Hsva, Hwba, Laba, Lcha, LinearRgba, Oklaba, Srgba, Xyza,
};

use bevy_math::{Vec3, Vec4};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, MulAssign};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum LegacyColor {
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

impl LegacyColor {
    /// <div style="background-color:rgb(94%, 97%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ALICE_BLUE: LegacyColor = LegacyColor::rgb(0.94, 0.97, 1.0);
    /// <div style="background-color:rgb(98%, 92%, 84%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ANTIQUE_WHITE: LegacyColor = LegacyColor::rgb(0.98, 0.92, 0.84);
    /// <div style="background-color:rgb(49%, 100%, 83%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AQUAMARINE: LegacyColor = LegacyColor::rgb(0.49, 1.0, 0.83);
    /// <div style="background-color:rgb(94%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AZURE: LegacyColor = LegacyColor::rgb(0.94, 1.0, 1.0);
    /// <div style="background-color:rgb(96%, 96%, 86%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BEIGE: LegacyColor = LegacyColor::rgb(0.96, 0.96, 0.86);
    /// <div style="background-color:rgb(100%, 89%, 77%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BISQUE: LegacyColor = LegacyColor::rgb(1.0, 0.89, 0.77);
    /// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: LegacyColor = LegacyColor::rgb(0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(0%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE: LegacyColor = LegacyColor::rgb(0.0, 0.0, 1.0);
    /// <div style="background-color:rgb(86%, 8%, 24%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CRIMSON: LegacyColor = LegacyColor::rgb(0.86, 0.08, 0.24);
    /// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CYAN: LegacyColor = LegacyColor::rgb(0.0, 1.0, 1.0);
    /// <div style="background-color:rgb(25%, 25%, 25%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GRAY: LegacyColor = LegacyColor::rgb(0.25, 0.25, 0.25);
    /// <div style="background-color:rgb(0%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GREEN: LegacyColor = LegacyColor::rgb(0.0, 0.5, 0.0);
    /// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const FUCHSIA: LegacyColor = LegacyColor::rgb(1.0, 0.0, 1.0);
    /// <div style="background-color:rgb(100%, 84%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD: LegacyColor = LegacyColor::rgb(1.0, 0.84, 0.0);
    /// <div style="background-color:rgb(50%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GRAY: LegacyColor = LegacyColor::rgb(0.5, 0.5, 0.5);
    /// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN: LegacyColor = LegacyColor::rgb(0.0, 1.0, 0.0);
    /// <div style="background-color:rgb(28%, 0%, 51%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const INDIGO: LegacyColor = LegacyColor::rgb(0.29, 0.0, 0.51);
    /// <div style="background-color:rgb(20%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIME_GREEN: LegacyColor = LegacyColor::rgb(0.2, 0.8, 0.2);
    /// <div style="background-color:rgb(50%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON: LegacyColor = LegacyColor::rgb(0.5, 0.0, 0.0);
    /// <div style="background-color:rgb(10%, 10%, 44%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MIDNIGHT_BLUE: LegacyColor = LegacyColor::rgb(0.1, 0.1, 0.44);
    /// <div style="background-color:rgb(0%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const NAVY: LegacyColor = LegacyColor::rgb(0.0, 0.0, 0.5);
    /// <div style="background-color:rgba(0%, 0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    #[doc(alias = "transparent")]
    pub const NONE: LegacyColor = LegacyColor::rgba(0.0, 0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(50%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const OLIVE: LegacyColor = LegacyColor::rgb(0.5, 0.5, 0.0);
    /// <div style="background-color:rgb(100%, 65%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE: LegacyColor = LegacyColor::rgb(1.0, 0.65, 0.0);
    /// <div style="background-color:rgb(100%, 27%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE_RED: LegacyColor = LegacyColor::rgb(1.0, 0.27, 0.0);
    /// <div style="background-color:rgb(100%, 8%, 57%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PINK: LegacyColor = LegacyColor::rgb(1.0, 0.08, 0.58);
    /// <div style="background-color:rgb(50%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE: LegacyColor = LegacyColor::rgb(0.5, 0.0, 0.5);
    /// <div style="background-color:rgb(100%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED: LegacyColor = LegacyColor::rgb(1.0, 0.0, 0.0);
    /// <div style="background-color:rgb(98%, 50%, 45%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SALMON: LegacyColor = LegacyColor::rgb(0.98, 0.5, 0.45);
    /// <div style="background-color:rgb(18%, 55%, 34%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SEA_GREEN: LegacyColor = LegacyColor::rgb(0.18, 0.55, 0.34);
    /// <div style="background-color:rgb(75%, 75%, 75%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SILVER: LegacyColor = LegacyColor::rgb(0.75, 0.75, 0.75);
    /// <div style="background-color:rgb(0%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL: LegacyColor = LegacyColor::rgb(0.0, 0.5, 0.5);
    /// <div style="background-color:rgb(100%, 39%, 28%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TOMATO: LegacyColor = LegacyColor::rgb(1.0, 0.39, 0.28);
    /// <div style="background-color:rgb(25%, 88%, 82%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TURQUOISE: LegacyColor = LegacyColor::rgb(0.25, 0.88, 0.82);
    /// <div style="background-color:rgb(93%, 51%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const VIOLET: LegacyColor = LegacyColor::rgb(0.93, 0.51, 0.93);
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: LegacyColor = LegacyColor::rgb(1.0, 1.0, 1.0);
    /// <div style="background-color:rgb(100%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW: LegacyColor = LegacyColor::rgb(1.0, 1.0, 0.0);
    /// <div style="background-color:rgb(60%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_GREEN: LegacyColor = LegacyColor::rgb(0.6, 0.8, 0.2);

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    ///
    /// See also [`LegacyColor::rgba`], [`LegacyColor::rgb_u8`], [`LegacyColor::hex`].
    ///
    pub const fn rgb(r: f32, g: f32, b: f32) -> LegacyColor {
        LegacyColor::Rgba {
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
    /// See also [`LegacyColor::rgb`], [`LegacyColor::rgba_u8`], [`LegacyColor::hex`].
    ///
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> LegacyColor {
        LegacyColor::Rgba {
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
    /// See also [`LegacyColor::rgb`], [`LegacyColor::rgba_linear`].
    ///
    pub const fn rgb_linear(r: f32, g: f32, b: f32) -> LegacyColor {
        LegacyColor::RgbaLinear {
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
    /// See also [`LegacyColor::rgba`], [`LegacyColor::rgb_linear`].
    ///
    pub const fn rgba_linear(r: f32, g: f32, b: f32, a: f32) -> LegacyColor {
        LegacyColor::RgbaLinear {
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
    /// See also [`LegacyColor::hsla`].
    ///
    pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> LegacyColor {
        LegacyColor::Hsla {
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
    /// See also [`LegacyColor::hsl`].
    ///
    pub const fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> LegacyColor {
        LegacyColor::Hsla {
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
    /// See also [`LegacyColor::lcha`].
    pub const fn lch(lightness: f32, chroma: f32, hue: f32) -> LegacyColor {
        LegacyColor::Lcha {
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
    /// See also [`LegacyColor::lch`].
    pub const fn lcha(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> LegacyColor {
        LegacyColor::Lcha {
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
    /// # use bevy_render::color::LegacyColor;
    /// let color = LegacyColor::hex("FF00FF").unwrap(); // fuchsia
    /// let color = LegacyColor::hex("FF00FF7F").unwrap(); // partially transparent fuchsia
    ///
    /// // A standard hex color notation is also available
    /// assert_eq!(LegacyColor::hex("#FFFFFF").unwrap(), LegacyColor::rgb(1.0, 1.0, 1.0));
    /// ```
    ///
    pub fn hex<T: AsRef<str>>(hex: T) -> Result<LegacyColor, HexColorError> {
        Srgba::hex(hex).map(|color| color.into())
    }

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0, 255]
    /// * `g` - Green channel. [0, 255]
    /// * `b` - Blue channel. [0, 255]
    ///
    /// See also [`LegacyColor::rgb`], [`LegacyColor::rgba_u8`], [`LegacyColor::hex`].
    ///
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> LegacyColor {
        LegacyColor::rgba_u8(r, g, b, u8::MAX)
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
    /// See also [`LegacyColor::rgba`], [`LegacyColor::rgb_u8`], [`LegacyColor::hex`].
    ///
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> LegacyColor {
        LegacyColor::rgba(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and return red in sRGB colorspace
    pub fn r(&self) -> f32 {
        match self.as_rgba() {
            LegacyColor::Rgba { red, .. } => red,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and return green in sRGB colorspace
    pub fn g(&self) -> f32 {
        match self.as_rgba() {
            LegacyColor::Rgba { green, .. } => green,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and return blue in sRGB colorspace
    pub fn b(&self) -> f32 {
        match self.as_rgba() {
            LegacyColor::Rgba { blue, .. } => blue,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and set red
    pub fn set_r(&mut self, r: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            LegacyColor::Rgba { red, .. } => *red = r,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and return this color with red set to a new value
    #[must_use]
    pub fn with_r(mut self, r: f32) -> Self {
        self.set_r(r);
        self
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and set green
    pub fn set_g(&mut self, g: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            LegacyColor::Rgba { green, .. } => *green = g,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and return this color with green set to a new value
    #[must_use]
    pub fn with_g(mut self, g: f32) -> Self {
        self.set_g(g);
        self
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and set blue
    pub fn set_b(&mut self, b: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            LegacyColor::Rgba { blue, .. } => *blue = b,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`LegacyColor::Rgba`] and return this color with blue set to a new value
    #[must_use]
    pub fn with_b(mut self, b: f32) -> Self {
        self.set_b(b);
        self
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and return hue
    pub fn h(&self) -> f32 {
        match self.as_hsla() {
            LegacyColor::Hsla { hue, .. } => hue,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and return saturation
    pub fn s(&self) -> f32 {
        match self.as_hsla() {
            LegacyColor::Hsla { saturation, .. } => saturation,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and return lightness
    pub fn l(&self) -> f32 {
        match self.as_hsla() {
            LegacyColor::Hsla { lightness, .. } => lightness,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and set hue
    pub fn set_h(&mut self, h: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            LegacyColor::Hsla { hue, .. } => *hue = h,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and return this color with hue set to a new value
    #[must_use]
    pub fn with_h(mut self, h: f32) -> Self {
        self.set_h(h);
        self
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and set saturation
    pub fn set_s(&mut self, s: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            LegacyColor::Hsla { saturation, .. } => *saturation = s,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and return this color with saturation set to a new value
    #[must_use]
    pub fn with_s(mut self, s: f32) -> Self {
        self.set_s(s);
        self
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and set lightness
    pub fn set_l(&mut self, l: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            LegacyColor::Hsla { lightness, .. } => *lightness = l,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`LegacyColor::Hsla`] and return this color with lightness set to a new value
    #[must_use]
    pub fn with_l(mut self, l: f32) -> Self {
        self.set_l(l);
        self
    }

    /// Get alpha.
    #[inline(always)]
    pub fn a(&self) -> f32 {
        match self {
            LegacyColor::Rgba { alpha, .. }
            | LegacyColor::RgbaLinear { alpha, .. }
            | LegacyColor::Hsla { alpha, .. }
            | LegacyColor::Lcha { alpha, .. } => *alpha,
        }
    }

    /// Set alpha.
    pub fn set_a(&mut self, a: f32) -> &mut Self {
        match self {
            LegacyColor::Rgba { alpha, .. }
            | LegacyColor::RgbaLinear { alpha, .. }
            | LegacyColor::Hsla { alpha, .. }
            | LegacyColor::Lcha { alpha, .. } => {
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

    /// Determine if the color is fully transparent, i.e. if the alpha is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_render::color::LegacyColor;
    /// // Fully transparent colors
    /// assert!(LegacyColor::NONE.is_fully_transparent());
    /// assert!(LegacyColor::rgba(1.0, 0.5, 0.5, 0.0).is_fully_transparent());
    ///
    /// // (Partially) opaque colors
    /// assert!(!LegacyColor::BLACK.is_fully_transparent());
    /// assert!(!LegacyColor::rgba(1.0, 0.5, 0.5, 0.2).is_fully_transparent());
    /// ```
    #[inline(always)]
    pub fn is_fully_transparent(&self) -> bool {
        self.a() == 0.0
    }

    /// Converts a `Color` to variant `LegacyColor::Rgba`
    pub fn as_rgba(self: &LegacyColor) -> LegacyColor {
        Srgba::from(*self).into()
    }

    /// Converts a `Color` to variant `LegacyColor::RgbaLinear`
    pub fn as_rgba_linear(self: &LegacyColor) -> LegacyColor {
        LinearRgba::from(*self).into()
    }

    /// Converts a `Color` to variant `LegacyColor::Hsla`
    pub fn as_hsla(self: &LegacyColor) -> LegacyColor {
        Hsla::from(*self).into()
    }

    /// Converts a `Color` to variant `LegacyColor::Lcha`
    pub fn as_lcha(self: &LegacyColor) -> LegacyColor {
        Lcha::from(*self).into()
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
    pub fn as_rgba_f32(self: LegacyColor) -> [f32; 4] {
        let Srgba {
            red,
            green,
            blue,
            alpha,
        } = Srgba::from(self);
        [red, green, blue, alpha]
    }

    /// Converts a `Color` to a `[f32; 4]` from linear RGB colorspace
    #[inline]
    pub fn as_linear_rgba_f32(self: LegacyColor) -> [f32; 4] {
        let LinearRgba {
            red,
            green,
            blue,
            alpha,
        } = LinearRgba::from(self);
        [red, green, blue, alpha]
    }

    /// Converts a `Color` to a `[f32; 4]` from HSL colorspace
    pub fn as_hsla_f32(self: LegacyColor) -> [f32; 4] {
        let Hsla {
            hue,
            saturation,
            lightness,
            alpha,
        } = Hsla::from(self);
        [hue, saturation, lightness, alpha]
    }

    /// Converts a `Color` to a `[f32; 4]` from LCH colorspace
    pub fn as_lcha_f32(self: LegacyColor) -> [f32; 4] {
        let Lcha {
            lightness,
            chroma,
            hue,
            alpha,
        } = Lcha::from(self);
        [lightness, chroma, hue, alpha]
    }

    /// Converts `Color` to a `u32` from sRGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    pub fn as_rgba_u32(self: LegacyColor) -> u32 {
        u32::from_le_bytes(self.as_rgba_u8())
    }

    /// Converts Color to a u32 from linear RGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    pub fn as_linear_rgba_u32(self: LegacyColor) -> u32 {
        let LinearRgba {
            red,
            green,
            blue,
            alpha,
        } = self.into();
        u32::from_le_bytes([
            (red * 255.0) as u8,
            (green * 255.0) as u8,
            (blue * 255.0) as u8,
            (alpha * 255.0) as u8,
        ])
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with RGB representation in sRGB colorspace.
    #[inline]
    pub fn rgba_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [r, g, b, a]: [f32; 4] = arr.into();
        LegacyColor::rgba(r, g, b, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with RGB representation in sRGB colorspace.
    #[inline]
    pub fn rgb_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [r, g, b]: [f32; 3] = arr.into();
        LegacyColor::rgb(r, g, b)
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with RGB representation in linear RGB colorspace.
    #[inline]
    pub fn rgba_linear_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [r, g, b, a]: [f32; 4] = arr.into();
        LegacyColor::rgba_linear(r, g, b, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with RGB representation in linear RGB colorspace.
    #[inline]
    pub fn rgb_linear_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [r, g, b]: [f32; 3] = arr.into();
        LegacyColor::rgb_linear(r, g, b)
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with HSL representation in sRGB colorspace.
    #[inline]
    pub fn hsla_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [h, s, l, a]: [f32; 4] = arr.into();
        LegacyColor::hsla(h, s, l, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with HSL representation in sRGB colorspace.
    #[inline]
    pub fn hsl_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [h, s, l]: [f32; 3] = arr.into();
        LegacyColor::hsl(h, s, l)
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with LCH representation in sRGB colorspace.
    #[inline]
    pub fn lcha_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [l, c, h, a]: [f32; 4] = arr.into();
        LegacyColor::lcha(l, c, h, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with LCH representation in sRGB colorspace.
    #[inline]
    pub fn lch_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [l, c, h]: [f32; 3] = arr.into();
        LegacyColor::lch(l, c, h)
    }

    /// Convert `Color` to RGBA and return as `Vec4`.
    #[inline]
    pub fn rgba_to_vec4(&self) -> Vec4 {
        let color = self.as_rgba();
        match color {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Vec4::new(red, green, blue, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to RGBA and return as `Vec3`.
    #[inline]
    pub fn rgb_to_vec3(&self) -> Vec3 {
        let color = self.as_rgba();
        match color {
            LegacyColor::Rgba {
                red, green, blue, ..
            } => Vec3::new(red, green, blue),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to linear RGBA and return as `Vec4`.
    #[inline]
    pub fn rgba_linear_to_vec4(&self) -> Vec4 {
        let color = self.as_rgba_linear();
        match color {
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Vec4::new(red, green, blue, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to linear RGBA and return as `Vec3`.
    #[inline]
    pub fn rgb_linear_to_vec3(&self) -> Vec3 {
        let color = self.as_rgba_linear();
        match color {
            LegacyColor::RgbaLinear {
                red, green, blue, ..
            } => Vec3::new(red, green, blue),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to HSLA and return as `Vec4`.
    #[inline]
    pub fn hsla_to_vec4(&self) -> Vec4 {
        let color = self.as_hsla();
        match color {
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Vec4::new(hue, saturation, lightness, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to HSLA and return as `Vec3`.
    #[inline]
    pub fn hsl_to_vec3(&self) -> Vec3 {
        let color = self.as_hsla();
        match color {
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => Vec3::new(hue, saturation, lightness),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to LCHA and return as `Vec4`.
    #[inline]
    pub fn lcha_to_vec4(&self) -> Vec4 {
        let color = self.as_lcha();
        match color {
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Vec4::new(lightness, chroma, hue, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to LCHA and return as `Vec3`.
    #[inline]
    pub fn lch_to_vec3(&self) -> Vec3 {
        let color = self.as_lcha();
        match color {
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => Vec3::new(lightness, chroma, hue),
            _ => unreachable!(),
        }
    }
}

impl Default for LegacyColor {
    fn default() -> Self {
        LegacyColor::WHITE
    }
}

impl Add<LegacyColor> for LegacyColor {
    type Output = LegacyColor;

    fn add(self, rhs: LegacyColor) -> Self::Output {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_rgba_f32();
                LegacyColor::Rgba {
                    red: red + rhs[0],
                    green: green + rhs[1],
                    blue: blue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_linear_rgba_f32();
                LegacyColor::RgbaLinear {
                    red: red + rhs[0],
                    green: green + rhs[1],
                    blue: blue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rhs = rhs.as_hsla_f32();
                LegacyColor::Hsla {
                    hue: hue + rhs[0],
                    saturation: saturation + rhs[1],
                    lightness: lightness + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rhs = rhs.as_lcha_f32();
                LegacyColor::Lcha {
                    lightness: lightness + rhs[0],
                    chroma: chroma + rhs[1],
                    hue: hue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
        }
    }
}

impl From<LegacyColor> for Color {
    fn from(value: LegacyColor) -> Self {
        match value {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Srgba::new(red, green, blue, alpha).into(),
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => LinearRgba::new(red, green, blue, alpha).into(),
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Hsla::new(hue, saturation, lightness, alpha).into(),
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Lcha::new(lightness, chroma, hue, alpha).into(),
        }
    }
}

impl From<Color> for LegacyColor {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(x) => x.into(),
            Color::LinearRgba(x) => x.into(),
            Color::Hsla(x) => x.into(),
            Color::Hsva(x) => x.into(),
            Color::Hwba(x) => x.into(),
            Color::Laba(x) => x.into(),
            Color::Lcha(x) => x.into(),
            Color::Oklaba(x) => x.into(),
            Color::Xyza(x) => x.into(),
        }
    }
}

impl From<LinearRgba> for LegacyColor {
    fn from(
        LinearRgba {
            red,
            green,
            blue,
            alpha,
        }: LinearRgba,
    ) -> Self {
        LegacyColor::RgbaLinear {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl From<LegacyColor> for Xyza {
    fn from(value: LegacyColor) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Xyza> for LegacyColor {
    fn from(value: Xyza) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<LegacyColor> for LinearRgba {
    fn from(value: LegacyColor) -> Self {
        Color::from(value).into()
    }
}

impl From<Srgba> for LegacyColor {
    fn from(
        Srgba {
            red,
            green,
            blue,
            alpha,
        }: Srgba,
    ) -> Self {
        LegacyColor::Rgba {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl From<LegacyColor> for Srgba {
    fn from(value: LegacyColor) -> Self {
        Color::from(value).into()
    }
}

impl From<Hsla> for LegacyColor {
    fn from(value: Hsla) -> Self {
        LegacyColor::Hsla {
            hue: value.hue,
            saturation: value.saturation,
            lightness: value.lightness,
            alpha: value.alpha,
        }
    }
}

impl From<LegacyColor> for Hsla {
    fn from(value: LegacyColor) -> Self {
        Color::from(value).into()
    }
}

impl From<LegacyColor> for Hsva {
    fn from(value: LegacyColor) -> Self {
        Hsla::from(value).into()
    }
}

impl From<Hsva> for LegacyColor {
    fn from(value: Hsva) -> Self {
        Hsla::from(value).into()
    }
}

impl From<LegacyColor> for Hwba {
    fn from(value: LegacyColor) -> Self {
        Hsla::from(value).into()
    }
}

impl From<Hwba> for LegacyColor {
    fn from(value: Hwba) -> Self {
        Hsla::from(value).into()
    }
}

impl From<Laba> for LegacyColor {
    fn from(value: Laba) -> Self {
        Lcha::from(value).into()
    }
}

impl From<Lcha> for LegacyColor {
    fn from(
        Lcha {
            lightness,
            chroma,
            hue,
            alpha,
        }: Lcha,
    ) -> Self {
        LegacyColor::Lcha {
            hue,
            chroma,
            lightness,
            alpha,
        }
    }
}

impl From<LegacyColor> for Lcha {
    fn from(value: LegacyColor) -> Self {
        Color::from(value).into()
    }
}

impl From<LegacyColor> for Laba {
    fn from(value: LegacyColor) -> Self {
        Color::from(value).into()
    }
}

impl From<LegacyColor> for Oklaba {
    fn from(value: LegacyColor) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<Oklaba> for LegacyColor {
    fn from(value: Oklaba) -> Self {
        LinearRgba::from(value).into()
    }
}

impl From<LegacyColor> for wgpu::Color {
    fn from(color: LegacyColor) -> Self {
        if let LegacyColor::RgbaLinear {
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

impl Mul<f32> for LegacyColor {
    type Output = LegacyColor;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::Rgba {
                red: red * rhs,
                green: green * rhs,
                blue: blue * rhs,
                alpha,
            },
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::RgbaLinear {
                red: red * rhs,
                green: green * rhs,
                blue: blue * rhs,
                alpha,
            },
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => LegacyColor::Hsla {
                hue: hue * rhs,
                saturation: saturation * rhs,
                lightness: lightness * rhs,
                alpha,
            },
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => LegacyColor::Lcha {
                lightness: lightness * rhs,
                chroma: chroma * rhs,
                hue: hue * rhs,
                alpha,
            },
        }
    }
}

impl MulAssign<f32> for LegacyColor {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            LegacyColor::Rgba {
                red, green, blue, ..
            }
            | LegacyColor::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs;
                *green *= rhs;
                *blue *= rhs;
            }
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs;
                *saturation *= rhs;
                *lightness *= rhs;
            }
            LegacyColor::Lcha {
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

impl Mul<Vec4> for LegacyColor {
    type Output = LegacyColor;

    fn mul(self, rhs: Vec4) -> Self::Output {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::Rgba {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha: alpha * rhs.w,
            },
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::RgbaLinear {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha: alpha * rhs.w,
            },
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => LegacyColor::Hsla {
                hue: hue * rhs.x,
                saturation: saturation * rhs.y,
                lightness: lightness * rhs.z,
                alpha: alpha * rhs.w,
            },
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => LegacyColor::Lcha {
                lightness: lightness * rhs.x,
                chroma: chroma * rhs.y,
                hue: hue * rhs.z,
                alpha: alpha * rhs.w,
            },
        }
    }
}

impl MulAssign<Vec4> for LegacyColor {
    fn mul_assign(&mut self, rhs: Vec4) {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            }
            | LegacyColor::RgbaLinear {
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
            LegacyColor::Hsla {
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
            LegacyColor::Lcha {
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

impl Mul<Vec3> for LegacyColor {
    type Output = LegacyColor;

    fn mul(self, rhs: Vec3) -> Self::Output {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::Rgba {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha,
            },
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::RgbaLinear {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha,
            },
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => LegacyColor::Hsla {
                hue: hue * rhs.x,
                saturation: saturation * rhs.y,
                lightness: lightness * rhs.z,
                alpha,
            },
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => LegacyColor::Lcha {
                lightness: lightness * rhs.x,
                chroma: chroma * rhs.y,
                hue: hue * rhs.z,
                alpha,
            },
        }
    }
}

impl MulAssign<Vec3> for LegacyColor {
    fn mul_assign(&mut self, rhs: Vec3) {
        match self {
            LegacyColor::Rgba {
                red, green, blue, ..
            }
            | LegacyColor::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs.x;
                *green *= rhs.y;
                *blue *= rhs.z;
            }
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs.x;
                *saturation *= rhs.y;
                *lightness *= rhs.z;
            }
            LegacyColor::Lcha {
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

impl Mul<[f32; 4]> for LegacyColor {
    type Output = LegacyColor;

    fn mul(self, rhs: [f32; 4]) -> Self::Output {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::Rgba {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha: alpha * rhs[3],
            },
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::RgbaLinear {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha: alpha * rhs[3],
            },
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => LegacyColor::Hsla {
                hue: hue * rhs[0],
                saturation: saturation * rhs[1],
                lightness: lightness * rhs[2],
                alpha: alpha * rhs[3],
            },
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => LegacyColor::Lcha {
                lightness: lightness * rhs[0],
                chroma: chroma * rhs[1],
                hue: hue * rhs[2],
                alpha: alpha * rhs[3],
            },
        }
    }
}

impl MulAssign<[f32; 4]> for LegacyColor {
    fn mul_assign(&mut self, rhs: [f32; 4]) {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            }
            | LegacyColor::RgbaLinear {
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
            LegacyColor::Hsla {
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
            LegacyColor::Lcha {
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

impl Mul<[f32; 3]> for LegacyColor {
    type Output = LegacyColor;

    fn mul(self, rhs: [f32; 3]) -> Self::Output {
        match self {
            LegacyColor::Rgba {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::Rgba {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha,
            },
            LegacyColor::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => LegacyColor::RgbaLinear {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha,
            },
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => LegacyColor::Hsla {
                hue: hue * rhs[0],
                saturation: saturation * rhs[1],
                lightness: lightness * rhs[2],
                alpha,
            },
            LegacyColor::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => LegacyColor::Lcha {
                lightness: lightness * rhs[0],
                chroma: chroma * rhs[1],
                hue: hue * rhs[2],
                alpha,
            },
        }
    }
}

impl MulAssign<[f32; 3]> for LegacyColor {
    fn mul_assign(&mut self, rhs: [f32; 3]) {
        match self {
            LegacyColor::Rgba {
                red, green, blue, ..
            }
            | LegacyColor::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs[0];
                *green *= rhs[1];
                *blue *= rhs[2];
            }
            LegacyColor::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs[0];
                *saturation *= rhs[1];
                *lightness *= rhs[2];
            }
            LegacyColor::Lcha {
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

impl encase::ShaderType for LegacyColor {
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

impl encase::private::WriteInto for LegacyColor {
    fn write_into<B: encase::private::BufferMut>(&self, writer: &mut encase::private::Writer<B>) {
        let linear = self.as_linear_rgba_f32();
        for el in &linear {
            encase::private::WriteInto::write_into(el, writer);
        }
    }
}

impl encase::private::ReadFrom for LegacyColor {
    fn read_from<B: encase::private::BufferRef>(
        &mut self,
        reader: &mut encase::private::Reader<B>,
    ) {
        let mut buffer = [0.0f32; 4];
        for el in &mut buffer {
            encase::private::ReadFrom::read_from(el, reader);
        }

        *self = LegacyColor::RgbaLinear {
            red: buffer[0],
            green: buffer[1],
            blue: buffer[2],
            alpha: buffer[3],
        }
    }
}

impl encase::private::CreateFrom for LegacyColor {
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
        LegacyColor::RgbaLinear {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl encase::ShaderSize for LegacyColor {}

#[cfg(test)]
mod tests {
    use std::num::ParseIntError;

    use super::*;

    #[test]
    fn hex_color() {
        assert_eq!(LegacyColor::hex("FFF"), Ok(LegacyColor::WHITE));
        assert_eq!(LegacyColor::hex("FFFF"), Ok(LegacyColor::WHITE));
        assert_eq!(LegacyColor::hex("FFFFFF"), Ok(LegacyColor::WHITE));
        assert_eq!(LegacyColor::hex("FFFFFFFF"), Ok(LegacyColor::WHITE));
        assert_eq!(LegacyColor::hex("000"), Ok(LegacyColor::BLACK));
        assert_eq!(LegacyColor::hex("000F"), Ok(LegacyColor::BLACK));
        assert_eq!(LegacyColor::hex("000000"), Ok(LegacyColor::BLACK));
        assert_eq!(LegacyColor::hex("000000FF"), Ok(LegacyColor::BLACK));
        assert_eq!(
            LegacyColor::hex("03a9f4"),
            Ok(LegacyColor::rgb_u8(3, 169, 244))
        );
        assert_eq!(LegacyColor::hex("yy"), Err(HexColorError::Length));
        let Err(HexColorError::Parse(ParseIntError { .. })) = LegacyColor::hex("yyy") else {
            panic!("Expected Parse Int Error")
        };
        assert_eq!(
            LegacyColor::hex("#f2a"),
            Ok(LegacyColor::rgb_u8(255, 34, 170))
        );
        assert_eq!(
            LegacyColor::hex("#e23030"),
            Ok(LegacyColor::rgb_u8(226, 48, 48))
        );
        assert_eq!(LegacyColor::hex("#ff"), Err(HexColorError::Length));
        let Err(HexColorError::Parse(ParseIntError { .. })) = LegacyColor::hex("##fff") else {
            panic!("Expected Parse Int Error")
        };
    }

    #[test]
    fn conversions_vec4() {
        let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
        let starting_color = LegacyColor::rgba_from_array(starting_vec4);

        assert_eq!(starting_vec4, starting_color.rgba_to_vec4());

        let transformation = Vec4::new(0.5, 0.5, 0.5, 1.0);

        assert_eq!(
            starting_color * transformation,
            LegacyColor::rgba_from_array(starting_vec4 * transformation)
        );
    }

    #[test]
    fn mul_and_mulassign_f32() {
        let transformation = 0.5;
        let starting_color = LegacyColor::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            LegacyColor::rgba(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color);
    }

    #[test]
    fn mul_and_mulassign_f32by3() {
        let transformation = [0.4, 0.5, 0.6];
        let starting_color = LegacyColor::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            LegacyColor::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color);
    }

    #[test]
    fn mul_and_mulassign_f32by4() {
        let transformation = [0.4, 0.5, 0.6, 0.9];
        let starting_color = LegacyColor::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            LegacyColor::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0 * 0.9),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color);
    }

    #[test]
    fn mul_and_mulassign_vec3() {
        let transformation = Vec3::new(0.2, 0.3, 0.4);
        let starting_color = LegacyColor::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            LegacyColor::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color);
    }

    #[test]
    fn mul_and_mulassign_vec4() {
        let transformation = Vec4::new(0.2, 0.3, 0.4, 0.5);
        let starting_color = LegacyColor::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            LegacyColor::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0 * 0.5),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color);
    }

    // regression test for https://github.com/bevyengine/bevy/pull/8040
    #[test]
    fn convert_to_rgba_linear() {
        let rgba = LegacyColor::rgba(0., 0., 0., 0.);
        let rgba_l = LegacyColor::rgba_linear(0., 0., 0., 0.);
        let hsla = LegacyColor::hsla(0., 0., 0., 0.);
        let lcha = LegacyColor::lcha(0., 0., 0., 0.);
        assert_eq!(rgba_l, rgba_l.as_rgba_linear());
        let LegacyColor::RgbaLinear { .. } = rgba.as_rgba_linear() else {
            panic!("from Rgba")
        };
        let LegacyColor::RgbaLinear { .. } = hsla.as_rgba_linear() else {
            panic!("from Hsla")
        };
        let LegacyColor::RgbaLinear { .. } = lcha.as_rgba_linear() else {
            panic!("from Lcha")
        };
    }
}
