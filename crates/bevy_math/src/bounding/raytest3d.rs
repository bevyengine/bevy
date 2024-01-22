use super::{Aabb3d, BoundingSphere, IntersectsVolume};
use crate::{primitives::Direction3d, Ray3d, Vec3};

/// A raycast intersection test for 3D bounding volumes
pub struct RayTest3d {
    /// The ray for the test
    pub ray: Ray3d,
    /// The maximum time of impact of the ray
    pub max: f32,
    /// The multiplicative inverse direction of the ray
    dir_recip: Vec3,
}

impl RayTest3d {
    /// Construct a [`RayTest3d`] from an origin, [`Direction3d`] and max time of impact.
    pub fn new(origin: Vec3, direction: Direction3d, max: f32) -> Self {
        Self::from_ray(Ray3d { origin, direction }, max)
    }

    /// Construct a [`RayTest3d`] from a [`Ray3d`] and max time of impact.
    pub fn from_ray(ray: Ray3d, max: f32) -> Self {
        Self {
            ray,
            dir_recip: ray.direction.recip(),
            max,
        }
    }

    /// Get the cached multiplicate inverse of the direction of the ray
    pub fn dir_recip(&self) -> Vec3 {
        self.dir_recip
    }

    /// Get the time of impact for an intersection with an [`Aabb3d`], if any.
    pub fn aabb_intersection_at(&self, aabb: &Aabb3d) -> Option<f32> {
        let (min_x, max_x) = if self.ray.direction.x.is_sign_positive() {
            (aabb.min.x, aabb.max.x)
        } else {
            (aabb.max.x, aabb.min.x)
        };
        let (min_y, max_y) = if self.ray.direction.y.is_sign_positive() {
            (aabb.min.y, aabb.max.y)
        } else {
            (aabb.max.y, aabb.min.y)
        };
        let (min_z, max_z) = if self.ray.direction.z.is_sign_positive() {
            (aabb.min.z, aabb.max.z)
        } else {
            (aabb.max.z, aabb.min.z)
        };

        // Calculate the minimum/maximum time for each based on how much the direction goes that
        // way. These values van get arbitrarily large, or even become NaN, which is handled by the
        // min/max operations below
        let tmin_x = (min_x - self.ray.origin.x) * self.dir_recip.x;
        let tmin_y = (min_y - self.ray.origin.y) * self.dir_recip.y;
        let tmin_z = (min_z - self.ray.origin.z) * self.dir_recip.z;
        let tmax_x = (max_x - self.ray.origin.x) * self.dir_recip.x;
        let tmax_y = (max_y - self.ray.origin.y) * self.dir_recip.y;
        let tmax_z = (max_z - self.ray.origin.z) * self.dir_recip.z;

        // An axis that is not relevant to the ray direction will be NaN. When one of the arguments
        // to min/max is NaN, the other argument is used.
        // An axis for which the direction is the wrong way will return an arbitrarily large
        // negative value.
        let tmin = tmin_x.max(tmin_y).max(tmin_z).max(0.);
        let tmax = tmax_z.min(tmax_y).min(tmax_x).min(self.max);

        if tmin <= tmax {
            Some(tmin)
        } else {
            None
        }
    }

    /// Get the time of impact for an intersection with a [`BoundingSphere`], if any.
    pub fn sphere_intersection_at(&self, sphere: &BoundingSphere) -> Option<f32> {
        let offset = self.ray.origin - sphere.center;
        let projected = offset.dot(*self.ray.direction);
        let closest_point = offset - projected * *self.ray.direction;
        let distance_squared = sphere.radius().powi(2) - closest_point.length_squared();
        if distance_squared < 0. || projected.powi(2).copysign(-projected) < distance_squared {
            None
        } else {
            Some(-projected - distance_squared.sqrt())
        }
    }
}

impl IntersectsVolume<Aabb3d> for RayTest3d {
    fn intersects(&self, volume: &Aabb3d) -> bool {
        self.aabb_intersection_at(volume).is_some()
    }
}

impl IntersectsVolume<BoundingSphere> for RayTest3d {
    fn intersects(&self, volume: &BoundingSphere) -> bool {
        self.sphere_intersection_at(volume).is_some()
    }
}
