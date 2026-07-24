use crate::bounding::{
    bounded3d::{Aabb3d, BoundingSphere},
    IntersectsVolume,
};
use bevy_math::{Dir3A, Ray3d, RayCast3d, Vec3A};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// Computes intersections with an [`Aabb3d`].
///
/// Implementors return the distance from the query's origin to the first
/// intersection with the given axis-aligned bounding box. If no intersection
/// occurs, `None` is returned.
pub trait Aabb3dIntersection {
    /// Get the distance of an intersection with an [`Aabb3d`], if any.
    fn aabb_intersection_at(&self, aabb: &Aabb3d) -> Option<f32>;
}

/// Computes intersections with a [`BoundingSphere`].
///
/// Implementors return the distance from the query's origin to the first
/// intersection with the given bounding sphere. If no intersection occurs,
/// `None` is returned.
pub trait BoundingSphereIntersection {
    /// Get the distance of an intersection with a [`BoundingSphere`], if any.
    fn sphere_intersection_at(&self, sphere: &BoundingSphere) -> Option<f32>;
}

impl Aabb3dIntersection for RayCast3d {
    /// Get the distance of an intersection with an [`Aabb3d`], if any.
    fn aabb_intersection_at(&self, aabb: &Aabb3d) -> Option<f32> {
        self.aabb_intersection_at_min_max(aabb.min, aabb.max)
    }
}

impl BoundingSphereIntersection for RayCast3d {
    /// Get the distance of an intersection with a [`BoundingSphere`], if any.
    fn sphere_intersection_at(&self, sphere: &BoundingSphere) -> Option<f32> {
        self.sphere_intersection_at_center_radius(sphere.center, sphere.radius())
    }
}

impl IntersectsVolume<Aabb3d> for RayCast3d {
    fn intersects(&self, volume: &Aabb3d) -> bool {
        self.aabb_intersection_at(volume).is_some()
    }
}

impl IntersectsVolume<BoundingSphere> for RayCast3d {
    fn intersects(&self, volume: &BoundingSphere) -> bool {
        self.sphere_intersection_at(volume).is_some()
    }
}

/// An intersection test that casts an [`Aabb3d`] along a ray.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
pub struct AabbCast3d {
    /// The ray along which to cast the bounding volume
    pub ray: RayCast3d,
    /// The aabb that is being cast
    pub aabb: Aabb3d,
}

impl AabbCast3d {
    /// Construct an [`AabbCast3d`] from an [`Aabb3d`], origin, [direction], and max distance.
    ///
    /// [direction]: bevy_math::Dir3
    pub fn new(
        aabb: Aabb3d,
        origin: impl Into<Vec3A>,
        direction: impl Into<Dir3A>,
        max: f32,
    ) -> Self {
        Self {
            ray: RayCast3d::new(origin, direction, max),
            aabb,
        }
    }

    /// Construct an [`AabbCast3d`] from an [`Aabb3d`], [`Ray3d`], and max distance.
    pub fn from_ray(aabb: Aabb3d, ray: Ray3d, max: f32) -> Self {
        Self::new(aabb, ray.origin, ray.direction, max)
    }

    /// Get the distance at which the [`Aabb3d`]s collide, if at all.
    pub fn aabb_collision_at(&self, mut aabb: Aabb3d) -> Option<f32> {
        aabb.min -= self.aabb.max;
        aabb.max -= self.aabb.min;

        self.ray.aabb_intersection_at(&aabb)
    }
}

impl IntersectsVolume<Aabb3d> for AabbCast3d {
    fn intersects(&self, volume: &Aabb3d) -> bool {
        self.aabb_collision_at(*volume).is_some()
    }
}

/// An intersection test that casts a [`BoundingSphere`] along a ray.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
pub struct BoundingSphereCast {
    /// The ray along which to cast the bounding volume
    pub ray: RayCast3d,
    /// The sphere that is being cast
    pub sphere: BoundingSphere,
}

impl BoundingSphereCast {
    /// Construct a [`BoundingSphereCast`] from a [`BoundingSphere`], origin, [direction], and max distance.
    ///
    /// [direction]: bevy_math::Dir3
    pub fn new(
        sphere: BoundingSphere,
        origin: impl Into<Vec3A>,
        direction: impl Into<Dir3A>,
        max: f32,
    ) -> Self {
        Self {
            ray: RayCast3d::new(origin, direction, max),
            sphere,
        }
    }

    /// Construct a [`BoundingSphereCast`] from a [`BoundingSphere`], [`Ray3d`], and max distance.
    pub fn from_ray(sphere: BoundingSphere, ray: Ray3d, max: f32) -> Self {
        Self::new(sphere, ray.origin, ray.direction, max)
    }

