//! Module for calculating distance between two colors in the same color space.

/// Calculate the distance between this and another color as if they were coordinates
/// in a Euclidean space. Alpha is not considered in the distance calculation.
pub trait EuclideanDistance: Sized {
    /// Distance from `self` to `other`.
    fn distance(&self, other: &Self) -> f32 {
        self.distance_squared(other).sqrt()
    }

    /// Distance squared from `self` to `other`.
    fn distance_squared(&self, other: &Self) -> f32;
}
