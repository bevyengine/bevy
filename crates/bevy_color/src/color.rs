use crate::{Alpha, Hsla, Hsva, Hwba, Laba, Lcha, LinearRgba, Oklaba, Srgba, StandardColor, Xyza};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
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
    /// A color in the HSV color space with alpha.
    Hsva(Hsva),
    /// A color in the HWB color space with alpha.
    Hwba(Hwba),
    /// A color in the LAB color space with alpha.
    Laba(Laba),
    /// A color in the LCH color space with alpha.
    Lcha(Lcha),
    /// A color in the Oklaba color space with alpha.
    Oklaba(Oklaba),
    /// A color in the XYZ color space with alpha.
    Xyza(Xyza),
}

impl StandardColor for Color {}

impl Color {
    /// Return the color as a linear RGBA color.
    pub fn linear(&self) -> LinearRgba {
        (*self).into()
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
            Color::Hsva(x) => *x = x.with_alpha(alpha),
            Color::Hwba(x) => *x = x.with_alpha(alpha),
            Color::Laba(x) => *x = x.with_alpha(alpha),
            Color::Lcha(x) => *x = x.with_alpha(alpha),
            Color::Oklaba(x) => *x = x.with_alpha(alpha),
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
            Color::Xyza(x) => x.alpha(),
        }
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

impl From<Hsva> for Color {
    fn from(value: Hsva) -> Self {
        Self::Hsva(value)
    }
}

impl From<Hwba> for Color {
    fn from(value: Hwba) -> Self {
        Self::Hwba(value)
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

impl From<Laba> for Color {
    fn from(value: Laba) -> Self {
        Self::Laba(value)
    }
}

impl From<Xyza> for Color {
    fn from(value: Xyza) -> Self {
        Self::Xyza(value)
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
            Color::Xyza(xyza) => xyza,
        }
    }
}
