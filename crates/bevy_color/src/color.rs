use crate::{Hsla, Lcha, LinearRgba, Oklaba, Srgba};

/// An enumerated type that can represent any of the color types in this crate.
///
/// This is useful when you need to store a color in a data structure that can't be generic over
/// the color type.
#[derive(Debug, Clone, Copy, PartialEq)]
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
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab.into(),
        }
    }
}

impl From<Color> for Lcha {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Lcha(lcha) => lcha,
            Color::Oklaba(oklab) => oklab.into(),
        }
    }
}

impl From<Color> for Oklaba {
    fn from(value: Color) -> Self {
        match value {
            Color::Srgba(srgba) => srgba.into(),
            Color::LinearRgba(linear) => linear.into(),
            Color::Hsla(hsla) => hsla.into(),
            Color::Lcha(lcha) => lcha.into(),
            Color::Oklaba(oklab) => oklab,
        }
    }
}
