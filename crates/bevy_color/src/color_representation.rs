use crate::{Hsla, Lcha, LinearRgba, Oklaba, Srgba};

/// An enumerated type that can represent any of the color types in this crate.
///
/// This is useful when you need to store a color in a data structure that can't be generic over
/// the color type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorRepresentation {
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

impl ColorRepresentation {
    /// Return the color as a linear RGBA color.
    pub fn linear(&self) -> LinearRgba {
        match self {
            ColorRepresentation::Srgba(srgba) => (*srgba).into(),
            ColorRepresentation::LinearRgba(linear) => *linear,
            ColorRepresentation::Hsla(hsla) => (*hsla).into(),
            ColorRepresentation::Lcha(lcha) => (*lcha).into(),
            ColorRepresentation::Oklaba(oklab) => (*oklab).into(),
        }
    }
}

impl Default for ColorRepresentation {
    fn default() -> Self {
        Self::Srgba(Srgba::WHITE)
    }
}

impl From<Srgba> for ColorRepresentation {
    fn from(value: Srgba) -> Self {
        Self::Srgba(value)
    }
}

impl From<LinearRgba> for ColorRepresentation {
    fn from(value: LinearRgba) -> Self {
        Self::LinearRgba(value)
    }
}

impl From<Hsla> for ColorRepresentation {
    fn from(value: Hsla) -> Self {
        Self::Hsla(value)
    }
}

impl From<Oklaba> for ColorRepresentation {
    fn from(value: Oklaba) -> Self {
        Self::Oklaba(value)
    }
}
