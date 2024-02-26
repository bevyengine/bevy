use crate::{Alpha, Hsla, Hsva, Hwba, Laba, Lcha, LinearRgba, Oklaba, Srgba, StandardColor, Xyza};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

use macros::define_color;

define_color! {
    /// An enumerated type that can represent any of the color types in this crate.
    ///
    /// This is useful when you need to store a color in a data structure that can't be generic over
    /// the color type.
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
    #[reflect(PartialEq, Serialize, Deserialize)]
    pub enum Color {
        /// A color in the sRGB color space with alpha.
        Srgba,
        /// A color in the linear sRGB color space with alpha.
        LinearRgba,
        /// A color in the HSL color space with alpha.
        Hsla,
        /// A color in the HSV color space with alpha.
        Hsva,
        /// A color in the HWB color space with alpha.
        Hwba,
        /// A color in the LAB color space with alpha.
        Laba,
        /// A color in the LCH color space with alpha.
        Lcha,
        /// A color in the Oklaba color space with alpha.
        Oklaba,
        /// A color in the XYZ color space with alpha.
        Xyza
    }
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

mod macros {
    //! Macro rules put into their own module to allow reordered definitions

    /// Creates a color enum where each variant contains a color type of the same name.
    /// Also implements various traits based on combinations of these colors.
    // Only this first rule is intended to be used. Subsequent rules are used to create all
    // 2D combinations of colors for trait implementations.
    macro_rules! define_color {
        ($(#[$c_attr:meta])* pub enum $color:ident { $($(#[$x_attr:meta])* $x:ident),* }) => {
            $(#[$c_attr])*
            pub enum $color {
                $(
                    $(#[$x_attr])*
                    $x($x),
                )*
            }

            define_color! { $color $($x),* }
        };
        ($color:ident [$([$x:ident, [$($y:ident),*]]),*]) => {
            impl Alpha for $color {
                fn with_alpha(&self, alpha: f32) -> Self {
                    let mut new = *self;

                    match &mut new {
                        $(
                            $color::$x(x) => *x = x.with_alpha(alpha),
                        )*
                    };

                    new
                }

                fn alpha(&self) -> f32 {
                    match self {
                        $(
                            $color::$x(x) => x.alpha(),
                        )*
                    }
                }
            }

            $(
                impl From<$x> for $color {
                    fn from(value: $x) -> Self {
                        Self::$x(value)
                    }
                }

                impl<'a> From<&'a $x> for $color {
                    fn from(value: &'a $x) -> Self {
                        (*value).into()
                    }
                }

                impl From<$color> for $x {
                    fn from(value: $color) -> Self {
                        match value {
                            $(
                                $color::$y(x) => Self::from(x),
                            )*
                        }
                    }
                }

                impl<'a> From<&'a $color> for $x {
                    fn from(value: &'a $color) -> Self {
                        (*value).into()
                    }
                }
            )*
        };
        ($color:ident $($x:ident),*) => {
            define_color!($color [$($x),*], [$($x),*]);
        };
        ($color:ident [$($x:ident),*], $all:tt) => {
            define_color!($color [$([$x, $all]),*]);
        };
    }

    pub(crate) use define_color;
}

