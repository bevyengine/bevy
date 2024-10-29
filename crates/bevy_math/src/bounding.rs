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
pub trait BoundingVolume: Sized {
    /// The position type used for the volume. This should be `Vec2` for 2D and `Vec3` for 3D.
    type Translation: Clone + Copy + PartialEq;

    /// The rotation type used for the volume. This should be `f32` for 2D and `Quat` for 3D.
    type Rotation: Clone + Copy + PartialEq;

    /// The type used for the size of the bounding volume. Usually a half size. For example an
    /// `f32` radius for a circle, or a `Vec3` with half sizes for x, y and z for a 3D axis-aligned
    /// bounding box
    type HalfSize;

    /// Returns the center of the bounding volume.
    fn center(&self) -> Self::Translation;

    /// Returns the half size of the bounding volume.
    fn half_size(&self) -> Self::HalfSize;

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
    fn merge(&self, other: &Self) -> Self;

    /// Increases the size of the bounding volume in each direction by the given amount.
    fn grow(&self, amount: impl Into<Self::HalfSize>) -> Self;

    /// Decreases the size of the bounding volume in each direction by the given amount.
    fn shrink(&self, amount: impl Into<Self::HalfSize>) -> Self;

    /// Scale the size of the bounding volume around its center by the given amount
    fn scale_around_center(&self, scale: impl Into<Self::HalfSize>) -> Self;

    /// Transforms the bounding volume by first rotating it around the origin and then applying a translation.
    fn transformed_by(
        mut self,
        translation: impl Into<Self::Translation>,
        rotation: impl Into<Self::Rotation>,
    ) -> Self {
        self.transform_by(translation, rotation);
        self
    }

    /// Transforms the bounding volume by first rotating it around the origin and then applying a translation.
    fn transform_by(
        &mut self,
        translation: impl Into<Self::Translation>,
        rotation: impl Into<Self::Rotation>,
    ) {
        self.rotate_by(rotation);
        self.translate_by(translation);
    }

    /// Translates the bounding volume by the given translation.
    fn translated_by(mut self, translation: impl Into<Self::Translation>) -> Self {
        self.translate_by(translation);
        self
    }

    /// Translates the bounding volume by the given translation.
    fn translate_by(&mut self, translation: impl Into<Self::Translation>);

    /// Rotates the bounding volume around the origin by the given rotation.
    ///
    /// The result is a combination of the original volume and the rotated volume,
    /// so it is guaranteed to be either the same size or larger than the original.
    fn rotated_by(mut self, rotation: impl Into<Self::Rotation>) -> Self {
        self.rotate_by(rotation);
        self
    }

    /// Rotates the bounding volume around the origin by the given rotation.
    ///
    /// The result is a combination of the original volume and the rotated volume,
    /// so it is guaranteed to be either the same size or larger than the original.
    fn rotate_by(&mut self, rotation: impl Into<Self::Rotation>);
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

mod raycast2d;
pub use raycast2d::*;
mod raycast3d;
pub use raycast3d::*;
