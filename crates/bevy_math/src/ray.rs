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
    /// Create a new `Ray2d` from a given origin and direction
    #[inline]
    pub const fn new(origin: Vec2, direction: Dir2) -> Self {
        Self { origin, direction }
    }

    /// Get a point at a given distance along the ray
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec2 {
        self.origin + *self.direction * distance
    }

    /// Get the distance to a plane if the ray intersects it
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
    /// Create a new `Ray3d` from a given origin and direction
    #[inline]
    pub const fn new(origin: Vec3, direction: Dir3) -> Self {
        Self { origin, direction }
    }

    /// Get a point at a given distance along the ray
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec3 {
        self.origin + *self.direction * distance
    }

    /// Get the distance to a plane if the ray intersects it
    #[inline]
    pub fn intersect_plane(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<f32> {
        let denominator = plane.normal.dot(*self.direction);
        if ops::abs(denominator) > f32::EPSILON {
            let distance = (plane_origin - self.origin).dot(*plane.normal) / denominator;
            if distance > f32::EPSILON {
                return Some(distance);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_plane_2d() {
        let ray = Ray2d::new(Vec2::ZERO, Dir2::Y);

        // Orthogonal, and test that an inverse plane_normal has the same result
        assert_eq!(
            ray.intersect_plane(Vec2::Y, Plane2d::new(Vec2::Y)),
            Some(1.0)
        );
        assert_eq!(
            ray.intersect_plane(Vec2::Y, Plane2d::new(Vec2::NEG_Y)),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(Vec2::NEG_Y, Plane2d::new(Vec2::Y))
            .is_none());
        assert!(ray
            .intersect_plane(Vec2::NEG_Y, Plane2d::new(Vec2::NEG_Y))
            .is_none());

        // Diagonal
        assert_eq!(
            ray.intersect_plane(Vec2::Y, Plane2d::new(Vec2::ONE)),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(Vec2::NEG_Y, Plane2d::new(Vec2::ONE))
            .is_none());

        // Parallel
        assert!(ray
            .intersect_plane(Vec2::X, Plane2d::new(Vec2::X))
            .is_none());

        // Parallel with simulated rounding error
        assert!(ray
            .intersect_plane(Vec2::X, Plane2d::new(Vec2::X + Vec2::Y * f32::EPSILON))
            .is_none());
    }

    #[test]
    fn intersect_plane_3d() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::Z);

        // Orthogonal, and test that an inverse plane_normal has the same result
        assert_eq!(
            ray.intersect_plane(Vec3::Z, InfinitePlane3d::new(Vec3::Z)),
            Some(1.0)
        );
        assert_eq!(
            ray.intersect_plane(Vec3::Z, InfinitePlane3d::new(Vec3::NEG_Z)),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(Vec3::NEG_Z, InfinitePlane3d::new(Vec3::Z))
            .is_none());
        assert!(ray
            .intersect_plane(Vec3::NEG_Z, InfinitePlane3d::new(Vec3::NEG_Z))
            .is_none());

        // Diagonal
        assert_eq!(
            ray.intersect_plane(Vec3::Z, InfinitePlane3d::new(Vec3::ONE)),
            Some(1.0)
        );
        assert!(ray
            .intersect_plane(Vec3::NEG_Z, InfinitePlane3d::new(Vec3::ONE))
            .is_none());

        // Parallel
        assert!(ray
            .intersect_plane(Vec3::X, InfinitePlane3d::new(Vec3::X))
            .is_none());

        // Parallel with simulated rounding error
        assert!(ray
            .intersect_plane(
                Vec3::X,
                InfinitePlane3d::new(Vec3::X + Vec3::Z * f32::EPSILON)
            )
            .is_none());
    }
}