    /// Get the distance at which the [`BoundingSphere`]s collide, if at all.
    pub fn sphere_collision_at(&self, mut sphere: BoundingSphere) -> Option<f32> {
        sphere.center -= self.sphere.center;
        sphere.sphere.radius += self.sphere.radius();
        self.ray.sphere_intersection_at(&sphere)
    }
}

impl IntersectsVolume<BoundingSphere> for BoundingSphereCast {
    fn intersects(&self, volume: &BoundingSphere) -> bool {
        self.sphere_collision_at(*volume).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::{ops, Dir3, Vec3};

    const EPSILON: f32 = 0.001;

    #[test]
    fn test_ray_intersection_sphere_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of a centered bounding sphere
                RayCast3d::new(Vec3::Y * -5., Dir3::Y, 90.),
                BoundingSphere::new(Vec3::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of a centered bounding sphere, but from the other side
                RayCast3d::new(Vec3::Y * 5., -Dir3::Y, 90.),
                BoundingSphere::new(Vec3::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of an offset sphere
                RayCast3d::new(Vec3::ZERO, Dir3::Y, 90.),
                BoundingSphere::new(Vec3::Y * 3., 2.),
                1.,
            ),
            (
                // Just barely hit the sphere before the max distance
                RayCast3d::new(Vec3::X, Dir3::Y, 1.),
                BoundingSphere::new(Vec3::new(1., 1., 0.), 0.01),
                0.99,
            ),
            (
                // Hit a sphere off-center
                RayCast3d::new(Vec3::X, Dir3::Y, 90.),
                BoundingSphere::new(Vec3::Y * 5., 2.),
                3.268,
            ),
            (
                // Barely hit a sphere on the side
                RayCast3d::new(Vec3::X * 0.99999, Dir3::Y, 90.),
                BoundingSphere::new(Vec3::Y * 5., 1.),
                4.996,
            ),
        ] {
            assert!(
                test.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
            let actual_distance = test.sphere_intersection_at(volume).unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );

            let inverted_ray = RayCast3d::new(test.origin, -test.direction, test.max);
            assert!(
                !inverted_ray.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
        }
    }

    #[test]
    fn test_ray_intersection_sphere_misses() {
        for (test, volume) in &[
            (
                // The ray doesn't go in the right direction
                RayCast3d::new(Vec3::ZERO, Dir3::X, 90.),
                BoundingSphere::new(Vec3::Y * 2., 1.),
            ),
            (
                // Ray's alignment isn't enough to hit the sphere
                RayCast3d::new(Vec3::ZERO, Dir3::from_xyz(1., 1., 1.).unwrap(), 90.),
                BoundingSphere::new(Vec3::Y * 2., 1.),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayCast3d::new(Vec3::ZERO, Dir3::Y, 0.5),
                BoundingSphere::new(Vec3::Y * 2., 1.),
            ),
        ] {
            assert!(
                !test.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}",
            );
        }
    }

