//! Representations of colors in various color spaces.
//!
//! This crate provides a number of color representations, including:
//!
//! - [`Srgba`] (standard RGBA, with gamma correction)
//! - [`LinearRgba`] (linear RGBA, without gamma correction)
//! - [`Hsla`] (hue, saturation, lightness, alpha)
//! - [`Lcha`] (lightness, chroma, hue, alpha)
//! - [`Oklaba`] (hue, chroma, lightness, alpha)
//!
//! Each of these color spaces is represented as a distinct Rust type. Colors can be converted
//! from one color space to another using the [`From`] trait.
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
//! tools are intended to be used by humans. However, GPU hardware operates on linear RGBA, so it
//! is important to convert standard colors to linear before sending them to the GPU. Most Bevy
//! APIs will handle this conversion automatically, but if you are writing a custom shader, you
//! will need to do this conversion yourself.
//!
//! # Other Utilities
//!
//! The crate also provides a number of color operations, such as blending, color difference,
//! and color range operations.
//!
//! In addition, there is a [`ColorRepresentation`] enum that can represent any of the color
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

pub mod color_difference;
mod color_ops;
mod color_range;
mod color_representation;
mod hsla;
mod lcha;
mod linear_rgba;
mod oklaba;
mod srgba;
#[cfg(test)]
mod test_colors;
#[cfg(test)]
mod testing;

pub use color_ops::*;
pub use color_range::*;
pub use color_representation::*;
pub use hsla::*;
pub use lcha::*;
pub use linear_rgba::*;
pub use oklaba::*;
pub use srgba::*;
