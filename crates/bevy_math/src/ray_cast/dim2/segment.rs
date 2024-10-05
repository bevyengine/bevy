use ops::FloatPow;

use crate::prelude::*;

impl PrimitiveRayCast2d for Segment2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, _solid: bool) -> Option<RayHit2d> {
        // Direction perpendicular to the line segment.
        let normal = Dir2::new_unchecked(-self.direction.perp());

        let normal_dot_origin = normal.dot(-ray.origin);
        let normal_dot_dir = normal.dot(*ray.direction);

        // Check if the ray is parallel to the line, within `f32::EPSILON`.
        if normal_dot_dir.abs() < f32::EPSILON {
            // Check if the ray is collinear with the line, within `f32::EPSILON`.
            if normal_dot_origin.abs() < f32::EPSILON {
                return Some(RayHit2d::new(0.0, -ray.direction));
            }
            return None;
        }

        let distance = normal_dot_origin / normal_dot_dir;

        if distance < 0.0 || distance > max_distance {
            return None;
        }

        // Check if we are within `self.half_length`.
        let intersection = ray.origin + *ray.direction * distance;
        if intersection.length_squared() > self.half_length.squared() {
            return None;
        }

        Some(RayHit2d::new(
            distance,
            Dir2::new_unchecked(-normal_dot_dir.signum() * normal),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_segment_2d() {
        let segment = Segment2d {
            direction: Dir2::NORTH_EAST,
            half_length: 5.0,
        };

        // Hit from above at a 45 degree angle.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Vec2::NEG_Y);
        let hit = segment.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(3.0, Dir2::NORTH_WEST)));

        // Hit from below at a 45 degree angle.
        let ray = Ray2d::new(Vec2::new(2.0, -1.0), Vec2::Y);
        let hit = segment.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(3.0, Dir2::SOUTH_EAST)));

        // If the ray is parallel to the line segment (within epsilon) but not collinear, they should not intersect.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), *Dir2::NORTH_EAST);
        assert!(!segment.intersects_local_ray(ray));

        // If the ray is collinear with the line segment (within epsilon), they should intersect.
        let ray = Ray2d::new(Vec2::new(-2.0, -2.0), *Dir2::NORTH_EAST);
        assert!(segment.intersects_local_ray(ray));

        // Ray goes past the left endpoint.
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(-6.0, 2.0), Vec2::NEG_Y)));

        // Ray goes past the right endpoint.
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(6.0, -2.0), Vec2::Y)));

        // Ray points away from the line segment.
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(1.0, 2.0), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Vec2::NEG_Y);
        let hit = segment.local_ray_cast(ray, 2.5, true);
        assert!(hit.is_none());
    }
}
