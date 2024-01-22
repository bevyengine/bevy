use super::{Aabb2d, BoundingCircle, IntersectsVolume};
use crate::{primitives::Direction2d, Vec2};

/// A raycast intersection test for 2D bounding volumes
pub struct RayTest2d {
    /// The origin of the ray
    pub origin: Vec2,
    /// The direction of the ray
    pub dir: Direction2d,
    /// The multiplicative inverse direction of the ray
    pub dir_recip: Vec2,
    /// The maximum time of impact of the ray
    pub max: f32,
}

impl RayTest2d {
    /// Construct a [`RayTest2d`] from an origin, [`Direction2d`] and max time of impact.
    pub fn new(origin: Vec2, dir: Direction2d, max: f32) -> Self {
        Self {
            origin,
            dir_recip: dir.recip(),
            dir,
            max,
        }
    }

    /// Get the time of impact for an intersection with an [`Aabb2d`], if any.
    pub fn aabb_intersection_at(&self, aabb: &Aabb2d) -> Option<f32> {
        let (min_x, max_x) = if self.dir.x.is_sign_positive() {
            (aabb.min.x, aabb.max.x)
        } else {
            (aabb.max.x, aabb.min.x)
        };
        let (min_y, max_y) = if self.dir.y.is_sign_positive() {
            (aabb.min.y, aabb.max.y)
        } else {
            (aabb.max.y, aabb.min.y)
        };
        let tmin_x = (min_x - self.origin.x) * self.dir_recip.x;
        let tmin_y = (min_y - self.origin.y) * self.dir_recip.y;
        let tmax_x = (max_x - self.origin.x) * self.dir_recip.x;
        let tmax_y = (max_y - self.origin.y) * self.dir_recip.y;

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
        let offset = self.origin - sphere.center;
        let projected = offset.dot(*self.dir);
        let closest_point = offset - projected * *self.dir;
        let distance_squared = sphere.radius().powi(2) - closest_point.length_squared();
        if distance_squared < 0. || projected.powi(2).copysign(-projected) < distance_squared {
            None
        } else {
            Some(-projected - distance_squared.sqrt())
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
