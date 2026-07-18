use crate::bounding::{
    bounded2d::{Aabb2d, BoundingCircle},
    IntersectsVolume,
};
use bevy_math::{Dir2, Ray2d, RayCast2d, Vec2};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

pub trait Aabb2dIntersection {
    /// Get the distance of an intersection with an [`Aabb2d`], if any.
    fn aabb_intersection_at(&self, aabb: &Aabb2d) -> Option<f32>;
}

pub trait BoundingCircleIntersection {
    /// Get the distance of an intersection with a [`BoundingCircle`], if any.
    fn circle_intersection_at(&self, circle: &BoundingCircle) -> Option<f32>;
}

impl Aabb2dIntersection for RayCast2d {
    /// Get the distance of an intersection with an [`Aabb2d`], if any.
    fn aabb_intersection_at(&self, aabb: &Aabb2d) -> Option<f32> {
        self.aabb_intersection_at_min_max(aabb.min, aabb.max)
    }
}

impl BoundingCircleIntersection for RayCast2d {
    /// Get the distance of an intersection with a [`BoundingCircle`], if any.
    fn circle_intersection_at(&self, circle: &BoundingCircle) -> Option<f32> {
        self.circle_intersection_at_center_radius(circle.center, circle.radius())
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
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
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
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
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
    use bevy_math::ops;

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
            assert!(
                test.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
            let actual_distance = test.circle_intersection_at(volume).unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );

            let inverted_ray = RayCast2d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(
                !inverted_ray.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
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
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}",
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

                    assert!(
                        test.intersects(&volume),
                        "Case:\n  origin: {origin:?}\n  Direction: {direction:?}\n  Max: {max}",
                    );
                    let actual_distance = test.circle_intersection_at(&volume);
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
            assert!(
                test.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
            let actual_distance = test.aabb_intersection_at(volume).unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );

            let inverted_ray = RayCast2d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(
                !inverted_ray.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
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
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}",
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

                    assert!(
                        test.intersects(&volume),
                        "Case:\n  origin: {origin:?}\n  Direction: {direction:?}\n  Max: {max}",
                    );
                    let actual_distance = test.aabb_intersection_at(&volume);
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
            assert!(
                test.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
            let actual_distance = test.aabb_collision_at(*volume).unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );

            let inverted_ray =
                RayCast2d::new(test.ray.ray.origin, -test.ray.ray.direction, test.ray.max);
            assert!(
                !inverted_ray.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
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
            assert!(
                test.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
            let actual_distance = test.circle_collision_at(*volume).unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );

            let inverted_ray =
                RayCast2d::new(test.ray.ray.origin, -test.ray.ray.direction, test.ray.max);
            assert!(
                !inverted_ray.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
        }
    }
}
