use crate::prelude::*;

impl PrimitiveRayCast2d for Rectangle {
    #[inline]
    fn local_ray_distance(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<f32> {
        // Adapted from Inigo Quilez's algorithm: https://iquilezles.org/articles/intersectors/

        let direction_recip_abs = ray.direction.recip().abs();

        // Note: The operations here were modified and rearranged to avoid the Inf - Inf = NaN result
        //       for the edge case where a component of the ray direction is zero and the reciprocal
        //       is infinity. The NaN would break the early return below.
        let n = -ray.direction.signum() * ray.origin;
        let t1 = direction_recip_abs * (n - self.half_size);
        let t2 = direction_recip_abs * (n + self.half_size);

        let distance_near = t1.max_element();
        let distance_far = t2.min_element();

        if distance_near > distance_far || distance_far < 0.0 {
            return None;
        }

        if distance_near > 0.0 {
            // The ray hit the outside of the rectangle.
            Some(distance_near)
        } else if solid {
            // The ray origin is inside of the solid rectangle.
            Some(0.0)
        } else if distance_far <= max_distance {
            // The ray hit the inside of the hollow rectangle.
            Some(distance_far)
        } else {
            None
        }
    }

    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        // Adapted from Inigo Quilez's algorithm: https://iquilezles.org/articles/intersectors/

        let direction_recip_abs = ray.direction.recip().abs();

        // Note: The operations here were modified and rearranged to avoid the Inf - Inf = NaN result
        //       for the edge case where a component of the ray direction is zero and the reciprocal
        //       is infinity. The NaN would break the early return below.
        let n = -ray.direction.signum() * ray.origin;
        let t1 = direction_recip_abs * (n - self.half_size);
        let t2 = direction_recip_abs * (n + self.half_size);

        let distance_near = t1.max_element();
        let distance_far = t2.min_element();

        if distance_near > distance_far || distance_far < 0.0 || distance_near > max_distance {
            return None;
        }

        if distance_near > 0.0 {
            // The ray hit the outside of the rectangle.
            // Note: We could also just have an if-else here, but it was measured to be ~15% slower.
            let normal_abs = Vec2::from(Vec2::splat(distance_near).cmple(t1));
            let normal = Dir2::new(-ray.direction.signum() * normal_abs).ok()?;
            Some(RayHit2d::new(distance_near, normal))
        } else if solid {
            // The ray origin is inside of the solid rectangle.
            Some(RayHit2d::new(0.0, -ray.direction))
        } else if distance_far <= max_distance {
            // The ray hit the inside of the hollow rectangle.
            // Note: We could also just have an if-else here, but it was measured to be ~15% slower.
            let normal_abs = Vec2::from(t2.cmple(Vec2::splat(distance_far)));
            let normal = Dir2::new(-ray.direction.signum() * normal_abs).ok()?;
            Some(RayHit2d::new(distance_far, normal))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_rectangle() {
        let rectangle = Rectangle::new(2.0, 1.0);

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.0), Vec2::NEG_X);
        let hit = rectangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::X)));

        // Ray origin is inside of the solid rectangle.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = rectangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_X)));

        // Ray origin is inside of the hollow rectangle.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = rectangle.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::NEG_X)));

        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = rectangle.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::NEG_Y)));

        // Ray points away from the rectangle.
        assert!(!rectangle.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Vec2::Y)));
        assert!(!rectangle.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.0), Vec2::X)));
        assert!(!rectangle.intersects_local_ray(Ray2d::new(Vec2::new(0.0, -1.0), Vec2::X)));
        assert!(!rectangle.intersects_local_ray(Ray2d::new(Vec2::new(2.0, 0.0), Vec2::Y)));
        assert!(!rectangle.intersects_local_ray(Ray2d::new(Vec2::new(-2.0, 0.0), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Vec2::NEG_Y);
        let hit = rectangle.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
