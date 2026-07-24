//! Ray intersection utilities.
//!
//! This module extends [`Ray2d`] and [`Ray3d`] with convenience methods for intersecting planes and
//! obtaining either the distance to the first intersection or the intersection point directly.
//!
//! It also re-exports ray casting utilities for intersecting rays with supported geometric
//! primitives from [`bevy_shape`] via [`bevy_math::RayCast2d`] and [`bevy_math::RayCast3d`].

pub(crate) mod raycast2d;
pub use raycast2d::*;
pub(crate) mod raycast3d;
pub use raycast3d::*;

use bevy_math::{Ray2d, Ray3d, Vec2, Vec3};
use bevy_shape::{InfinitePlane3d, Plane2d};

/// Computes intersections with planes.
///
/// Implementors return the distance from the ray's origin to the intersection
/// with a plane, or the intersection point directly.
pub trait Ray2dIntersectionExt {
    /// Returns the distance to a [`bevy_shape::Plane2d`] if the ray intersects it
    ///
    /// Use [`Ray2d::plane_intersection_point`] to get the intersection point directly.
    fn intersect_plane(&self, plane_origin: Vec2, plane: Plane2d) -> Option<f32>;

    /// Returns the intersection point of the ray with a plane, if it exists.
    ///
    /// Calls [`Ray2d::get_point`] on the result of [`Ray2d::intersect_plane`].
    fn plane_intersection_point(&self, plane_origin: Vec2, plane: Plane2d) -> Option<Vec2>;
}

impl Ray2dIntersectionExt for Ray2d {
    #[inline]
    fn intersect_plane(&self, plane_origin: Vec2, plane: Plane2d) -> Option<f32> {
        self.intersect_plane_normal(plane_origin, plane.normal)
    }

    #[inline]
    fn plane_intersection_point(&self, plane_origin: Vec2, plane: Plane2d) -> Option<Vec2> {
        self.intersect_plane(plane_origin, plane)
            .map(|distance| self.get_point(distance))
    }
}

/// Computes intersections with planes.
///
/// Implementors return the distance from the ray's origin to the intersection
/// with a plane, or the intersection point directly.
pub trait Ray3dIntersectionExt {
    /// Returns the distance to a [`bevy_shape::InfinitePlane3d`] if the ray intersects it
    ///
    /// Use [`Ray3d::plane_intersection_point`] to get the intersection point directly.
    fn intersect_plane(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<f32>;

    /// Returns the intersection point of the ray with a plane, if it exists.
    ///
    /// Calls [`Ray3d::get_point`] on the result of [`Ray3d::intersect_plane`].
    fn plane_intersection_point(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<Vec3>;
}

impl Ray3dIntersectionExt for Ray3d {
    #[inline]
    fn intersect_plane(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<f32> {
        self.intersect_plane_normal(plane_origin, plane.normal)
    }

    #[inline]
    fn plane_intersection_point(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<Vec3> {
        self.intersect_plane(plane_origin, plane)
            .map(|distance| self.get_point(distance))
    }
}
