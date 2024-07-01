use super::{Aabb2d, BoundingCircle, IntersectsVolume};
use crate::{Dir2, Ray2d, Vec2};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// A raycast intersection test for 2D bounding volumes
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
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

impl IntersectsVolume<Aabb2d> for RayCast2d {
    fn intersects(&self, volume: &Aabb2d) -> bool {
        self.aabb_intersection_at(volume).is_some()
    }
}

impl IntersectsVolume<BoundingCircle> for RayCast2d {
    fn intersects(&self, volume: &BoundingCircle) -> bool {
        self.circle_intersection_at(volume).is_some()
    }
}

/// An intersection test that casts an [`Aabb2d`] along a ray.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct AabbCast2d {
    /// The ray along which to cast the bounding volume
    pub ray: RayCast2d,
    /// The aabb that is being cast
    pub aabb: Aabb2d,
}

impl AabbCast2d {
    /// Construct an [`AabbCast2d`] from an [`Aabb2d`], origin, [`Dir2`], and max distance.
    pub fn new(aabb: Aabb2d, origin: Vec2, direction: Dir2, max: f32) -> Self {
        Self::from_ray(aabb, Ray2d { origin, direction }, max)
    }

    /// Construct an [`AabbCast2d`] from an [`Aabb2d`], [`Ray2d`], and max distance.
    pub fn from_ray(aabb: Aabb2d, ray: Ray2d, max: f32) -> Self {
        Self {
            ray: RayCast2d::from_ray(ray, max),
            aabb,
        }
    }

    /// Get the distance at which the [`Aabb2d`]s collide, if at all.
    pub fn aabb_collision_at(&self, mut aabb: Aabb2d) -> Option<f32> {
        aabb.min -= self.aabb.max;
        aabb.max -= self.aabb.min;
        self.ray.aabb_intersection_at(&aabb)
    }
}

impl IntersectsVolume<Aabb2d> for AabbCast2d {
    fn intersects(&self, volume: &Aabb2d) -> bool {
        self.aabb_collision_at(*volume).is_some()
    }
}

/// An intersection test that casts a [`BoundingCircle`] along a ray.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct BoundingCircleCast {
    /// The ray along which to cast the bounding volume
    pub ray: RayCast2d,
    /// The circle that is being cast
    pub circle: BoundingCircle,
}

impl BoundingCircleCast {
    /// Construct a [`BoundingCircleCast`] from a [`BoundingCircle`], origin, [`Dir2`], and max distance.
    pub fn new(circle: BoundingCircle, origin: Vec2, direction: Dir2, max: f32) -> Self {
        Self::from_ray(circle, Ray2d { origin, direction }, max)
    }

    /// Construct a [`BoundingCircleCast`] from a [`BoundingCircle`], [`Ray2d`], and max distance.
    pub fn from_ray(circle: BoundingCircle, ray: Ray2d, max: f32) -> Self {
        Self {
            ray: RayCast2d::from_ray(ray, max),
            circle,
        }
    }

    /// Get the distance at which the [`BoundingCircle`]s collide, if at all.
    pub fn circle_collision_at(&self, mut circle: BoundingCircle) -> Option<f32> {
        circle.center -= self.circle.center;
        circle.circle.radius += self.circle.radius();
        self.ray.circle_intersection_at(&circle)
    }
}

