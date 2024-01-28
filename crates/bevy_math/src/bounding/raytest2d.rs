use super::{Aabb2d, BoundingCircle, IntersectsVolume};
use crate::{primitives::Direction2d, Ray2d, Vec2};

/// A raycast intersection test for 2D bounding volumes
#[derive(Debug)]
pub struct RayTest2d {
    /// The ray for the test
    pub ray: Ray2d,
    /// The maximum distance for the ray
    pub max: f32,
    /// The multiplicative inverse direction of the ray
    direction_recip: Vec2,
}

impl RayTest2d {
    /// Construct a [`RayTest2d`] from an origin, [`Direction2d`] and max distance.
    pub fn new(origin: Vec2, direction: Direction2d, max: f32) -> Self {
        Self::from_ray(Ray2d { origin, direction }, max)
    }

    /// Construct a [`RayTest2d`] from a [`Ray2d`] and max distance.
    pub fn from_ray(ray: Ray2d, max: f32) -> Self {
        Self {
            ray,
            direction_recip: ray.direction.recip(),
            max,
        }
    }

    /// Get the cached multiplicative inverse of the direction of the ray.
    pub fn direction_recip(&self) -> Vec2 {
        self.direction_recip
    }

    /// Get the distance of an intersection with an [`Aabb2d`], if any.
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

