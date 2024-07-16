//! Provides a simple aspect ratio struct to help with calculations.

use crate::Vec2;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// An `AspectRatio` is the ratio of width to height.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
pub struct AspectRatio(f32);

impl AspectRatio {
    /// Create a new `AspectRatio` from a given `width` and `height`.
    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Self(width / height)
    }

    /// Create a new `AspectRatio` from a given amount of `x` pixels and `y` pixels.
    #[inline]
    pub fn from_pixels(x: u32, y: u32) -> Self {
        Self::new(x as f32, y as f32)
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
