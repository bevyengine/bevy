//! Module for calculating distance between two colors in the same color space.

use bevy_math::ops;

/// Calculate the distance between this and another color as if they were coordinates
/// in a Euclidean space. Alpha is not considered in the distance calculation.
pub trait EuclideanDistance: Sized {
    /// Distance from `self` to `other`.
    fn distance(&self, other: &Self) -> f32 {
        ops::sqrt(self.distance_squared(other))
    }

    /// Distance squared from `self` to `other`.
    fn distance_squared(&self, other: &Self) -> f32;
}
