//! Contains error types related to aspect ratio calculations.

/// An Error type for when [`super::AspectRatio`] is provided invalid width or height values
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AspectRatioError {
    /// Error due to width or height having zero as a value.
    Zero,
    /// Error due to width or height being infinite.
    Infinite,
    /// Error due to width or height being Not a Number (NaN).
    NaN,
}

impl std::fmt::Display for AspectRatioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AspectRatioError::Zero => write!(f, "AspectRatio error: width or height is zero"),
            AspectRatioError::Infinite => {
                write!(f, "AspectRatio error: width or height is infinite")
            }
            AspectRatioError::NaN => write!(f, "AspectRatio error: width or height is NaN"),
        }
    }
}

impl std::error::Error for AspectRatioError {}
