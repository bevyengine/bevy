//! Provides a simple aspect ratio struct to help with calculations.

/// An `AspectRatio` is the ratio of width to height.
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

impl From<AspectRatio> for f32 {
    #[inline]
    fn from(aspect_ratio: AspectRatio) -> Self {
        aspect_ratio.0
    }
}
