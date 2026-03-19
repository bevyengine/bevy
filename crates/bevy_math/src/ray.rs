use crate::{
    ops,
    primitives::{InfinitePlane3d, Plane2d},
    Dir2, Dir3, Vec2, Vec3,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// An infinite half-line starting at `origin` and going in `direction` in 2D space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub struct Ray2d {
    /// The origin of the ray.
    pub origin: Vec2,
    /// The direction of the ray.
    pub direction: Dir2,
}

impl Ray2d {
    /// Creates a new `Ray2d` from a given origin and direction
    #[inline]
    pub const fn new(origin: Vec2, direction: Dir2) -> Self {
        Self { origin, direction }
    }

    /// Returns the point at a given distance along the ray.
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec2 {
        self.origin + *self.direction * distance
    }

    /// Returns the distance to a plane if the ray intersects it.
    ///
    /// Use [`Ray2d::plane_intersection_point`] to get the intersection point directly.
    #[inline]
    pub fn intersect_plane(&self, plane_origin: Vec2, plane: Plane2d) -> Option<f32> {
        let denominator = plane.normal.dot(*self.direction);
        if ops::abs(denominator) > f32::EPSILON {
            let distance = (plane_origin - self.origin).dot(*plane.normal) / denominator;
            if distance > f32::EPSILON {
                return Some(distance);
            }
        }
        None
    }

    /// Returns the intersection point with a plane, if it exists.
    ///
    /// Calls [`Ray2d::get_point`] on the result of [`Ray2d::intersect_plane`].
    #[inline]
    pub fn plane_intersection_point(&self, plane_origin: Vec2, plane: Plane2d) -> Option<Vec2> {
        self.intersect_plane(plane_origin, plane)
            .map(|distance| self.get_point(distance))
    }
}

/// An infinite half-line starting at `origin` and going in `direction` in 3D space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub struct Ray3d {
    /// The origin of the ray.
    pub origin: Vec3,
    /// The direction of the ray.
    pub direction: Dir3,
}

impl Ray3d {
    /// Creates a new `Ray3d` from a given origin and direction
    #[inline]
    pub const fn new(origin: Vec3, direction: Dir3) -> Self {
        Self { origin, direction }
    }

    /// Returns the point at a given distance along the ray
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec3 {
        self.origin + *self.direction * distance
    }

    /// Returns the distance to a plane if the ray intersects it
    ///
    /// Use [`Ray3d::plane_intersection_point`] to get the intersection point directly.
    #[inline]
    pub fn intersect_plane(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<f32> {
        let normal = Vec3::from(plane.normal().as_vec3a());
        let denominator = normal.dot(*self.direction);
        if ops::abs(denominator) > f32::EPSILON {
            let distance = (plane_origin - self.origin).dot(normal) / denominator;
            if distance > f32::EPSILON {
                return Some(distance);
            }
        }
        None
    }

    /// Returns the intersection point of the ray with a plane, if it exists.
    ///
    /// Calls [`Ray3d::get_point`] on the result of [`Ray3d::intersect_plane`].
    #[inline]
    pub fn plane_intersection_point(
        &self,
        plane_origin: Vec3,
        plane: InfinitePlane3d,
    ) -> Option<Vec3> {
        self.intersect_plane(plane_origin, plane)
            .map(|distance| self.get_point(distance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dir3A;

    #[test]
    fn intersect_plane_2d() {
        let ray = Ray2d::new(Vec2::ZERO, Dir2::Y);

        // Orthogonal, and test that an inverse plane_normal has the same result
        assert_eq!(
            ray.intersect_plane(Vec2::Y, Plane2d::new(Dir2::Y, 0.0)),
            Some(1.0)
        );
        assert_eq!(
            ray.intersect_plane(Vec2::Y, Plane2d::new(Dir2::NEG_Y, 0.0)),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(Vec2::NEG_Y, Plane2d::new(Dir2::Y, 0.0))
            .is_none());
        assert!(ray
            .intersect_plane(Vec2::NEG_Y, Plane2d::new(Dir2::NEG_Y, 0.0))
            .is_none());

        // Diagonal
        assert_eq!(
            ray.intersect_plane(Vec2::Y, Plane2d::new(Dir2::from_xy(1.0, 1.0).unwrap(), 0.0)),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(
                Vec2::NEG_Y,
                Plane2d::new(Dir2::from_xy(1.0, 1.0).unwrap(), 0.0)
            )
            .is_none());

        // Parallel
        assert!(ray
            .intersect_plane(Vec2::X, Plane2d::new(Dir2::X, 0.0))
            .is_none());

        // Parallel with simulated rounding error
        assert!(ray
            .intersect_plane(
                Vec2::X,
                Plane2d::new(Dir2::from_xy(1.0, f32::EPSILON).unwrap(), 0.0)
            )
            .is_none());
    }

    #[test]
    fn intersect_plane_3d() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::Z);

        // Orthogonal, and test that an inverse plane_normal has the same result
        assert_eq!(
            ray.intersect_plane(Vec3::Z, InfinitePlane3d::new(Dir3A::Z, 0.0)),
            Some(1.0)
        );
        assert_eq!(
            ray.intersect_plane(Vec3::Z, InfinitePlane3d::new(Dir3A::NEG_Z, 0.0)),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(Vec3::NEG_Z, InfinitePlane3d::new(Dir3A::Z, 0.0))
            .is_none());
        assert!(ray
            .intersect_plane(Vec3::NEG_Z, InfinitePlane3d::new(Dir3A::NEG_Z, 0.0))
            .is_none());

        // Diagonal
        assert_eq!(
            ray.intersect_plane(
                Vec3::Z,
                InfinitePlane3d::new(Dir3A::from_xyz(1.0, 1.0, 1.0).unwrap(), 0.0)
            ),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(
                Vec3::NEG_Z,
                InfinitePlane3d::new(Dir3A::from_xyz(1.0, 1.0, 1.0).unwrap(), 0.0)
            )
            .is_none());

        // Parallel
        assert!(ray
            .intersect_plane(Vec3::X, InfinitePlane3d::new(Dir3A::X, 0.0))
            .is_none());

        // Parallel with simulated rounding error
        assert!(ray
            .intersect_plane(
                Vec3::X,
                InfinitePlane3d::new(Dir3A::from_xyz(1.0, 0.0, f32::EPSILON).unwrap(), 0.0)
            )
            .is_none());
    }
}
