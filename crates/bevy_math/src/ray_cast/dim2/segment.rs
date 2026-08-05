use crate::prelude::*;

impl PrimitiveRayCast2d for Segment2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, _solid: bool) -> Option<RayHit2d> {
        // Unit normal to the supporting line of the segment.
        let normal = -self.direction().perp();

        let denominator = normal.dot(*ray.direction);

        // Parallel?
        if ops::abs(denominator) < f32::EPSILON {
            // Collinear?
            let numerator = normal.dot(self.point1() - ray.origin);
            if ops::abs(numerator) < f32::EPSILON {
                return Some(RayHit2d::new(0.0, -ray.direction));
            }

            return None;
        }

        // Distance along the ray to the supporting line.
        let numerator = normal.dot(self.point1() - ray.origin);
        let distance = numerator / denominator;

        if distance < 0.0 || distance > max_distance {
            return None;
        }

        // Compute the intersection point.
        let intersection = ray.origin + *ray.direction * distance;

        // Check whether the intersection lies on the finite segment.
        let segment = self.point2() - self.point1();
        let t = (intersection - self.point1()).dot(segment) / segment.length_squared();

        if !(0.0..=1.0).contains(&t) {
            return None;
        }

        Some(RayHit2d::new(
            distance,
            Dir2::new(-denominator.signum() * normal).ok()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_segment_2d() {
        let segment = Segment2d::new(Vec2::NEG_ONE * 5.0, Vec2::ONE * 5.0);

        // Hit from above at a 45 degree angle.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Dir2::NEG_Y);
        let hit = segment
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(3.0, Dir2::NORTH_WEST);
        assert_eq!(hit.distance, expected_hit.distance);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        // Hit from below at a 45 degree angle.
        let ray = Ray2d::new(Vec2::new(2.0, -1.0), Dir2::Y);
        let hit = segment
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(3.0, Dir2::SOUTH_EAST);
        assert!(ops::abs(hit.distance - expected_hit.distance) < 0.000_001);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        // If the ray is parallel to the line segment (within epsilon) but not collinear, they should not intersect.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Dir2::NORTH_EAST);
        assert!(!segment.intersects_local_ray(ray));

        // If the ray is collinear with the line segment (within epsilon), they should intersect.
        let ray = Ray2d::new(Vec2::new(-2.0, -2.0), Dir2::NORTH_EAST);
        assert!(segment.intersects_local_ray(ray));

        // Ray goes past the left endpoint.
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(-6.0, 2.0), Dir2::NEG_Y)));

        // Ray goes past the right endpoint.
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(6.0, -2.0), Dir2::Y)));

        // Ray points away from the line segment.
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(1.0, 2.0), Dir2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Dir2::NEG_Y);
        let hit = segment.local_ray_cast(ray, 2.5, true);
        assert!(hit.is_none());
    }
}
