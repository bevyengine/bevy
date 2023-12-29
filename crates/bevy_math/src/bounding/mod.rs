//! This module contains traits and implements for working with bounding shapes
//!
//! There are four traits used:
//! - [`BoundingVolume`] is a generic abstraction for any bounding volume
//! - [`IntersectsVolume`] abstracts intersection tests against a [`BoundingVolume`]
//! - [`Bounded2d`]/[`Bounded3d`] are abstractions for shapes to generate [`BoundingVolume`]s

/// A trait that generalizes different bounding volumes.
/// Bounding volumes are simplified shapes that are used to get simpler ways to check for
/// overlapping elements or finding intersections.
///
/// This trait supports both 2D and 3D bounding shapes.
pub trait BoundingVolume {
    /// The position type used for the volume. This should be `Vec2` for 2D and `Vec3` for 3D.
    type Position: Clone + Copy + PartialEq;
    /// The type used for the `padded` and `shrunk` methods. For example, an `f32` radius for
    /// circles and spheres.
    type Padding;

    /// Returns the center of the bounding volume.
    fn center(&self) -> Self::Position;

    /// Computes the visible surface area of the bounding volume.
    /// This method can be useful to make decisions about merging bounding volumes,
    /// using a Surface Area Heuristic.
    ///
    /// For 2D shapes this would simply be the area of the shape.
    /// For 3D shapes this would usually be half the area of the shape.
    fn visible_area(&self) -> f32;

    /// Checks if this bounding volume contains another one.
    fn contains(&self, other: &Self) -> bool;

    /// Computes the smallest bounding volume that contains both `self` and `other`.
    fn merged(&self, other: &Self) -> Self;

    /// Expand the bounding volume in each direction by the given amount
    fn padded(&self, amount: Self::Padding) -> Self;

    /// Shrink the bounding volume in each direction by the given amount
    fn shrunk(&self, amount: Self::Padding) -> Self;
}

/// A trait that generalizes intersection tests against a volume.
/// Intersection tests can be used for a variety of tasks, for example:
/// - Raycasting
/// - Testing for overlap
/// - Checking if an object is within the view frustum of a camera
pub trait IntersectsVolume<Volume: BoundingVolume> {
    /// Check if a volume intersects with this intersection test
    fn intersects(&self, volume: &Volume) -> bool;
}

mod bounded2d;
pub use bounded2d::*;
mod bounded3d;
pub use bounded3d::*;
