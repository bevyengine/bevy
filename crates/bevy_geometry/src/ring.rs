use bevy_math::{Isometry2d, Isometry3d};
use bevy_shape::Primitive2d;

use crate::{
    bounding::{Aabb2d, Aabb3d, Bounded2d, BoundedExtrusion, BoundingCircle, BoundingSphere},
    inset::Inset,
    measured::Measured2d,
};

/// A 2D shape representing the ring version of a base shape.
///
/// The `inner_shape` forms the "hollow" of the `outer_shape`.
///
/// The resulting shapes are rings or hollow shapes.
/// For example, a circle becomes an annulus.
///
/// # Warning
///
/// The `outer_shape` must contain the `inner_shape` for the generated meshes to be accurate.
///
/// If there are vertices in the `inner_shape` that escape the `outer_shape`
/// (for example, if the `inner_shape` is in fact larger),
/// it may result in incorrect geometries.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Ring<P: Primitive2d> {
    /// The outer shape
    pub outer_shape: P,
    /// The inner shape (the same shape of a different size)
    pub inner_shape: P,
}

impl<P: Primitive2d> Ring<P> {
    /// Create a new `Ring` from a given `outer_shape` and `inner_shape`.
    ///
    /// If the primitive implements [`Inset`] and you would like a uniform thickness, consider using [`ToRing::to_ring`]
    pub const fn new(outer_shape: P, inner_shape: P) -> Self {
        Self {
            outer_shape,
            inner_shape,
        }
    }
}

impl<T: Primitive2d> Primitive2d for Ring<T> {}

impl<P: Primitive2d + Clone + Inset> Ring<P> {
    /// Generate a `Ring` from a given `primitive` and a `thickness`.
    pub fn from_primitive_and_thickness(primitive: P, thickness: f32) -> Self {
        let hollow = primitive.clone().inset(thickness);
        Ring::new(primitive, hollow)
    }
}

impl<P: Primitive2d + Measured2d> Measured2d for Ring<P> {
    #[inline]
    fn area(&self) -> f32 {
        self.outer_shape.area() - self.inner_shape.area()
    }

    #[inline]
    fn perimeter(&self) -> f32 {
        self.outer_shape.perimeter() + self.inner_shape.perimeter()
    }
}

/// Provides a convenience method for converting a primitive to a [`Ring`], with a given thickness.
///
/// The primitive must implement [`Inset`].
pub trait ToRing: Primitive2d + Inset
where
    Self: Sized,
{
    /// Construct a `Ring`
    fn to_ring(self, thickness: f32) -> Ring<Self>;
}

impl<P> ToRing for P
where
    P: Primitive2d + Clone + Inset,
{
    fn to_ring(self, thickness: f32) -> Ring<Self> {
        Ring::from_primitive_and_thickness(self, thickness)
    }
}

impl<P: Bounded2d + Primitive2d> Bounded2d for Ring<P> {
    fn aabb_2d(&self, isometry: impl Into<Isometry2d>) -> Aabb2d {
        self.outer_shape.aabb_2d(isometry)
    }

    fn bounding_circle(&self, isometry: impl Into<Isometry2d>) -> BoundingCircle {
        self.outer_shape.bounding_circle(isometry)
    }
}

impl<T: BoundedExtrusion> BoundedExtrusion for Ring<T> {
    fn extrusion_aabb_3d(&self, half_depth: f32, isometry: impl Into<Isometry3d>) -> Aabb3d {
        self.outer_shape.extrusion_aabb_3d(half_depth, isometry)
    }

    fn extrusion_bounding_sphere(
        &self,
        half_depth: f32,
        isometry: impl Into<Isometry3d>,
    ) -> BoundingSphere {
        self.outer_shape
            .extrusion_bounding_sphere(half_depth, isometry)
    }
}
