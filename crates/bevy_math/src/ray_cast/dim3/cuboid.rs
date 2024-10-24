use crate::prelude::*;

// This is the same as `Rectangle`, but with 3D types.
impl PrimitiveRayCast3d for Cuboid {
    #[inline]
    fn local_ray_distance(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<f32> {
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
    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<RayHit3d> {
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
            let normal_abs = Vec3::from(Vec3::splat(distance_near).cmple(t1));
            let normal = Dir3::new_unchecked(-ray.direction.signum() * normal_abs);
            Some(RayHit3d::new(distance_near, normal))
        } else if solid {
            // The ray origin is inside of the solid rectangle.
            Some(RayHit3d::new(0.0, -ray.direction))
        } else if distance_far <= max_distance {
            // The ray hit the inside of the hollow rectangle.
            // Note: We could also just have an if-else here, but it was measured to be ~15% slower.
            let normal_abs = Vec3::from(t2.cmple(Vec3::splat(distance_far)));
            let normal = Dir3::new_unchecked(-ray.direction.signum() * normal_abs);
            Some(RayHit3d::new(distance_far, normal))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_cuboid() {
        let cuboid = Cuboid::new(2.0, 1.0, 0.5);

        // Ray origin is outside of the shape.
        let ray = Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::NEG_X);
        let hit = cuboid.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::X)));

        // Ray origin is inside of the solid cuboid.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = cuboid.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(0.0, Dir3::NEG_X)));

        // Ray origin is inside of the hollow cuboid.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = cuboid.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::NEG_X)));

        let ray = Ray3d::new(Vec3::ZERO, Vec3::Y);
        let hit = cuboid.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(0.5, Dir3::NEG_Y)));

        // Ray points away from the cuboid.
        assert!(!cuboid.intersects_local_ray(Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::Y)));
        assert!(!cuboid.intersects_local_ray(Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::X)));
        assert!(!cuboid.intersects_local_ray(Ray3d::new(Vec3::new(0.0, -1.0, 0.0), Vec3::X)));
        assert!(!cuboid.intersects_local_ray(Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::Z)));
        assert!(!cuboid.intersects_local_ray(Ray3d::new(Vec3::new(0.0, -1.0, 0.0), Vec3::Z)));
        assert!(!cuboid.intersects_local_ray(Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::Y)));
        assert!(!cuboid.intersects_local_ray(Ray3d::new(Vec3::new(-2.0, 0.0, 0.0), Vec3::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray3d::new(Vec3::new(0.0, 2.0, 0.0), Vec3::NEG_Y);
        let hit = cuboid.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
