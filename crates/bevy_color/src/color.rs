use crate::{
    color_difference::EuclideanDistance, Alpha, Hsla, Hsva, Hue, Hwba, Laba, Lcha, LinearRgba,
    Luminance, Mix, Oklaba, Oklcha, Saturation, Srgba, StandardColor, Xyza,
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;
use derive_more::derive::From;

/// An enumerated type that can represent any of the color types in this crate.
///
/// This is useful when you need to store a color in a data structure that can't be generic over
/// the color type.
#[doc = include_str!("../docs/conversion.md")]
/// <div>
#[doc = include_str!("../docs/diagrams/model_graph.svg")]
/// </div>
///
/// # Operations
///
/// [`Color`] supports all the standard color operations, such as [mixing](Mix),
/// [luminance](Luminance) and [hue](Hue) adjustment,
/// and [diffing](EuclideanDistance). These operations delegate to the concrete color space contained
/// by [`Color`], but will convert to [`Oklch`](Oklcha) for operations which aren't supported in the
/// current space. After performing the operation, if a conversion was required, the result will be
/// converted back into the original color space.
///
/// ```rust
/// # use bevy_color::{Hue, Color};
/// let red_hsv = Color::hsv(0., 1., 1.);
/// let red_srgb = Color::srgb(1., 0., 0.);
///
/// // HSV has a definition of hue, so it will be returned.
/// red_hsv.hue();
///
/// // SRGB doesn't have a native definition for hue.
/// // Converts to Oklch and returns that result.
/// red_srgb.hue();
/// ```
///
/// [`Oklch`](Oklcha) has been chosen as the intermediary space in cases where conversion is required
/// due to its perceptual uniformity and broad support for Bevy's color operations.
/// To avoid the cost of repeated conversion, and ensure consistent results where that is desired,
/// first convert this [`Color`] into your desired color space.
#[derive(Debug, Clone, Copy, PartialEq, From)]
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
pub enum Color {
    /// A color in the sRGB color space with alpha.
    Srgba(Srgba),
    /// A color in the linear sRGB color space with alpha.
    LinearRgba(LinearRgba),
    /// A color in the HSL color space with alpha.
    Hsla(Hsla),
    /// A color in the HSV color space with alpha.
    Hsva(Hsva),
    /// A color in the HWB color space with alpha.
    Hwba(Hwba),
    /// A color in the LAB color space with alpha.
    Laba(Laba),
    /// A color in the LCH color space with alpha.
    Lcha(Lcha),
    /// A color in the Oklab color space with alpha.
    Oklaba(Oklaba),
    /// A color in the Oklch color space with alpha.
    Oklcha(Oklcha),
    /// A color in the XYZ color space with alpha.
    Xyza(Xyza),
}

impl StandardColor for Color {}

impl Color {
    /// Return the color as a linear RGBA color.
    pub fn to_linear(&self) -> LinearRgba {
        (*self).into()
    }

    /// Return the color as an SRGBA color.
    pub fn to_srgba(&self) -> Srgba {
        (*self).into()
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn srgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self::Srgba(Srgba {
            red,
            green,
            blue,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    pub const fn srgb(red: f32, green: f32, blue: f32) -> Self {
        Self::Srgba(Srgba {
            red,
            green,
            blue,
            alpha: 1.0,
        })
    }

    /// Reads an array of floats to creates a new [`Color`] object storing a [`Srgba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    /// * `array` - Red, Green and Blue channels. Each channel is in the range [0.0, 1.0]
    pub const fn srgb_from_array(array: [f32; 3]) -> Self {
        Self::Srgba(Srgba {
            red: array[0],
            green: array[1],
            blue: array[2],
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color from [`u8`] values.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0, 255]
    /// * `green` - Green channel. [0, 255]
    /// * `blue` - Blue channel. [0, 255]
    /// * `alpha` - Alpha channel. [0, 255]
    pub const fn srgba_u8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self::Srgba(Srgba {
            red: red as f32 / 255.0,
            green: green as f32 / 255.0,
            blue: blue as f32 / 255.0,
            alpha: alpha as f32 / 255.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Srgba`] color from [`u8`] values with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0, 255]
    /// * `green` - Green channel. [0, 255]
    /// * `blue` - Blue channel. [0, 255]
    pub const fn srgb_u8(red: u8, green: u8, blue: u8) -> Self {
        Self::Srgba(Srgba {
            red: red as f32 / 255.0,
            green: green as f32 / 255.0,
            blue: blue as f32 / 255.0,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`LinearRgba`] color.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn linear_rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self::LinearRgba(LinearRgba {
            red,
            green,
            blue,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`LinearRgba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    pub const fn linear_rgb(red: f32, green: f32, blue: f32) -> Self {
        Self::LinearRgba(LinearRgba {
            red,
            green,
            blue,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Hsla`] color.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Self {
        Self::Hsla(Hsla {
            hue,
            saturation,
            lightness,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Hsla`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> Self {
        Self::Hsla(Hsla {
            hue,
            saturation,
            lightness,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Hsva`] color.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `value` - Value channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn hsva(hue: f32, saturation: f32, value: f32, alpha: f32) -> Self {
        Self::Hsva(Hsva {
            hue,
            saturation,
            value,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Hsva`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `value` - Value channel. [0.0, 1.0]
    pub const fn hsv(hue: f32, saturation: f32, value: f32) -> Self {
        Self::Hsva(Hsva {
            hue,
            saturation,
            value,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Hwba`] color.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `whiteness` - Whiteness channel. [0.0, 1.0]
    /// * `blackness` - Blackness channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn hwba(hue: f32, whiteness: f32, blackness: f32, alpha: f32) -> Self {
        Self::Hwba(Hwba {
            hue,
            whiteness,
            blackness,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Hwba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `whiteness` - Whiteness channel. [0.0, 1.0]
    /// * `blackness` - Blackness channel. [0.0, 1.0]
    pub const fn hwb(hue: f32, whiteness: f32, blackness: f32) -> Self {
        Self::Hwba(Hwba {
            hue,
            whiteness,
            blackness,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Laba`] color.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `a` - a axis. [-1.5, 1.5]
    /// * `b` - b axis. [-1.5, 1.5]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn laba(lightness: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self::Laba(Laba {
            lightness,
            a,
            b,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Laba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `a` - a axis. [-1.5, 1.5]
    /// * `b` - b axis. [-1.5, 1.5]
    pub const fn lab(lightness: f32, a: f32, b: f32) -> Self {
        Self::Laba(Laba {
            lightness,
            a,
            b,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Lcha`] color.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn lcha(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Self {
        Self::Lcha(Lcha {
            lightness,
            chroma,
            hue,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Lcha`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    pub const fn lch(lightness: f32, chroma: f32, hue: f32) -> Self {
        Self::Lcha(Lcha {
            lightness,
            chroma,
            hue,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Oklaba`] color.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `a` - Green-red channel. [-1.0, 1.0]
    /// * `b` - Blue-yellow channel. [-1.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn oklaba(lightness: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self::Oklaba(Oklaba {
            lightness,
            a,
            b,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Oklaba`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `a` - Green-red channel. [-1.0, 1.0]
    /// * `b` - Blue-yellow channel. [-1.0, 1.0]
    pub const fn oklab(lightness: f32, a: f32, b: f32) -> Self {
        Self::Oklaba(Oklaba {
            lightness,
            a,
            b,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Oklcha`] color.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `chroma` - Chroma channel. [0.0, 1.0]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn oklcha(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Self {
        Self::Oklcha(Oklcha {
            lightness,
            chroma,
            hue,
            alpha,
        })
    }

    /// Creates a new [`Color`] object storing a [`Oklcha`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `chroma` - Chroma channel. [0.0, 1.0]
    /// * `hue` - Hue channel. [0.0, 360.0]
    pub const fn oklch(lightness: f32, chroma: f32, hue: f32) -> Self {
        Self::Oklcha(Oklcha {
            lightness,
            chroma,
            hue,
            alpha: 1.0,
        })
    }

    /// Creates a new [`Color`] object storing a [`Xyza`] color.
    ///
    /// # Arguments
    ///
    /// * `x` - x-axis. [0.0, 1.0]
    /// * `y` - y-axis. [0.0, 1.0]
    /// * `z` - z-axis. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    pub const fn xyza(x: f32, y: f32, z: f32, alpha: f32) -> Self {
        Self::Xyza(Xyza { x, y, z, alpha })
    }

    /// Creates a new [`Color`] object storing a [`Xyza`] color with an alpha of 1.0.
    ///
    /// # Arguments
    ///
    /// * `x` - x-axis. [0.0, 1.0]
    /// * `y` - y-axis. [0.0, 1.0]
    /// * `z` - z-axis. [0.0, 1.0]
    pub const fn xyz(x: f32, y: f32, z: f32) -> Self {
        Self::Xyza(Xyza {
            x,
            y,
            z,
            alpha: 1.0,
        })
    }

    /// A fully white [`Color::LinearRgba`] color with an alpha of 1.0.
    pub const WHITE: Self = Self::linear_rgb(1.0, 1.0, 1.0);

    /// A fully black [`Color::LinearRgba`] color with an alpha of 1.0.
    pub const BLACK: Self = Self::linear_rgb(0., 0., 0.);

    /// A fully transparent [`Color::LinearRgba`] color with 0 red, green and blue.
    pub const NONE: Self = Self::linear_rgba(0., 0., 0., 0.);
}

impl Default for Color {
    /// A fully white [`Color::LinearRgba`] color with an alpha of 1.0.
    fn default() -> Self {
        Color::WHITE
    }
}

impl Alpha for Color {
    fn with_alpha(&self, alpha: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.with_alpha(alpha),
            Color::LinearRgba(x) => *x = x.with_alpha(alpha),
            Color::Hsla(x) => *x = x.with_alpha(alpha),
            Color::Hsva(x) => *x = x.with_alpha(alpha),
            Color::Hwba(x) => *x = x.with_alpha(alpha),
            Color::Laba(x) => *x = x.with_alpha(alpha),
            Color::Lcha(x) => *x = x.with_alpha(alpha),
            Color::Oklaba(x) => *x = x.with_alpha(alpha),
            Color::Oklcha(x) => *x = x.with_alpha(alpha),
            Color::Xyza(x) => *x = x.with_alpha(alpha),
        }

        new
    }

    fn alpha(&self) -> f32 {
        match self {
            Color::Srgba(x) => x.alpha(),
            Color::LinearRgba(x) => x.alpha(),
            Color::Hsla(x) => x.alpha(),
            Color::Hsva(x) => x.alpha(),
            Color::Hwba(x) => x.alpha(),
            Color::Laba(x) => x.alpha(),
            Color::Lcha(x) => x.alpha(),
            Color::Oklaba(x) => x.alpha(),
            Color::Oklcha(x) => x.alpha(),
            Color::Xyza(x) => x.alpha(),
        }
    }

    fn set_alpha(&mut self, alpha: f32) {
        match self {
            Color::Srgba(x) => x.set_alpha(alpha),
            Color::LinearRgba(x) => x.set_alpha(alpha),
            Color::Hsla(x) => x.set_alpha(alpha),
            Color::Hsva(x) => x.set_alpha(alpha),
            Color::Hwba(x) => x.set_alpha(alpha),
            Color::Laba(x) => x.set_alpha(alpha),
            Color::Lcha(x) => x.set_alpha(alpha),
            Color::Oklaba(x) => x.set_alpha(alpha),
            Color::Oklcha(x) => x.set_alpha(alpha),
            Color::Xyza(x) => x.set_alpha(alpha),
        }
    }
}

impl From<Color> for Srgba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba,
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for LinearRgba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear,
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Hsla {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla,
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Hsva {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva,
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Hwba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba,
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Laba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba,
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Lcha {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha,
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Oklaba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab,
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Oklcha {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
            Color::Oklcha(oklch) => oklch,
            Color::Xyza(xyza) => xyza.into(),
        }
    }
}

impl From<Color> for Xyza {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(x) => x.into(),
            Color::LinearRgba(x) => x.into(),
            Color::Hsla(x) => x.into(),
            Color::Hsva(hsva) => hsva.into(),
            Color::Hwba(hwba) => hwba.into(),
            Color::Laba(laba) => laba.into(),
            Color::Lcha(x) => x.into(),
            Color::Oklaba(x) => x.into(),
            Color::Oklcha(oklch) => oklch.into(),
            Color::Xyza(xyza) => xyza,
        }
    }
}

/// Color space chosen for operations on `Color`.
type ChosenColorSpace = Oklcha;

impl Luminance for Color {
    fn luminance(&self) -> f32 {
        match self {
            Color::Srgba(x) => x.luminance(),
            Color::LinearRgba(x) => x.luminance(),
            Color::Hsla(x) => x.luminance(),
            Color::Hsva(x) => ChosenColorSpace::from(*x).luminance(),
            Color::Hwba(x) => ChosenColorSpace::from(*x).luminance(),
            Color::Laba(x) => x.luminance(),
            Color::Lcha(x) => x.luminance(),
            Color::Oklaba(x) => x.luminance(),
            Color::Oklcha(x) => x.luminance(),
            Color::Xyza(x) => x.luminance(),
        }
    }

    fn with_luminance(&self, value: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.with_luminance(value),
            Color::LinearRgba(x) => *x = x.with_luminance(value),
            Color::Hsla(x) => *x = x.with_luminance(value),
            Color::Hsva(x) => *x = ChosenColorSpace::from(*x).with_luminance(value).into(),
            Color::Hwba(x) => *x = ChosenColorSpace::from(*x).with_luminance(value).into(),
            Color::Laba(x) => *x = x.with_luminance(value),
            Color::Lcha(x) => *x = x.with_luminance(value),
            Color::Oklaba(x) => *x = x.with_luminance(value),
            Color::Oklcha(x) => *x = x.with_luminance(value),
            Color::Xyza(x) => *x = x.with_luminance(value),
        }

        new
    }

    fn darker(&self, amount: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.darker(amount),
            Color::LinearRgba(x) => *x = x.darker(amount),
            Color::Hsla(x) => *x = x.darker(amount),
            Color::Hsva(x) => *x = ChosenColorSpace::from(*x).darker(amount).into(),
            Color::Hwba(x) => *x = ChosenColorSpace::from(*x).darker(amount).into(),
            Color::Laba(x) => *x = x.darker(amount),
            Color::Lcha(x) => *x = x.darker(amount),
            Color::Oklaba(x) => *x = x.darker(amount),
            Color::Oklcha(x) => *x = x.darker(amount),
            Color::Xyza(x) => *x = x.darker(amount),
        }

        new
    }

    fn lighter(&self, amount: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.lighter(amount),
            Color::LinearRgba(x) => *x = x.lighter(amount),
            Color::Hsla(x) => *x = x.lighter(amount),
            Color::Hsva(x) => *x = ChosenColorSpace::from(*x).lighter(amount).into(),
            Color::Hwba(x) => *x = ChosenColorSpace::from(*x).lighter(amount).into(),
            Color::Laba(x) => *x = x.lighter(amount),
            Color::Lcha(x) => *x = x.lighter(amount),
            Color::Oklaba(x) => *x = x.lighter(amount),
            Color::Oklcha(x) => *x = x.lighter(amount),
            Color::Xyza(x) => *x = x.lighter(amount),
        }

        new
    }
}

impl Hue for Color {
    fn with_hue(&self, hue: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = ChosenColorSpace::from(*x).with_hue(hue).into(),
            Color::LinearRgba(x) => *x = ChosenColorSpace::from(*x).with_hue(hue).into(),
            Color::Hsla(x) => *x = x.with_hue(hue),
            Color::Hsva(x) => *x = x.with_hue(hue),
            Color::Hwba(x) => *x = x.with_hue(hue),
            Color::Laba(x) => *x = ChosenColorSpace::from(*x).with_hue(hue).into(),
            Color::Lcha(x) => *x = x.with_hue(hue),
            Color::Oklaba(x) => *x = ChosenColorSpace::from(*x).with_hue(hue).into(),
            Color::Oklcha(x) => *x = x.with_hue(hue),
            Color::Xyza(x) => *x = ChosenColorSpace::from(*x).with_hue(hue).into(),
        }

        new
    }

    fn hue(&self) -> f32 {
        match self {
            Color::Srgba(x) => ChosenColorSpace::from(*x).hue(),
            Color::LinearRgba(x) => ChosenColorSpace::from(*x).hue(),
            Color::Hsla(x) => x.hue(),
            Color::Hsva(x) => x.hue(),
            Color::Hwba(x) => x.hue(),
            Color::Laba(x) => ChosenColorSpace::from(*x).hue(),
            Color::Lcha(x) => x.hue(),
            Color::Oklaba(x) => ChosenColorSpace::from(*x).hue(),
            Color::Oklcha(x) => x.hue(),
            Color::Xyza(x) => ChosenColorSpace::from(*x).hue(),
        }
    }

    fn set_hue(&mut self, hue: f32) {
        *self = self.with_hue(hue);
    }
}

impl Saturation for Color {
    fn with_saturation(&self, saturation: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => Hsla::from(*x).with_saturation(saturation).into(),
            Color::LinearRgba(x) => Hsla::from(*x).with_saturation(saturation).into(),
            Color::Hsla(x) => x.with_saturation(saturation).into(),
            Color::Hsva(x) => x.with_saturation(saturation).into(),
            Color::Hwba(x) => Hsla::from(*x).with_saturation(saturation).into(),
            Color::Laba(x) => Hsla::from(*x).with_saturation(saturation).into(),
            Color::Lcha(x) => Hsla::from(*x).with_saturation(saturation).into(),
            Color::Oklaba(x) => Hsla::from(*x).with_saturation(saturation).into(),
            Color::Oklcha(x) => Hsla::from(*x).with_saturation(saturation).into(),
            Color::Xyza(x) => Hsla::from(*x).with_saturation(saturation).into(),
        }
    }

    fn saturation(&self) -> f32 {
        match self {
            Color::Srgba(x) => Hsla::from(*x).saturation(),
            Color::LinearRgba(x) => Hsla::from(*x).saturation(),
            Color::Hsla(x) => x.saturation(),
            Color::Hsva(x) => x.saturation(),
            Color::Hwba(x) => Hsla::from(*x).saturation(),
            Color::Laba(x) => Hsla::from(*x).saturation(),
            Color::Lcha(x) => Hsla::from(*x).saturation(),
            Color::Oklaba(x) => Hsla::from(*x).saturation(),
            Color::Oklcha(x) => Hsla::from(*x).saturation(),
            Color::Xyza(x) => Hsla::from(*x).saturation(),
        }
    }

    fn set_saturation(&mut self, saturation: f32) {
        *self = self.with_saturation(saturation);
    }
}

impl Mix for Color {
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.mix(&(*other).into(), factor),
            Color::LinearRgba(x) => *x = x.mix(&(*other).into(), factor),
            Color::Hsla(x) => *x = x.mix(&(*other).into(), factor),
            Color::Hsva(x) => *x = x.mix(&(*other).into(), factor),
            Color::Hwba(x) => *x = x.mix(&(*other).into(), factor),
            Color::Laba(x) => *x = x.mix(&(*other).into(), factor),
            Color::Lcha(x) => *x = x.mix(&(*other).into(), factor),
            Color::Oklaba(x) => *x = x.mix(&(*other).into(), factor),
            Color::Oklcha(x) => *x = x.mix(&(*other).into(), factor),
            Color::Xyza(x) => *x = x.mix(&(*other).into(), factor),
        }

        new
    }
}

impl EuclideanDistance for Color {
    fn distance_squared(&self, other: &Self) -> f32 {
        match self {
            Color::Srgba(x) => x.distance_squared(&(*other).into()),
            Color::LinearRgba(x) => x.distance_squared(&(*other).into()),
            Color::Hsla(x) => ChosenColorSpace::from(*x).distance_squared(&(*other).into()),
            Color::Hsva(x) => ChosenColorSpace::from(*x).distance_squared(&(*other).into()),
            Color::Hwba(x) => ChosenColorSpace::from(*x).distance_squared(&(*other).into()),
            Color::Laba(x) => ChosenColorSpace::from(*x).distance_squared(&(*other).into()),
            Color::Lcha(x) => ChosenColorSpace::from(*x).distance_squared(&(*other).into()),
            Color::Oklaba(x) => x.distance_squared(&(*other).into()),
            Color::Oklcha(x) => x.distance_squared(&(*other).into()),
            Color::Xyza(x) => ChosenColorSpace::from(*x).distance_squared(&(*other).into()),
        }
    }
}