impl IntersectsVolume<BoundingCircle> for BoundingCircleCast {
    fn intersects(&self, volume: &BoundingCircle) -> bool {
        self.circle_collision_at(*volume).is_some()
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
                RayCast2d::new(Vec2::Y * -5., Dir2::Y, 90.),
                BoundingCircle::new(Vec2::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of a centered bounding circle, but from the other side
                RayCast2d::new(Vec2::Y * 5., -Dir2::Y, 90.),
                BoundingCircle::new(Vec2::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of an offset circle
                RayCast2d::new(Vec2::ZERO, Dir2::Y, 90.),
                BoundingCircle::new(Vec2::Y * 3., 2.),
                1.,
            ),
            (
                // Just barely hit the circle before the max distance
                RayCast2d::new(Vec2::X, Dir2::Y, 1.),
                BoundingCircle::new(Vec2::ONE, 0.01),
                0.99,
            ),
            (
                // Hit a circle off-center
                RayCast2d::new(Vec2::X, Dir2::Y, 90.),
                BoundingCircle::new(Vec2::Y * 5., 2.),
                3.268,
            ),
            (
                // Barely hit a circle on the side
                RayCast2d::new(Vec2::X * 0.99999, Dir2::Y, 90.),
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

            let inverted_ray = RayCast2d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }

    #[test]
    fn test_ray_intersection_circle_misses() {
        for (test, volume) in &[
            (
                // The ray doesn't go in the right direction
                RayCast2d::new(Vec2::ZERO, Dir2::X, 90.),
                BoundingCircle::new(Vec2::Y * 2., 1.),
            ),
            (
                // Ray's alignment isn't enough to hit the circle
                RayCast2d::new(Vec2::ZERO, Dir2::from_xy(1., 1.).unwrap(), 90.),
                BoundingCircle::new(Vec2::Y * 2., 1.),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayCast2d::new(Vec2::ZERO, Dir2::Y, 0.5),
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
            for direction in &[Dir2::X, Dir2::Y, -Dir2::X, -Dir2::Y] {
                for max in &[0., 1., 900.] {
                    let test = RayCast2d::new(*origin, *direction, *max);

                    let case = format!(
                        "Case:\n  origin: {:?}\n  Direction: {:?}\n  Max: {}",
                        origin, direction, max,
                    );
                    assert!(test.intersects(&volume), "{}", case);

                    let actual_distance = test.circle_intersection_at(&volume);
                    assert_eq!(actual_distance, Some(0.), "{}", case);
                }
            }
        }
    }

    #[test]
    fn test_ray_intersection_aabb_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of a centered aabb
                RayCast2d::new(Vec2::Y * -5., Dir2::Y, 90.),
                Aabb2d::new(Vec2::ZERO, Vec2::ONE),
                4.,
            ),
            (
                // Hit the center of a centered aabb, but from the other side
                RayCast2d::new(Vec2::Y * 5., -Dir2::Y, 90.),
                Aabb2d::new(Vec2::ZERO, Vec2::ONE),
                4.,
            ),
            (
                // Hit the center of an offset aabb
                RayCast2d::new(Vec2::ZERO, Dir2::Y, 90.),
                Aabb2d::new(Vec2::Y * 3., Vec2::splat(2.)),
                1.,
            ),
            (
                // Just barely hit the aabb before the max distance
                RayCast2d::new(Vec2::X, Dir2::Y, 1.),
                Aabb2d::new(Vec2::ONE, Vec2::splat(0.01)),
                0.99,
            ),
            (
                // Hit an aabb off-center
                RayCast2d::new(Vec2::X, Dir2::Y, 90.),
                Aabb2d::new(Vec2::Y * 5., Vec2::splat(2.)),
                3.,
            ),
            (
                // Barely hit an aabb on corner
                RayCast2d::new(Vec2::X * -0.001, Dir2::from_xy(1., 1.).unwrap(), 90.),
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

            let inverted_ray = RayCast2d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }

    #[test]
    fn test_ray_intersection_aabb_misses() {
        for (test, volume) in &[
            (
                // The ray doesn't go in the right direction
                RayCast2d::new(Vec2::ZERO, Dir2::X, 90.),
                Aabb2d::new(Vec2::Y * 2., Vec2::ONE),
            ),
            (
                // Ray's alignment isn't enough to hit the aabb
                RayCast2d::new(Vec2::ZERO, Dir2::from_xy(1., 0.99).unwrap(), 90.),
                Aabb2d::new(Vec2::Y * 2., Vec2::ONE),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayCast2d::new(Vec2::ZERO, Dir2::Y, 0.5),
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
            for direction in &[Dir2::X, Dir2::Y, -Dir2::X, -Dir2::Y] {
                for max in &[0., 1., 900.] {
                    let test = RayCast2d::new(*origin, *direction, *max);

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

    #[test]
    fn test_aabb_cast_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of the aabb, that a ray would've also hit
                AabbCast2d::new(Aabb2d::new(Vec2::ZERO, Vec2::ONE), Vec2::ZERO, Dir2::Y, 90.),
                Aabb2d::new(Vec2::Y * 5., Vec2::ONE),
                3.,
            ),
            (
                // Hit the center of the aabb, but from the other side
                AabbCast2d::new(
                    Aabb2d::new(Vec2::ZERO, Vec2::ONE),
                    Vec2::Y * 10.,
                    -Dir2::Y,
                    90.,
                ),
                Aabb2d::new(Vec2::Y * 5., Vec2::ONE),
                3.,
            ),
            (
                // Hit the edge of the aabb, that a ray would've missed
                AabbCast2d::new(
                    Aabb2d::new(Vec2::ZERO, Vec2::ONE),
                    Vec2::X * 1.5,
                    Dir2::Y,
                    90.,
                ),
                Aabb2d::new(Vec2::Y * 5., Vec2::ONE),
                3.,
            ),
            (
                // Hit the edge of the aabb, by casting an off-center AABB
                AabbCast2d::new(
                    Aabb2d::new(Vec2::X * -2., Vec2::ONE),
                    Vec2::X * 3.,
                    Dir2::Y,
                    90.,
                ),
                Aabb2d::new(Vec2::Y * 5., Vec2::ONE),
                3.,
            ),
        ] {
            let case = format!(
                "Case:\n  Test: {:?}\n  Volume: {:?}\n  Expected distance: {:?}",
                test, volume, expected_distance
            );
            assert!(test.intersects(volume), "{}", case);
            let actual_distance = test.aabb_collision_at(*volume).unwrap();
            assert!(
                (actual_distance - expected_distance).abs() < EPSILON,
                "{}\n  Actual distance: {}",
                case,
                actual_distance
            );

            let inverted_ray =
                RayCast2d::new(test.ray.ray.origin, -test.ray.ray.direction, test.ray.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }

    #[test]
    fn test_circle_cast_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of the bounding circle, that a ray would've also hit
                BoundingCircleCast::new(
                    BoundingCircle::new(Vec2::ZERO, 1.),
                    Vec2::ZERO,
                    Dir2::Y,
                    90.,
                ),
                BoundingCircle::new(Vec2::Y * 5., 1.),
                3.,
            ),
            (
                // Hit the center of the bounding circle, but from the other side
                BoundingCircleCast::new(
                    BoundingCircle::new(Vec2::ZERO, 1.),
                    Vec2::Y * 10.,
                    -Dir2::Y,
                    90.,
                ),
                BoundingCircle::new(Vec2::Y * 5., 1.),
                3.,
            ),
            (
                // Hit the bounding circle off-center, that a ray would've missed
                BoundingCircleCast::new(
                    BoundingCircle::new(Vec2::ZERO, 1.),
                    Vec2::X * 1.5,
                    Dir2::Y,
                    90.,
                ),
                BoundingCircle::new(Vec2::Y * 5., 1.),
                3.677,
            ),
            (
                // Hit the bounding circle off-center, by casting a circle that is off-center
                BoundingCircleCast::new(
                    BoundingCircle::new(Vec2::X * -1.5, 1.),
                    Vec2::X * 3.,
                    Dir2::Y,
                    90.,
                ),
                BoundingCircle::new(Vec2::Y * 5., 1.),
                3.677,
            ),
        ] {
            let case = format!(
                "Case:\n  Test: {:?}\n  Volume: {:?}\n  Expected distance: {:?}",
                test, volume, expected_distance
            );
            assert!(test.intersects(volume), "{}", case);
            let actual_distance = test.circle_collision_at(*volume).unwrap();
            assert!(
                (actual_distance - expected_distance).abs() < EPSILON,
                "{}\n  Actual distance: {}",
                case,
                actual_distance
            );

            let inverted_ray =
                RayCast2d::new(test.ray.ray.origin, -test.ray.ray.direction, test.ray.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }
}
