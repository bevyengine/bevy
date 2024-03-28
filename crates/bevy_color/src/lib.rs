#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Representations of colors in various color spaces.
//!
//! This crate provides a number of color representations, including:
//!
//! - [`Srgba`] (standard RGBA, with gamma correction)
//! - [`LinearRgba`] (linear RGBA, without gamma correction)
//! - [`Hsla`] (hue, saturation, lightness, alpha)
//! - [`Hsva`] (hue, saturation, value, alpha)
//! - [`Hwba`] (hue, whiteness, blackness, alpha)
//! - [`Laba`] (lightness, a-axis, b-axis, alpha)
//! - [`Lcha`] (lightness, chroma, hue, alpha)
//! - [`Oklaba`] (lightness, a-axis, b-axis, alpha)
//! - [`Oklcha`] (lightness, chroma, hue, alpha)
//! - [`Xyza`] (x-axis, y-axis, z-axis, alpha)
//!
//! Each of these color spaces is represented as a distinct Rust type.
//!
//! # Color Space Usage
//!
//! Rendering engines typically use linear RGBA colors, which allow for physically accurate
//! lighting calculations. However, linear RGBA colors are not perceptually uniform, because
//! both human eyes and computer monitors have non-linear responses to light. "Standard" RGBA
//! represents an industry-wide compromise designed to encode colors in a way that looks good to
//! humans in as few bits as possible, but it is not suitable for lighting calculations.
//!
//! Most image file formats and scene graph formats use standard RGBA, because graphic design
//! tools are intended to be used by humans. However, 3D lighting calculations operate in linear
//! RGBA, so it is important to convert standard colors to linear before sending them to the GPU.
//! Most Bevy APIs will handle this conversion automatically, but if you are writing a custom
//! shader, you will need to do this conversion yourself.
//!
//! HSL and LCH are "cylindrical" color spaces, which means they represent colors as a combination
//! of hue, saturation, and lightness (or chroma). These color spaces are useful for working
//! with colors in an artistic way - for example, when creating gradients or color palettes.
//! A gradient in HSL space from red to violet will produce a rainbow. The LCH color space is
//! more perceptually accurate than HSL, but is less intuitive to work with.
//!
//! HSV and HWB are very closely related to HSL in their derivation, having identical definitions for
//! hue. Where HSL uses saturation and lightness, HSV uses a slightly modified definition of saturation,
//! and an analog of lightness in the form of value. In contrast, HWB instead uses whiteness and blackness
//! parameters, which can be used to lighten and darken a particular hue respectively.
//!
//! Oklab and Oklch are perceptually uniform color spaces that are designed to be used for tasks such
//! as image processing. They are not as widely used as the other color spaces, but are useful
//! for tasks such as color correction and image analysis, where it is important to be able
//! to do things like change color saturation without causing hue shifts.
//!
//! XYZ is a foundational space commonly used in the definition of other more modern color
//! spaces. The space is more formally known as CIE 1931, where the `x` and `z` axes represent
//! a form of chromaticity, while `y` defines an illuminance level.
//!
//! See also the [Wikipedia article on color spaces](https://en.wikipedia.org/wiki/Color_space).
//!
#![doc = include_str!("../docs/conversion.md")]
//! <div>
#![doc = include_str!("../docs/diagrams/model_graph.svg")]
//! </div>
//!
//! # Other Utilities
//!
//! The crate also provides a number of color operations, such as blending, color difference,
//! and color range operations.
//!
//! In addition, there is a [`Color`] enum that can represent any of the color
//! types in this crate. This is useful when you need to store a color in a data structure
//! that can't be generic over the color type.
//!
//! Color types that are either physically or perceptually linear also implement `Add<Self>`, `Sub<Self>`, `Mul<f32>` and `Div<f32>`
//! allowing you to use them with splines.
//!
//! Please note that most often adding or subtracting colors is not what you may want.
//! Please have a look at other operations like blending, lightening or mixing colors using e.g. [`Mix`] or [`Luminance`] instead.
//!
//! # Example
//!
//! ```
//! use bevy_color::{Srgba, Hsla};
//!
//! let srgba = Srgba::new(0.5, 0.2, 0.8, 1.0);
//! let hsla: Hsla = srgba.into();
//!
//! println!("Srgba: {:?}", srgba);
//! println!("Hsla: {:?}", hsla);
//! ```

