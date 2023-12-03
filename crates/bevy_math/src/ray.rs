use crate::{
    primitives::{Direction2d, Direction3d, Plane2d, Plane3d},
    Vec2, Vec3,
};

/// An infinite half-line starting at `origin` and going in `direction` in 2D space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Ray2d {
    /// The origin of the ray.
    pub origin: Vec2,
    /// The direction of the ray.
    pub direction: Direction2d,
}

impl Ray2d {
    /// Create a new `Ray2d` from a given origin and direction
    #[inline]
    pub fn new(origin: Vec2, direction: Vec2) -> Self {
        Self {
            origin,
            direction: direction.into(),
        }
    }

    /// Get a point at a given distance along the ray
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec2 {
        self.origin + *self.direction * distance
    }

    /// Get the distance to a plane if the ray intersects it
    #[inline]
    pub fn intersect_plane(&self, plane_origin: Vec2, plane: impl Into<Plane2d>) -> Option<f32> {
        let plane = plane.into();
        let denominator = plane.normal.dot(*self.direction);
        if denominator.abs() > f32::EPSILON {
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
pub struct Ray3d {
    /// The origin of the ray.
    pub origin: Vec3,
    /// The direction of the ray.
    pub direction: Direction3d,
}

impl Ray3d {
    /// Create a new `Ray3d` from a given origin and direction
    #[inline]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.into(),
        }
    }

    /// Get a point at a given distance along the ray
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec3 {
        self.origin + *self.direction * distance
    }

    /// Get the distance to a plane if the ray intersects it
    #[inline]
    pub fn intersect_plane(&self, plane_origin: Vec3, plane: impl Into<Plane3d>) -> Option<f32> {
        let plane = plane.into();
        let denominator = plane.normal.dot(*self.direction);
        if denominator.abs() > f32::EPSILON {
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
        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);

        // Orthogonal, and test that an inverse plane_normal has the same result
        assert_eq!(ray.intersect_plane(Vec2::Y, Vec2::Y), Some(1.0),);
        assert_eq!(ray.intersect_plane(Vec2::NEG_Y, Vec2::Y), Some(1.0),);
        assert!(ray.intersect_plane(Vec2::Y, Vec2::NEG_Y).is_none());
        assert!(ray.intersect_plane(Vec2::NEG_Y, Vec2::NEG_Y).is_none());

        // Diagonal
        assert_eq!(ray.intersect_plane(Vec2::ONE, Vec2::Y), Some(1.0),);
        assert!(ray.intersect_plane(Vec2::ONE, Vec2::NEG_Y).is_none());

        // Parallel
        assert!(ray.intersect_plane(Vec2::X, Vec2::X).is_none());

        // Parallel with simulated rounding error
        assert!(ray
            .intersect_plane(Vec2::X + Vec2::Y * f32::EPSILON, Vec2::X)
            .is_none());
    }

    #[test]
    fn intersect_plane_3d() {
        let ray = Ray3d::new(Vec3::ZERO, Vec3::Y);

        // Orthogonal, and test that an inverse plane_normal has the same result
        assert_eq!(ray.intersect_plane(Vec3::Y, Vec3::Y), Some(1.0),);
        assert_eq!(ray.intersect_plane(Vec3::NEG_Y, Vec3::Y), Some(1.0),);
        assert!(ray.intersect_plane(Vec3::Y, Vec3::NEG_Y).is_none());
        assert!(ray.intersect_plane(Vec3::NEG_Y, Vec3::NEG_Y).is_none());

        // Diagonal
        assert_eq!(ray.intersect_plane(Vec3::ONE, Vec3::Y), Some(1.0),);
        assert!(ray.intersect_plane(Vec3::ONE, Vec3::NEG_Y).is_none());

        // Parallel
        assert!(ray.intersect_plane(Vec3::X, Vec3::X).is_none());

        // Parallel with simulated rounding error
        assert!(ray
            .intersect_plane(Vec3::X + Vec3::Y * f32::EPSILON, Vec3::X)
            .is_none());
    }
}
