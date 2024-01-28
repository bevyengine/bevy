use super::{Aabb3d, BoundingSphere, IntersectsVolume};
use crate::{primitives::Direction3d, Ray3d, Vec3};

/// A raycast intersection test for 3D bounding volumes
#[derive(Debug)]
pub struct RayTest3d {
    /// The ray for the test
    pub ray: Ray3d,
    /// The maximum distance for the ray
    pub max: f32,
    /// The multiplicative inverse direction of the ray
    direction_recip: Vec3,
}

impl RayTest3d {
    /// Construct a [`RayTest3d`] from an origin, [`Direction3d`] and max distance.
    pub fn new(origin: Vec3, direction: Direction3d, max: f32) -> Self {
        Self::from_ray(Ray3d { origin, direction }, max)
    }

    /// Construct a [`RayTest3d`] from a [`Ray3d`] and max distance.
    pub fn from_ray(ray: Ray3d, max: f32) -> Self {
        Self {
            ray,
            direction_recip: ray.direction.recip(),
            max,
        }
    }

    /// Get the cached multiplicative inverse of the direction of the ray.
    pub fn direction_recip(&self) -> Vec3 {
        self.direction_recip
    }

    /// Get the distance of an intersection with an [`Aabb3d`], if any.
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

        // Calculate the minimum/maximum time for each axis based on how much the direction goes that
        // way. These values can get arbitrarily large, or even become NaN, which is handled by the
        // min/max operations below
        let tmin_x = (min_x - self.ray.origin.x) * self.direction_recip.x;
        let tmin_y = (min_y - self.ray.origin.y) * self.direction_recip.y;
        let tmin_z = (min_z - self.ray.origin.z) * self.direction_recip.z;
        let tmax_x = (max_x - self.ray.origin.x) * self.direction_recip.x;
        let tmax_y = (max_y - self.ray.origin.y) * self.direction_recip.y;
        let tmax_z = (max_z - self.ray.origin.z) * self.direction_recip.z;

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