    /// Get the distance of an intersection with a [`BoundingCircle`], if any.
    pub fn circle_intersection_at(&self, circle: &BoundingCircle) -> Option<f32> {
        let offset = self.ray.origin - circle.center;
        let projected = offset.dot(*self.ray.direction);
        let closest_point = offset - projected * *self.ray.direction;
        let distance_squared = circle.radius().powi(2) - closest_point.length_squared();
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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.001;

    #[test]
    fn test_ray_intersection_circle_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of a centered bounding circle
                RayTest2d::new(Vec2::Y * -5., Direction2d::Y, 90.),
                BoundingCircle::new(Vec2::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of a centered bounding circle, but from the other side
                RayTest2d::new(Vec2::Y * 5., -Direction2d::Y, 90.),
                BoundingCircle::new(Vec2::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of an offset circle
                RayTest2d::new(Vec2::ZERO, Direction2d::Y, 90.),
                BoundingCircle::new(Vec2::Y * 3., 2.),
                1.,
            ),
            (
                // Just barely hit the circle before the max distance
                RayTest2d::new(Vec2::X, Direction2d::Y, 1.),
                BoundingCircle::new(Vec2::ONE, 0.01),
                0.99,
            ),
            (
                // Hit a circle off-center
                RayTest2d::new(Vec2::X, Direction2d::Y, 90.),
                BoundingCircle::new(Vec2::Y * 5., 2.),
                3.268,
            ),
            (
                // Barely hit a circle on the side
                RayTest2d::new(Vec2::X * 0.99999, Direction2d::Y, 90.),
                BoundingCircle::new(Vec2::Y * 5., 1.),
                4.996,
            ),
        ] {
            let case = format!(
                "Case:\n  Test: {:?}\n  Volume: {:?}\n  Expected distance: {:?}",
                test, volume, expected_distance
            );
            assert!(test.intersects(volume), "{}", case);
            let actual_distance = test.circle_intersection_at(volume).unwrap();
            assert!(
                (actual_distance - expected_distance).abs() < EPSILON,
                "{}\n  Actual distance: {}",
                case,
                actual_distance
            );

            let inverted_ray = RayTest2d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }

    #[test]
    fn test_ray_intersection_circle_misses() {
        for (test, volume) in &[
            (
                // The ray doesn't go in the right direction
                RayTest2d::new(Vec2::ZERO, Direction2d::X, 90.),
                BoundingCircle::new(Vec2::Y * 2., 1.),
            ),
            (
                // Ray's alignment isn't enough to hit the circle
                RayTest2d::new(Vec2::ZERO, Direction2d::from_xy(1., 1.).unwrap(), 90.),
                BoundingCircle::new(Vec2::Y * 2., 1.),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayTest2d::new(Vec2::ZERO, Direction2d::Y, 0.5),
                BoundingCircle::new(Vec2::Y * 2., 1.),
            ),
        ] {
            assert!(
                !test.intersects(volume),
                "Case:\n  Test: {:?}\n  Volume: {:?}",
                test,
                volume,
            );
        }
    }

    #[test]
    fn test_ray_intersection_circle_inside() {
        let volume = BoundingCircle::new(Vec2::splat(0.5), 1.);
        for origin in &[Vec2::X, Vec2::Y, Vec2::ONE, Vec2::ZERO] {
            for direction in &[
                Direction2d::X,
                Direction2d::Y,
                -Direction2d::X,
                -Direction2d::Y,
            ] {
                for max in &[0., 1., 900.] {
                    let test = RayTest2d::new(*origin, *direction, *max);

                    let case = format!(
                        "Case:\n  origin: {:?}\n  Direction: {:?}\n  Max: {}",
                        origin, direction, max,
                    );
                    assert!(test.intersects(&volume), "{}", case);

                    let actual_distance = test.circle_intersection_at(&volume);
                    assert_eq!(actual_distance, Some(0.), "{}", case,);
                }
            }
        }
    }

    #[test]
    fn test_ray_intersection_aabb_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of a centered aabb
                RayTest2d::new(Vec2::Y * -5., Direction2d::Y, 90.),
                Aabb2d::new(Vec2::ZERO, Vec2::ONE),
                4.,
            ),
            (
                // Hit the center of a centered aabb, but from the other side
                RayTest2d::new(Vec2::Y * 5., -Direction2d::Y, 90.),
                Aabb2d::new(Vec2::ZERO, Vec2::ONE),
                4.,
            ),
            (
                // Hit the center of an offset aabb
                RayTest2d::new(Vec2::ZERO, Direction2d::Y, 90.),
                Aabb2d::new(Vec2::Y * 3., Vec2::splat(2.)),
                1.,
            ),
            (
                // Just barely hit the aabb before the max distance
                RayTest2d::new(Vec2::X, Direction2d::Y, 1.),
                Aabb2d::new(Vec2::ONE, Vec2::splat(0.01)),
                0.99,
            ),
            (
                // Hit an aabb off-center
                RayTest2d::new(Vec2::X, Direction2d::Y, 90.),
                Aabb2d::new(Vec2::Y * 5., Vec2::splat(2.)),
                3.,
            ),
            (
                // Barely hit an aabb on corner
                RayTest2d::new(Vec2::X * -0.001, Direction2d::from_xy(1., 1.).unwrap(), 90.),
                Aabb2d::new(Vec2::Y * 2., Vec2::ONE),
                1.414,
            ),
        ] {
            let case = format!(
                "Case:\n  Test: {:?}\n  Volume: {:?}\n  Expected distance: {:?}",
                test, volume, expected_distance
            );
            assert!(test.intersects(volume), "{}", case);
            let actual_distance = test.aabb_intersection_at(volume).unwrap();
            assert!(
                (actual_distance - expected_distance).abs() < EPSILON,
                "{}\n  Actual distance: {}",
                case,
                actual_distance
            );

            let inverted_ray = RayTest2d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }

    #[test]
    fn test_ray_intersection_aabb_misses() {
        for (test, volume) in &[
            (
                // The ray doesn't go in the right direction
                RayTest2d::new(Vec2::ZERO, Direction2d::X, 90.),
                Aabb2d::new(Vec2::Y * 2., Vec2::ONE),
            ),
            (
                // Ray's alignment isn't enough to hit the aabb
                RayTest2d::new(Vec2::ZERO, Direction2d::from_xy(1., 0.99).unwrap(), 90.),
                Aabb2d::new(Vec2::Y * 2., Vec2::ONE),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayTest2d::new(Vec2::ZERO, Direction2d::Y, 0.5),
                Aabb2d::new(Vec2::Y * 2., Vec2::ONE),
            ),
        ] {
            assert!(
                !test.intersects(volume),
                "Case:\n  Test: {:?}\n  Volume: {:?}",
                test,
                volume,
            );
        }
    }

    #[test]
    fn test_ray_intersection_aabb_inside() {
        let volume = Aabb2d::new(Vec2::splat(0.5), Vec2::ONE);
        for origin in &[Vec2::X, Vec2::Y, Vec2::ONE, Vec2::ZERO] {
            for direction in &[
                Direction2d::X,
                Direction2d::Y,
                -Direction2d::X,
                -Direction2d::Y,
            ] {
                for max in &[0., 1., 900.] {
                    let test = RayTest2d::new(*origin, *direction, *max);

                    let case = format!(
                        "Case:\n  origin: {:?}\n  Direction: {:?}\n  Max: {}",
                        origin, direction, max,
                    );
                    assert!(test.intersects(&volume), "{}", case);

                    let actual_distance = test.aabb_intersection_at(&volume);
                    assert_eq!(actual_distance, Some(0.), "{}", case,);
                }
            }
        }
    }
}
