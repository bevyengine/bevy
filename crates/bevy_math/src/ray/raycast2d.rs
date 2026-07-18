use crate::{
    ops::{self, FloatPow},
    Dir2, Ray2d, Vec2,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// A raycast intersection test for 2D bounding volumes
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
pub struct RayCast2d {
    /// The ray for the test
    pub ray: Ray2d,
    /// The maximum distance for the ray
    pub max: f32,
    /// The multiplicative inverse direction of the ray
    direction_recip: Vec2,
}

impl RayCast2d {
    /// Construct a [`RayCast2d`] from an origin, [`Dir2`], and max distance.
    pub fn new(origin: Vec2, direction: Dir2, max: f32) -> Self {
        Self::from_ray(Ray2d { origin, direction }, max)
    }

    /// Construct a [`RayCast2d`] from a [`Ray2d`] and max distance.
    pub fn from_ray(ray: Ray2d, max: f32) -> Self {
        Self {
            ray,
            direction_recip: ray.direction.recip(),
            max,
        }
    }

    /// Get the cached multiplicative inverse of the direction of the ray.
    pub const fn direction_recip(&self) -> Vec2 {
        self.direction_recip
    }

    /// Get the distance of an intersection with an box defined by min/max, if any.
    pub fn aabb_intersection_at_min_max(&self, min: Vec2, max: Vec2) -> Option<f32> {
        let (min_x, max_x) = if self.ray.direction.x.is_sign_positive() {
            (min.x, max.x)
        } else {
            (max.x, min.x)
        };
        let (min_y, max_y) = if self.ray.direction.y.is_sign_positive() {
            (min.y, max.y)
        } else {
            (max.y, min.y)
        };

        // Calculate the minimum/maximum time for each axis based on how much the direction goes that
        // way. These values can get arbitrarily large, or even become NaN, which is handled by the
        // min/max operations below
        let tmin_x = (min_x - self.ray.origin.x) * self.direction_recip.x;
        let tmin_y = (min_y - self.ray.origin.y) * self.direction_recip.y;
        let tmax_x = (max_x - self.ray.origin.x) * self.direction_recip.x;
        let tmax_y = (max_y - self.ray.origin.y) * self.direction_recip.y;

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

    /// Get the distance of an intersection with a circle with `center` and `radius`, if any.
    pub fn circle_intersection_at_center_radius(&self, center: Vec2, radius: f32) -> Option<f32> {
        let offset = self.ray.origin - center;
        let projected = offset.dot(*self.ray.direction);
        let cross = offset.perp_dot(*self.ray.direction);
        let distance_squared = radius.squared() - cross.squared();
        if distance_squared < 0.
            || ops::copysign(projected.squared(), -projected) < -distance_squared
        {
            None
        } else {
            let toi = -projected - ops::sqrt(distance_squared);
            if toi > self.max {
                None
            } else {
                Some(toi.max(0.))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.001;

    #[test]
    fn test_ray_intersection_circle_hits() {
        for (test, (center, radius), expected_distance) in &[
            (
                // Hit the center of a centered bounding circle
                RayCast2d::new(Vec2::Y * -5., Dir2::Y, 90.),
                (Vec2::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of a centered bounding circle, but from the other side
                RayCast2d::new(Vec2::Y * 5., -Dir2::Y, 90.),
                (Vec2::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of an offset circle
                RayCast2d::new(Vec2::ZERO, Dir2::Y, 90.),
                (Vec2::Y * 3., 2.),
                1.,
            ),
            (
                // Just barely hit the circle before the max distance
                RayCast2d::new(Vec2::X, Dir2::Y, 1.),
                (Vec2::ONE, 0.01),
                0.99,
            ),
            (
                // Hit a circle off-center
                RayCast2d::new(Vec2::X, Dir2::Y, 90.),
                (Vec2::Y * 5., 2.),
                3.268,
            ),
            (
                // Barely hit a circle on the side
                RayCast2d::new(Vec2::X * 0.99999, Dir2::Y, 90.),
                (Vec2::Y * 5., 1.),
                4.996,
            ),
        ] {
            let actual_distance = test
                .circle_intersection_at_center_radius(*center, *radius)
                .unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: center: {center:?}, radius: {radius}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );
        }
    }

    #[test]
    fn test_ray_intersection_circle_inside() {
        let (center, radius) = (Vec2::splat(0.5), 1.);
        for origin in &[Vec2::X, Vec2::Y, Vec2::ONE, Vec2::ZERO] {
            for direction in &[Dir2::X, Dir2::Y, -Dir2::X, -Dir2::Y] {
                for max in &[0., 1., 900.] {
                    let test = RayCast2d::new(*origin, *direction, *max);

                    let actual_distance = test.circle_intersection_at_center_radius(center, radius);
                    assert_eq!(
                        actual_distance,
                        Some(0.),
                        "Case:\n  origin: {origin:?}\n  Direction: {direction:?}\n  Max: {max}",
                    );
                }
            }
        }
    }

    #[test]
    fn test_ray_intersection_aabb_hits() {
        for (test, (min, max), expected_distance) in &[
            (
                // Hit the center of a centered aabb
                RayCast2d::new(Vec2::Y * -5., Dir2::Y, 90.),
                (Vec2::ZERO, Vec2::ONE),
                4.,
            ),
            (
                // Hit the center of a centered aabb, but from the other side
                RayCast2d::new(Vec2::Y * 5., -Dir2::Y, 90.),
                (Vec2::ZERO, Vec2::ONE),
                4.,
            ),
            (
                // Hit the center of an offset aabb
                RayCast2d::new(Vec2::ZERO, Dir2::Y, 90.),
                (Vec2::Y * 3., Vec2::splat(2.)),
                1.,
            ),
            (
                // Just barely hit the aabb before the max distance
                RayCast2d::new(Vec2::X, Dir2::Y, 1.),
                (Vec2::ONE, Vec2::splat(0.01)),
                0.99,
            ),
            (
                // Hit an aabb off-center
                RayCast2d::new(Vec2::X, Dir2::Y, 90.),
                (Vec2::Y * 5., Vec2::splat(2.)),
                3.,
            ),
            (
                // Barely hit an aabb on corner
                RayCast2d::new(Vec2::X * -0.001, Dir2::from_xy(1., 1.).unwrap(), 90.),
                (Vec2::Y * 2., Vec2::ONE),
                1.414,
            ),
        ]
        .map(|(a, (center, half_size), b)| (a, (center - half_size, center + half_size), b))
        {
            let actual_distance = test.aabb_intersection_at_min_max(*min, *max).unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: min: {min:?}, max: {max:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );
        }
    }

    #[test]
    fn test_ray_intersection_aabb_inside() {
        let (min, max) = (Vec2::splat(0.5) - Vec2::ONE, Vec2::splat(0.5) + Vec2::ONE);
        for origin in &[Vec2::X, Vec2::Y, Vec2::ONE, Vec2::ZERO] {
            for direction in &[Dir2::X, Dir2::Y, -Dir2::X, -Dir2::Y] {
                for max_dist in &[0., 1., 900.] {
                    let test = RayCast2d::new(*origin, *direction, *max_dist);

                    let actual_distance = test.aabb_intersection_at_min_max(min, max);
                    assert_eq!(
                        actual_distance,
                        Some(0.),
                        "Case:\n  origin: {origin:?}\n  Direction: {direction:?}\n  Max: {max}",
                    );
                }
            }
        }
    }
}