mod color;
pub mod color_difference;
mod color_ops;
mod color_range;
mod hsla;
mod hsva;
mod hwba;
mod laba;
mod lcha;
mod linear_rgba;
mod oklaba;
mod oklcha;
pub mod palettes;
mod srgba;
#[cfg(test)]
mod test_colors;
#[cfg(test)]
mod testing;
mod xyza;

/// Commonly used color types and traits.
pub mod prelude {
    pub use crate::color::*;
    pub use crate::color_ops::*;
    pub use crate::hsla::*;
    pub use crate::hsva::*;
    pub use crate::hwba::*;
    pub use crate::laba::*;
    pub use crate::lcha::*;
    pub use crate::linear_rgba::*;
    pub use crate::oklaba::*;
    pub use crate::oklcha::*;
    pub use crate::srgba::*;
    pub use crate::xyza::*;
}

pub use color::*;
pub use color_ops::*;
pub use color_range::*;
pub use hsla::*;
pub use hsva::*;
pub use hwba::*;
pub use laba::*;
pub use lcha::*;
pub use linear_rgba::*;
pub use oklaba::*;
pub use oklcha::*;
pub use srgba::*;
pub use xyza::*;

/// Describes the traits that a color should implement for consistency.
#[allow(dead_code)] // This is an internal marker trait used to ensure that our color types impl the required traits
pub(crate) trait StandardColor
where
    Self: core::fmt::Debug,
    Self: Clone + Copy,
    Self: PartialEq,
    Self: bevy_reflect::Reflect,
    Self: Default,
    Self: From<Color> + Into<Color>,
    Self: From<Srgba> + Into<Srgba>,
    Self: From<LinearRgba> + Into<LinearRgba>,
    Self: From<Hsla> + Into<Hsla>,
    Self: From<Hsva> + Into<Hsva>,
    Self: From<Hwba> + Into<Hwba>,
    Self: From<Laba> + Into<Laba>,
    Self: From<Lcha> + Into<Lcha>,
    Self: From<Oklaba> + Into<Oklaba>,
    Self: From<Oklcha> + Into<Oklcha>,
    Self: From<Xyza> + Into<Xyza>,
    Self: Alpha,
{
}

macro_rules! impl_componentwise_vector_space {
    ($ty: ident, [$($element: ident),+]) => {
        impl std::ops::Add<Self> for $ty {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self::Output {
                    $($element: self.$element + rhs.$element,)+
                }
            }
        }

        impl std::ops::AddAssign<Self> for $ty {
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }

        impl std::ops::Neg for $ty {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self::Output {
                    $($element: -self.$element,)+
                }
            }
        }

        impl std::ops::Sub<Self> for $ty {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Self::Output {
                    $($element: self.$element - rhs.$element,)+
                }
            }
        }

        impl std::ops::SubAssign<Self> for $ty {
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }

        impl std::ops::Mul<f32> for $ty {
            type Output = Self;

            fn mul(self, rhs: f32) -> Self::Output {
                Self::Output {
                    $($element: self.$element * rhs,)+
                }
            }
        }

        impl std::ops::Mul<$ty> for f32 {
            type Output = $ty;

            fn mul(self, rhs: $ty) -> Self::Output {
                Self::Output {
                    $($element: self * rhs.$element,)+
                }
            }
        }

        impl std::ops::MulAssign<f32> for $ty {
            fn mul_assign(&mut self, rhs: f32) {
                *self = *self * rhs;
            }
        }

        impl std::ops::Div<f32> for $ty {
            type Output = Self;

            fn div(self, rhs: f32) -> Self::Output {
                Self::Output {
                    $($element: self.$element / rhs,)+
                }
            }
        }

        impl std::ops::DivAssign<f32> for $ty {
            fn div_assign(&mut self, rhs: f32) {
                *self = *self / rhs;
            }
        }

        impl bevy_math::VectorSpace for $ty {
            const ZERO: Self = Self {
                $($element: 0.0,)+
            };
        }
    };
}

pub(crate) use impl_componentwise_vector_space;
