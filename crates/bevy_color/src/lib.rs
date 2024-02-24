//! Representations of colors in various color spaces.
//!
//! This crate provides a number of color representations, including:
//!
//! - [`Srgba`] (standard RGBA, with gamma correction)
//! - [`LinearRgba`] (linear RGBA, without gamma correction)
//! - [`Hsla`] (hue, saturation, lightness, alpha)
//! - [`Lcha`] (lightness, chroma, hue, alpha)
//! - [`Oklaba`] (lightness, a-axis, b-axis, alpha)
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
//! Oklab is a perceptually uniform color space that is designed to be used for tasks such
//! as image processing. It is not as widely used as the other color spaces, but it is useful
//! for tasks such as color correction and image analysis, where it is important to be able
//! to do things like change color saturation without causing hue shifts.
//!
//! See also the [Wikipedia article on color spaces](https://en.wikipedia.org/wiki/Color_space).
//!
//! # Conversions
//!
//! Each color space can be converted to and from the others using the [`From`] trait. Not all
//! possible combinations of conversions are provided, but every color space has a converstion to
//! and from [`Srgba`] and [`LinearRgba`].
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
mod lcha;
mod linear_rgba;
mod oklaba;
mod srgba;
#[cfg(test)]
mod test_colors;
#[cfg(test)]
mod testing;

pub use color::*;
pub use color_ops::*;
pub use color_range::*;
pub use hsla::*;
pub use lcha::*;
pub use linear_rgba::*;
pub use oklaba::*;
pub use srgba::*;

use bevy_render::color::Color as LegacyColor;

/// Enforces that an implementing type can be transformed to and from a type `T`.
pub(crate) trait InterchangeableWith<T>
where
    Self: From<T> + Into<T>,
{
}

impl<C, T> InterchangeableWith<T> for C
where
    C: From<T>,
    T: From<C>,
{
}

/// Describes the traits that a color should implement for consistency.
pub(crate) trait StandardColor
where
    Self: core::fmt::Debug,
    Self: Clone + Copy,
    Self: PartialEq,
    Self: serde::Serialize + for<'a> serde::Deserialize<'a>,
    Self: bevy_reflect::Reflect,
    Self: Default,
    Self: From<Color> + Into<Color>,
    Self: From<LegacyColor> + Into<LegacyColor>,
    Self: From<Srgba> + Into<Srgba>,
    Self: From<LinearRgba> + Into<LinearRgba>,
    Self: From<Hsla> + Into<Hsla>,
    Self: From<Lcha> + Into<Lcha>,
    Self: From<Oklaba> + Into<Oklaba>,
    Self: Alpha,
{
}
