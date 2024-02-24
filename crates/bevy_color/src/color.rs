use crate::{Alpha, Hsla, Lcha, LinearRgba, Mix, Oklaba, Srgba, StandardColor};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::color::Color as LegacyColor;
use serde::{Deserialize, Serialize};

/// An enumerated type that can represent any of the color types in this crate.
///
/// This is useful when you need to store a color in a data structure that can't be generic over
/// the color type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Color {
    /// A color in the sRGB color space with alpha.
    Srgba(Srgba),
    /// A color in the linear sRGB color space with alpha.
    LinearRgba(LinearRgba),
    /// A color in the HSL color space with alpha.
    Hsla(Hsla),
    /// A color in the LCH color space with alpha.
    Lcha(Lcha),
    /// A color in the Oklaba color space with alpha.
    Oklaba(Oklaba),
}

impl StandardColor for Color {}

impl Color {
    /// Return the color as a linear RGBA color.
    pub fn linear(&self) -> LinearRgba {
        match self {
            Color::Srgba(srgba) => (*srgba).into(),
            Color::LinearRgba(linear) => *linear,
            Color::Hsla(hsla) => (*hsla).into(),
            Color::Lcha(lcha) => (*lcha).into(),
            Color::Oklaba(oklab) => (*oklab).into(),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::Srgba(Srgba::WHITE)
    }
}

impl Alpha for Color {
    fn with_alpha(&self, alpha: f32) -> Self {
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.with_alpha(alpha),
            Color::LinearRgba(x) => *x = x.with_alpha(alpha),
            Color::Hsla(x) => *x = x.with_alpha(alpha),
            Color::Lcha(x) => *x = x.with_alpha(alpha),
            Color::Oklaba(x) => *x = x.with_alpha(alpha),
        }

        new
    }

    fn alpha(&self) -> f32 {
        match self {
            Color::Srgba(x) => x.alpha(),
            Color::LinearRgba(x) => x.alpha(),
            Color::Hsla(x) => x.alpha(),
            Color::Lcha(x) => x.alpha(),
            Color::Oklaba(x) => x.alpha(),
        }
    }
}

impl Mix for Color {
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let other = *other;
        let mut new = *self;

        match &mut new {
            Color::Srgba(x) => *x = x.mix(&other.into(), factor),
            Color::LinearRgba(x) => *x = x.mix(&other.into(), factor),
            Color::Hsla(x) => *x = x.mix(&other.into(), factor),
            Color::Lcha(x) => *x = x.mix(&other.into(), factor),
            Color::Oklaba(x) => *x = x.mix(&other.into(), factor),
        }

        new
    }
}

impl From<Srgba> for Color {
    fn from(value: Srgba) -> Self {
        Self::Srgba(value)
    }
}

impl From<LinearRgba> for Color {
    fn from(value: LinearRgba) -> Self {
        Self::LinearRgba(value)
    }
}

impl From<Hsla> for Color {
    fn from(value: Hsla) -> Self {
        Self::Hsla(value)
    }
}

impl From<Oklaba> for Color {
    fn from(value: Oklaba) -> Self {
        Self::Oklaba(value)
    }
}

impl From<Lcha> for Color {
    fn from(value: Lcha) -> Self {
        Self::Lcha(value)
    }
}

impl From<Color> for Srgba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba,
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
        }
    }
}

impl From<Color> for LinearRgba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear,
            Color::Hsla(hsla) => hsla.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
        }
    }
}

impl From<Color> for Hsla {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla,
            Color::Lcha(lcha) => LinearRgba::from(lcha).into(),
            Color::Oklaba(oklab) => LinearRgba::from(oklab).into(),
        }
    }
}

impl From<Color> for Lcha {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => Srgba::from(hsla).into(),
            Color::Lcha(lcha) => lcha,
            Color::Oklaba(oklab) => LinearRgba::from(oklab).into(),
        }
    }
}

impl From<Color> for Oklaba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => Srgba::from(hsla).into(),
            Color::Lcha(lcha) => LinearRgba::from(lcha).into(),
            Color::Oklaba(oklab) => oklab,
        }
    }
}

impl From<LegacyColor> for Color {
    fn from(value: LegacyColor) -> Self {
        match value {
            LegacyColor::Rgba { .. } => Srgba::from(value).into(),
            LegacyColor::RgbaLinear { .. } => LinearRgba::from(value).into(),
            LegacyColor::Hsla { .. } => Hsla::from(value).into(),
            LegacyColor::Lcha { .. } => Lcha::from(value).into(),
        }
    }
}

impl From<Color> for LegacyColor {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(x) => x.into(),
            Color::LinearRgba(x) => x.into(),
            Color::Hsla(x) => x.into(),
            Color::Lcha(x) => x.into(),
            Color::Oklaba(x) => x.into(),
        }
    }
}
