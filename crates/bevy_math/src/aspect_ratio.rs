//! Provides a simple aspect ratio struct to help with calculations.

/// An `AspectRatio` is the ratio of width to height.
pub struct AspectRatio(f32);

impl AspectRatio {
    /// Create a new `AspectRatio` from a given `width` and `height`.
    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Self(width / height)
    }
}

impl From<(f32, f32)> for AspectRatio {
    #[inline]
    fn from((width, height): (f32, f32)) -> Self {
        Self::new(width, height)
    }
}

impl From<AspectRatio> for f32 {
    #[inline]
    fn from(aspect_ratio: AspectRatio) -> Self {
        aspect_ratio.0
    }
}
