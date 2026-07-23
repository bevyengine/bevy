use crate::prelude::*;

impl PrimitiveRayCast2d for Line2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, _solid: bool) -> Option<RayHit2d> {
        // Direction perpendicular to the line.
        let normal = Dir2::new(-self.direction.perp()).ok()?;

        let normal_dot_origin = normal.dot(-ray.origin);
        let normal_dot_dir = normal.dot(*ray.direction);

        // Check if the ray is parallel to the line, within `f32::EPSILON`.
        if ops::abs(normal_dot_dir) < f32::EPSILON {
            // Check if the ray is collinear with the line, within `f32::EPSILON`.
            if ops::abs(normal_dot_origin) < f32::EPSILON {
                return Some(RayHit2d::new(0.0, -ray.direction));
            }
            return None;
        }

        let distance = normal_dot_origin / normal_dot_dir;

        if distance < 0.0 || distance > max_distance {
            return None;
        }

        Some(RayHit2d::new(
            distance,
            Dir2::new(-normal_dot_dir.signum() * normal).ok()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_line_2d() {
        let line = Line2d {
            direction: Dir2::NORTH_EAST,
        };

        // Hit from above at a 45 degree angle.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Dir2::NEG_Y);
        let hit = line
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(3.0, Dir2::NORTH_WEST);
        assert_eq!(hit.distance, expected_hit.distance);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        // Hit from below at a 45 degree angle.
        let ray = Ray2d::new(Vec2::new(2.0, -1.0), Dir2::Y);
        let hit = line
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(3.0, Dir2::SOUTH_EAST);
        assert_eq!(hit.distance, expected_hit.distance);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        // If the ray is parallel to the line (within epsilon) but not collinear, they should not intersect.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Dir2::NORTH_EAST);
        assert!(!line.intersects_local_ray(ray));

        // If the ray is collinear with the line (within epsilon), they should intersect.
        let ray = Ray2d::new(Vec2::new(-2.0, -2.0), Dir2::NORTH_EAST);
        assert!(line.intersects_local_ray(ray));

        // Ray points away from the line.
        assert!(!line.intersects_local_ray(Ray2d::new(Vec2::new(1.0, 2.0), Dir2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Dir2::NEG_Y);
        let hit = line.local_ray_cast(ray, 2.5, true);
        assert!(hit.is_none());
    }
}
