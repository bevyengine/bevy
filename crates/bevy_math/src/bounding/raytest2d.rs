use super::{Aabb2d, BoundingCircle, IntersectsVolume};
use crate::{primitives::Direction2d, Ray2d, Vec2};

/// A raycast intersection test for 2D bounding volumes
pub struct RayTest2d {
    /// The ray for the test
    pub ray: Ray2d,
    /// The maximum time of impact of the ray
    pub max: f32,
    /// The multiplicative inverse direction of the ray
    dir_recip: Vec2,
}

impl RayTest2d {
    /// Construct a [`RayTest2d`] from an origin, [`Direction2d`] and max time of impact.
    pub fn new(origin: Vec2, direction: Direction2d, max: f32) -> Self {
        Self::from_ray(Ray2d { origin, direction }, max)
    }

    /// Construct a [`RayTest2d`] from a [`Ray2d`] and max time of impact.
    pub fn from_ray(ray: Ray2d, max: f32) -> Self {
        Self {
            ray,
            dir_recip: ray.direction.recip(),
            max,
        }
    }

    /// Get the cached multiplicate inverse of the direction of the ray
    pub fn dir_recip(&self) -> Vec2 {
        self.dir_recip
    }

    /// Get the time of impact for an intersection with an [`Aabb2d`], if any.
    pub fn aabb_intersection_at(&self, aabb: &Aabb2d) -> Option<f32> {
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

        // Calculate the minimum/maximum time for each based on how much the direction goes that
        // way. These values van get arbitrarily large, or even become NaN, which is handled by the
        // min/max operations below
        let tmin_x = (min_x - self.ray.origin.x) * self.dir_recip.x;
        let tmin_y = (min_y - self.ray.origin.y) * self.dir_recip.y;
        let tmax_x = (max_x - self.ray.origin.x) * self.dir_recip.x;
        let tmax_y = (max_y - self.ray.origin.y) * self.dir_recip.y;

        // An axis that is not relevant to the ray direction will be NaN. When one of the arguments
        // to min/max is NaN, the other argument is used.
        // An axis for which the direction is the wrong way will return an arbitrarily large
        // negative value.
        let tmin = tmin_x.max(tmin_y).max(0.);
        let tmax = tmax_y.min(tmax_x).min(self.max);

        if tmin <= tmax {
            Some(tmin)
        } else {
            None
        }
    }

    /// Get the time of impact for an intersection with a [`BoundingCircle`], if any.
    pub fn circle_intersection_at(&self, sphere: &BoundingCircle) -> Option<f32> {
        let offset = self.ray.origin - sphere.center;
        let projected = offset.dot(*self.ray.direction);
        let closest_point = offset - projected * *self.ray.direction;
        let distance_squared = sphere.radius().powi(2) - closest_point.length_squared();
        if distance_squared < 0. || projected.powi(2).copysign(-projected) < -distance_squared {
            None
        } else {
            let toi = -projected - distance_squared.sqrt();
            if toi > self.max {
                None
            } else {
                Some(toi.max(0.))
            }
        }
    }
}

impl IntersectsVolume<Aabb2d> for RayTest2d {
    fn intersects(&self, volume: &Aabb2d) -> bool {
        self.aabb_intersection_at(volume).is_some()
    }
}

impl IntersectsVolume<BoundingCircle> for RayTest2d {
    fn intersects(&self, volume: &BoundingCircle) -> bool {
        self.circle_intersection_at(volume).is_some()
    }
}
