//! This module contains traits and implements for working with bounding shapes

/// A trait that generalizes different bounding volumes.
/// It supports both 2D and 3D bounding shapes.
pub trait BoundingVolume {
    /// The position type used for the volume. This should be Vec2 for 2D and Vec3 for 3D.
    type Position: Clone + Copy + PartialEq;
    /// The type used for the `padded` and `shrunk` methods.
    type Padding;

    /// Returns the center of the bounding volume.
    fn center(&self) -> Self::Position;

    /// Computes the maximum surface area of the bounding volume that is visible from any angle.
    fn visible_area(&self) -> f32;

    /// Checks if this bounding volume contains another one.
    fn contains(&self, other: &Self) -> bool;

    /// Computes the smallest bounding volume that contains both `self` and `other`.
    fn merged(&self, other: &Self) -> Self;

    /// Increases the size of this bounding volume by the given amount.
    fn padded(&self, amount: Self::Padding) -> Self;

    /// Decreases the size of this bounding volume by the given amount.
    fn shrunk(&self, amount: Self::Padding) -> Self;
}

/// A trait that generalizes intersection tests against a volume
pub trait IntersectsVolume<Volume: BoundingVolume> {
    /// Check if a volume intersects with this intersection test
    fn intersects(&self, volume: &Volume) -> bool;
}

mod bounded2d;
pub use bounded2d::*;
mod bounded3d;
pub use bounded3d::*;