    /// Get the distance of an intersection with a [`BoundingSphere`], if any.
    pub fn sphere_intersection_at(&self, sphere: &BoundingSphere) -> Option<f32> {
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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.001;

    #[test]
    fn test_ray_intersection_sphere_hits() {
        for (test, volume, expected_distance) in &[
            (
                // Hit the center of a centered bounding sphere
                RayTest3d::new(Vec3::Y * -5., Direction3d::Y, 90.),
                BoundingSphere::new(Vec3::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of a centered bounding sphere, but from the other side
                RayTest3d::new(Vec3::Y * 5., -Direction3d::Y, 90.),
                BoundingSphere::new(Vec3::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of an offset sphere
                RayTest3d::new(Vec3::ZERO, Direction3d::Y, 90.),
                BoundingSphere::new(Vec3::Y * 3., 2.),
                1.,
            ),
            (
                // Just barely hit the sphere before the max distance
                RayTest3d::new(Vec3::X, Direction3d::Y, 1.),
                BoundingSphere::new(Vec3::new(1., 1., 0.), 0.01),
                0.99,
            ),
            (
                // Hit a sphere off-center
                RayTest3d::new(Vec3::X, Direction3d::Y, 90.),
                BoundingSphere::new(Vec3::Y * 5., 2.),
                3.268,
            ),
            (
                // Barely hit a sphere on the side
                RayTest3d::new(Vec3::X * 0.99999, Direction3d::Y, 90.),
                BoundingSphere::new(Vec3::Y * 5., 1.),
                4.996,
            ),
        ] {
            let case = format!(
                "Case:\n  Test: {:?}\n  Volume: {:?}\n  Expected distance: {:?}",
                test, volume, expected_distance
            );
            assert!(test.intersects(volume), "{}", case);
            let actual_distance = test.sphere_intersection_at(volume).unwrap();
            assert!(
                (actual_distance - expected_distance).abs() < EPSILON,
                "{}\n  Actual distance: {}",
                case,
                actual_distance
            );

            let inverted_ray = RayTest3d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }

    #[test]
    fn test_ray_intersection_sphere_misses() {
        for (test, volume) in &[
            (
                // The ray doesn't go in the right direction
                RayTest3d::new(Vec3::ZERO, Direction3d::X, 90.),
                BoundingSphere::new(Vec3::Y * 2., 1.),
            ),
            (
                // Ray's alignment isn't enough to hit the sphere
                RayTest3d::new(Vec3::ZERO, Direction3d::from_xyz(1., 1., 1.).unwrap(), 90.),
                BoundingSphere::new(Vec3::Y * 2., 1.),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayTest3d::new(Vec3::ZERO, Direction3d::Y, 0.5),
                BoundingSphere::new(Vec3::Y * 2., 1.),
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
    fn test_ray_intersection_sphere_inside() {
        let volume = BoundingSphere::new(Vec3::splat(0.5), 1.);
        for origin in &[Vec3::X, Vec3::Y, Vec3::ONE, Vec3::ZERO] {
            for direction in &[
                Direction3d::X,
                Direction3d::Y,
                -Direction3d::X,
                -Direction3d::Y,
            ] {
                for max in &[0., 1., 900.] {
                    let test = RayTest3d::new(*origin, *direction, *max);

                    let case = format!(
                        "Case:\n  origin: {:?}\n  Direction: {:?}\n  Max: {}",
                        origin, direction, max,
                    );
                    assert!(test.intersects(&volume), "{}", case);

                    let actual_distance = test.sphere_intersection_at(&volume);
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
                RayTest3d::new(Vec3::Y * -5., Direction3d::Y, 90.),
                Aabb3d::new(Vec3::ZERO, Vec3::ONE),
                4.,
            ),
            (
                // Hit the center of a centered aabb, but from the other side
                RayTest3d::new(Vec3::Y * 5., -Direction3d::Y, 90.),
                Aabb3d::new(Vec3::ZERO, Vec3::ONE),
                4.,
            ),
            (
                // Hit the center of an offset aabb
                RayTest3d::new(Vec3::ZERO, Direction3d::Y, 90.),
                Aabb3d::new(Vec3::Y * 3., Vec3::splat(2.)),
                1.,
            ),
            (
                // Just barely hit the aabb before the max distance
                RayTest3d::new(Vec3::X, Direction3d::Y, 1.),
                Aabb3d::new(Vec3::new(1., 1., 0.), Vec3::splat(0.01)),
                0.99,
            ),
            (
                // Hit an aabb off-center
                RayTest3d::new(Vec3::X, Direction3d::Y, 90.),
                Aabb3d::new(Vec3::Y * 5., Vec3::splat(2.)),
                3.,
            ),
            (
                // Barely hit an aabb on corner
                RayTest3d::new(
                    Vec3::X * -0.001,
                    Direction3d::from_xyz(1., 1., 1.).unwrap(),
                    90.,
                ),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
                1.732,
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

            let inverted_ray = RayTest3d::new(test.ray.origin, -test.ray.direction, test.max);
            assert!(!inverted_ray.intersects(volume), "{}", case);
        }
    }

    #[test]
    fn test_ray_intersection_aabb_misses() {
        for (test, volume) in &[
            (
                // The ray doesn't go in the right direction
                RayTest3d::new(Vec3::ZERO, Direction3d::X, 90.),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
            ),
            (
                // Ray's alignment isn't enough to hit the aabb
                RayTest3d::new(
                    Vec3::ZERO,
                    Direction3d::from_xyz(1., 0.99, 1.).unwrap(),
                    90.,
                ),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
            ),
            (
                // The ray's maximum distance isn't high enough
                RayTest3d::new(Vec3::ZERO, Direction3d::Y, 0.5),
                Aabb3d::new(Vec3::Y * 2., Vec3::ONE),
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
        let volume = Aabb3d::new(Vec3::splat(0.5), Vec3::ONE);
        for origin in &[Vec3::X, Vec3::Y, Vec3::ONE, Vec3::ZERO] {
            for direction in &[
                Direction3d::X,
                Direction3d::Y,
                -Direction3d::X,
                -Direction3d::Y,
            ] {
                for max in &[0., 1., 900.] {
                    let test = RayTest3d::new(*origin, *direction, *max);

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