    #[test]
    fn test_ray_intersection_sphere_inside() {
        let volume = BoundingSphere::new(Vec3::splat(0.5), 1.);
        for origin in &[Vec3::X, Vec3::Y, Vec3::ONE, Vec3::ZERO] {
            for direction in &[Dir3::X, Dir3::Y, Dir3::Z, -Dir3::X, -Dir3::Y, -Dir3::Z] {
                for max in &[0., 1., 900.] {
                    let test = RayCast3d::new(*origin, *direction, *max);

                    assert!(
                        test.intersects(&volume),
                        "Case:\n  origin: {origin:?}\n  Direction: {direction:?}\n  Max: {max}",
                    );

                    let actual_distance = test.sphere_intersection_at(&volume);
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
                RayCast3d::new(Vec3::Y * -5., Dir3::Y, 90.),
                Aabb3d::new(Vec3::ZERO, Vec3::ONE),
                4.,
            ),
            (
                // Hit the center of a centered aabb, but from the other side
                RayCast3d::new(Vec3::Y * 5., -Dir3::Y, 90.),
                Aabb3d::new(Vec3::ZERO, Vec3::ONE),
                4.,
            ),
            (
                // Hit the center of an offset aabb
                RayCast3d::new(Vec3::ZERO, Dir3::Y, 90.),
                Aabb3d::new(Vec3::Y * 3., Vec3::splat(2.)),
                1.,
            ),
            (
                // Just barely hit the aabb before the max distance
                RayCast3d::new(Vec3::X, Dir3::Y, 1.),
                Aabb3d::new(Vec3::new(1., 1., 0.), Vec3::splat(0.01)),
                0.99,
            ),
            (
                // Hit an aabb off-center
                RayCast3d::new(Vec3::X, Dir3::Y, 90.),
                Aabb3d::new(Vec3::Y * 5., Vec3::splat(2.)),
                3.,
            ),
            (
                // Barely hit an aabb on corner
                RayCast3d::new(Vec3::X * -0.001, Dir3::from_xyz(1., 1., 1.).unwrap(), 90.),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
                1.732,
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

            let inverted_ray = RayCast3d::new(test.origin, -test.direction, test.max);
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
                RayCast3d::new(Vec3::ZERO, Dir3::X, 90.),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
            ),
            (
                // Ray's alignment isn't enough to hit the aabb
                RayCast3d::new(Vec3::ZERO, Dir3::from_xyz(1., 0.99, 1.).unwrap(), 90.),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayCast3d::new(Vec3::ZERO, Dir3::Y, 0.5),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
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
        let volume = Aabb3d::new(Vec3::splat(0.5), Vec3::ONE);
        for origin in &[Vec3::X, Vec3::Y, Vec3::ONE, Vec3::ZERO] {
            for direction in &[Dir3::X, Dir3::Y, Dir3::Z, -Dir3::X, -Dir3::Y, -Dir3::Z] {
                for max in &[0., 1., 900.] {
                    let test = RayCast3d::new(*origin, *direction, *max);

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
                AabbCast3d::new(Aabb3d::new(Vec3::ZERO, Vec3::ONE), Vec3::ZERO, Dir3::Y, 90.),
                Aabb3d::new(Vec3::Y * 5., Vec3::ONE),
                3.,
            ),
            (
                // Hit the center of the aabb, but from the other side
                AabbCast3d::new(
                    Aabb3d::new(Vec3::ZERO, Vec3::ONE),
                    Vec3::Y * 10.,
                    -Dir3::Y,
                    90.,
                ),
                Aabb3d::new(Vec3::Y * 5., Vec3::ONE),
                3.,
            ),
            (
                // Hit the edge of the aabb, that a ray would've missed
                AabbCast3d::new(
                    Aabb3d::new(Vec3::ZERO, Vec3::ONE),
                    Vec3::X * 1.5,
                    Dir3::Y,
                    90.,
                ),
                Aabb3d::new(Vec3::Y * 5., Vec3::ONE),
                3.,
            ),
            (
                // Hit the edge of the aabb, by casting an off-center AABB
                AabbCast3d::new(
                    Aabb3d::new(Vec3::X * -2., Vec3::ONE),
                    Vec3::X * 3.,
                    Dir3::Y,
                    90.,
                ),
                Aabb3d::new(Vec3::Y * 5., Vec3::ONE),
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

            let inverted_ray = RayCast3d::new(test.ray.origin, -test.ray.direction, test.ray.max);
            assert!(
                !inverted_ray.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
        }
    }

    #[test]
    fn test_sphere_cast_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of the bounding sphere, that a ray would've also hit
                BoundingSphereCast::new(
                    BoundingSphere::new(Vec3::ZERO, 1.),
                    Vec3::ZERO,
                    Dir3::Y,
                    90.,
                ),
                BoundingSphere::new(Vec3::Y * 5., 1.),
                3.,
            ),
            (
                // Hit the center of the bounding sphere, but from the other side
                BoundingSphereCast::new(
                    BoundingSphere::new(Vec3::ZERO, 1.),
                    Vec3::Y * 10.,
                    -Dir3::Y,
                    90.,
                ),
                BoundingSphere::new(Vec3::Y * 5., 1.),
                3.,
            ),
            (
                // Hit the bounding sphere off-center, that a ray would've missed
                BoundingSphereCast::new(
                    BoundingSphere::new(Vec3::ZERO, 1.),
                    Vec3::X * 1.5,
                    Dir3::Y,
                    90.,
                ),
                BoundingSphere::new(Vec3::Y * 5., 1.),
                3.677,
            ),
            (
                // Hit the bounding sphere off-center, by casting a sphere that is off-center
                BoundingSphereCast::new(
                    BoundingSphere::new(Vec3::X * -1.5, 1.),
                    Vec3::X * 3.,
                    Dir3::Y,
                    90.,
                ),
                BoundingSphere::new(Vec3::Y * 5., 1.),
                3.677,
            ),
        ] {
            assert!(
                test.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
            let actual_distance = test.sphere_collision_at(*volume).unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );

            let inverted_ray = RayCast3d::new(test.ray.origin, -test.ray.direction, test.ray.max);
            assert!(
                !inverted_ray.intersects(volume),
                "Case:\n  Test: {test:?}\n  Volume: {volume:?}\n  Expected distance: {expected_distance:?}",
            );
        }
    }
}
