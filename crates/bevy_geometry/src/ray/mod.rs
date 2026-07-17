pub(crate) mod raycast2d;
pub use raycast2d::*;
pub(crate) mod raycast3d;
pub use raycast3d::*;

use bevy_math::{Ray2d, Ray3d, Vec2, Vec3};
use bevy_shape::{InfinitePlane3d, Plane2d};

pub trait Ray2dIntersection {
    /// Returns the distance to a [`bevy_shape::Plane2d`] if the ray intersects it
    ///
    /// Use [`Ray2d::plane_intersection_point`] to get the intersection point directly.
    fn intersect_plane(&self, plane_origin: Vec2, plane: Plane2d) -> Option<f32>;

    /// Returns the intersection point of the ray with a plane, if it exists.
    ///
    /// Calls [`Ray2d::get_point`] on the result of [`Ray2d::intersect_plane`].
    fn plane_intersection_point(&self, plane_origin: Vec2, plane: Plane2d) -> Option<Vec2>;
}

impl Ray2dIntersection for Ray2d {
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

pub trait Ray3dIntersection {
    /// Returns the distance to a [`bevy_shape::InfinitePlane3d`] if the ray intersects it
    ///
    /// Use [`Ray3d::plane_intersection_point`] to get the intersection point directly.
    fn intersect_plane(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<f32>;

    /// Returns the intersection point of the ray with a plane, if it exists.
    ///
    /// Calls [`Ray3d::get_point`] on the result of [`Ray3d::intersect_plane`].
    fn plane_intersection_point(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<Vec3>;
}

impl Ray3dIntersection for Ray3d {
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
