//! Provides a simple aspect ratio struct to help with calculations.

pub mod error;

use crate::{aspect_ratio::error::AspectRatioError, Vec2};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// An `AspectRatio` is the ratio of width to height.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
pub struct AspectRatio(f32);

impl AspectRatio {
    /// Standard 16:9 aspect ratio
    pub const SIXTEEN_NINE: Self = Self(16.0 / 9.0);
    /// Standard 4:3 aspect ratio
    pub const FOUR_THREE: Self = Self(4.0 / 3.0);
    /// Standard 21:9 ultrawide aspect ratio
    pub const ULTRAWIDE: Self = Self(21.0 / 9.0);

    /// Create a new [`AspectRatio`] from a given width and height.
    ///
    /// # Panics
    /// Panics when the width and height values are invalid, such a zero or infinite widths/heights, or if either
    /// value is `NaN`.
    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Self::try_new(width, height).expect("Invalid aspect ratio")
    }

    /// Create a new [`AspectRatio`] from a given amount of x pixels and y pixels.
    #[inline]
    pub fn from_pixels(x: u32, y: u32) -> Self {
        Self::new(x as f32, y as f32)
    }

    /// Attempts to create a new [`AspectRatio`] from a given width and height.
    ///
    /// # Errors
    ///
    /// Returns an `Err` with `AspectRatioError` if:
    /// - Either width or height is zero (`AspectRatioError::Zero`)
    /// - Either width or height is infinite (`AspectRatioError::Infinite`)
    /// - Either width or height is NaN (`AspectRatioError::NaN`)
    #[inline]
    pub fn try_new(width: f32, height: f32) -> Result<Self, AspectRatioError> {
        match (width, height) {
            (w, h) if w == 0.0 || h == 0.0 => Err(AspectRatioError::Zero),
            (w, h) if w.is_infinite() || h.is_infinite() => Err(AspectRatioError::Infinite),
            (w, h) if w.is_nan() || h.is_nan() => Err(AspectRatioError::NaN),
            _ => Ok(Self(width / height)),
        }
    }

    /// Returns the inverse of this aspect ratio (height/width).
    #[inline]
    pub fn inverse(&self) -> Self {
        Self(1.0 / self.0)
    }

    /// Returns true if the aspect ratio represents a landscape orientation.
    #[inline]
    pub fn is_landscape(&self) -> bool {
        self.0 > 1.0
    }

    /// Returns true if the aspect ratio represents a portrait orientation.
    #[inline]
    pub fn is_portrait(&self) -> bool {
        self.0 < 1.0
    }

    /// Returns true if the aspect ratio is exactly square.
    #[inline]
    pub fn is_square(&self) -> bool {
        (self.0 - 1.0).abs() < f32::EPSILON
    }
}

impl From<Vec2> for AspectRatio {
    #[inline]
    fn from(value: Vec2) -> Self {
        Self::new(value.x, value.y)
    }
}

impl From<AspectRatio> for f32 {
    #[inline]
    fn from(aspect_ratio: AspectRatio) -> Self {
        aspect_ratio.0
    }
}

impl From<(f32, f32)> for AspectRatio {
    #[inline]
    fn from(value: (f32, f32)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(u32, u32)> for AspectRatio {
    #[inline]
    fn from(value: (u32, u32)) -> Self {
        Self::from_pixels(value.0, value.1)
    }
}
