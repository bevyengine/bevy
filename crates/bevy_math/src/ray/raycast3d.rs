use crate::{
    ops::{self, FloatPow},
    Dir3A, Ray3d, Vec3A,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// A raycast intersection test for 3D bounding volumes
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
pub struct RayCast3d {
    /// The origin of the ray.
    pub origin: Vec3A,
    /// The direction of the ray.
    pub direction: Dir3A,
    /// The maximum distance for the ray
    pub max: f32,
    /// The multiplicative inverse direction of the ray
    direction_recip: Vec3A,
}

impl RayCast3d {
    /// Construct a [`RayCast3d`] from an origin, [direction], and max distance.
    ///
    /// [direction]: crate::direction::Dir3
    pub fn new(origin: impl Into<Vec3A>, direction: impl Into<Dir3A>, max: f32) -> Self {
        let direction = direction.into();
        Self {
            origin: origin.into(),
            direction,
            direction_recip: direction.recip(),
            max,
        }
    }

    /// Construct a [`RayCast3d`] from a [`Ray3d`] and max distance.
    pub fn from_ray(ray: Ray3d, max: f32) -> Self {
        Self::new(ray.origin, ray.direction, max)
    }

    /// Get the cached multiplicative inverse of the direction of the ray.
    pub const fn direction_recip(&self) -> Vec3A {
        self.direction_recip
    }

    /// Get the distance of an intersection with an box defined by min/max points, if any.
    #[inline]
    pub fn aabb_intersection_at_min_max(&self, min: Vec3A, max: Vec3A) -> Option<f32> {
        let positive = self.direction.signum().cmpgt(Vec3A::ZERO);
        let min_selected = Vec3A::select(positive, min, max);
        let max_selected = Vec3A::select(positive, max, min);

        // Calculate the minimum/maximum time for each axis based on how much the direction goes that
        // way. These values can get arbitrarily large, or even become NaN, which is handled by the
        // min/max operations below
        let tmin = (min_selected - self.origin) * self.direction_recip;
        let tmax = (max_selected - self.origin) * self.direction_recip;

        // An axis that is not relevant to the ray direction will be NaN. When one of the arguments
        // to min/max is NaN, the other argument is used.
        // An axis for which the direction is the wrong way will return an arbitrarily large
        // negative value.
        let tmin = tmin.max_element().max(0.);
        let tmax = tmax.min_element().min(self.max);

        if tmin <= tmax {
            Some(tmin)
        } else {
            None
        }
    }

    /// Get the distance of an intersection with a sphere defined by center/radius, if any.
    #[inline]
    pub fn sphere_intersection_at_center_radius(&self, center: Vec3A, radius: f32) -> Option<f32> {
        let offset = self.origin - center;
        let projected = offset.dot(*self.direction);
        let closest_point = offset - projected * *self.direction;
        let distance_squared = radius.squared() - closest_point.length_squared();
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
    use crate::{Dir3, Vec3};

    const EPSILON: f32 = 0.001;

    #[test]
    fn test_ray_intersection_sphere_hits() {
        for (test, (center, radius), expected_distance) in &[
            (
                // Hit the center of a centered bounding sphere
                RayCast3d::new(Vec3::Y * -5., Dir3::Y, 90.),
                (Vec3::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of a centered bounding sphere, but from the other side
                RayCast3d::new(Vec3::Y * 5., -Dir3::Y, 90.),
                (Vec3::ZERO, 1.),
                4.,
            ),
            (
                // Hit the center of an offset sphere
                RayCast3d::new(Vec3::ZERO, Dir3::Y, 90.),
                (Vec3::Y * 3., 2.),
                1.,
            ),
            (
                // Just barely hit the sphere before the max distance
                RayCast3d::new(Vec3::X, Dir3::Y, 1.),
                (Vec3::new(1., 1., 0.), 0.01),
                0.99,
            ),
            (
                // Hit a sphere off-center
                RayCast3d::new(Vec3::X, Dir3::Y, 90.),
                (Vec3::Y * 5., 2.),
                3.268,
            ),
            (
                // Barely hit a sphere on the side
                RayCast3d::new(Vec3::X * 0.99999, Dir3::Y, 90.),
                (Vec3::Y * 5., 1.),
                4.996,
            ),
        ] {
            let actual_distance = test
                .sphere_intersection_at_center_radius(center.to_vec3a(), *radius)
                .unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: center: {center:?}, radius: {radius}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );
        }
    }

    #[test]
    fn test_ray_intersection_sphere_inside() {
        let (center, radius) = (Vec3::splat(0.5).to_vec3a(), 1.);
        for origin in &[Vec3::X, Vec3::Y, Vec3::ONE, Vec3::ZERO] {
            for direction in &[Dir3::X, Dir3::Y, Dir3::Z, -Dir3::X, -Dir3::Y, -Dir3::Z] {
                for max in &[0., 1., 900.] {
                    let test = RayCast3d::new(*origin, *direction, *max);

                    let actual_distance = test.sphere_intersection_at_center_radius(center, radius);
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
                RayCast3d::new(Vec3::Y * -5., Dir3::Y, 90.),
                (Vec3::ZERO, Vec3::ONE),
                4.,
            ),
            (
                // Hit the center of a centered aabb, but from the other side
                RayCast3d::new(Vec3::Y * 5., -Dir3::Y, 90.),
                (Vec3::ZERO, Vec3::ONE),
                4.,
            ),
            (
                // Hit the center of an offset aabb
                RayCast3d::new(Vec3::ZERO, Dir3::Y, 90.),
                (Vec3::Y * 3., Vec3::splat(2.)),
                1.,
            ),
            (
                // Just barely hit the aabb before the max distance
                RayCast3d::new(Vec3::X, Dir3::Y, 1.),
                (Vec3::new(1., 1., 0.), Vec3::splat(0.01)),
                0.99,
            ),
            (
                // Hit an aabb off-center
                RayCast3d::new(Vec3::X, Dir3::Y, 90.),
                (Vec3::Y * 5., Vec3::splat(2.)),
                3.,
            ),
            (
                // Barely hit an aabb on corner
                RayCast3d::new(Vec3::X * -0.001, Dir3::from_xyz(1., 1., 1.).unwrap(), 90.),
                (Vec3::Y * 2., Vec3::ONE),
                1.732,
            ),
        ]
        .map(|(a, (center, half_size), b)| (a, (center - half_size, center + half_size), b))
        {
            let actual_distance = test
                .aabb_intersection_at_min_max(min.to_vec3a(), max.to_vec3a())
                .unwrap();
            assert!(
                ops::abs(actual_distance - expected_distance) < EPSILON,
                "Case:\n  Test: {test:?}\n  Volume: min: {min:?}, max: {max:?}\n  Expected distance: {expected_distance:?}\n  Actual distance: {actual_distance}",
            );
        }
    }

    #[test]
    fn test_ray_intersection_aabb_inside() {
        let (min, max) = (Vec3::splat(0.5) - Vec3::ONE, Vec3::splat(0.5) + Vec3::ONE);
        for origin in &[Vec3::X, Vec3::Y, Vec3::ONE, Vec3::ZERO] {
            for direction in &[Dir3::X, Dir3::Y, Dir3::Z, -Dir3::X, -Dir3::Y, -Dir3::Z] {
                for max_dist in &[0., 1., 900.] {
                    let test = RayCast3d::new(*origin, *direction, *max_dist);

                    let actual_distance =
                        test.aabb_intersection_at_min_max(min.to_vec3a(), max.to_vec3a());
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
