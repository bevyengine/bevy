use crate::prelude::*;

impl PrimitiveRayCast2d for Ellipse {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        // Adapted from:
        // - The `Circle` ray casting implementation
        // - Inigo Quilez's ray-ellipse intersection algorithm: https://www.shadertoy.com/view/NdccWH

        // If the ellipse is just a circle, use the ray casting implemention from `Circle`.
        if self.half_size.x == self.half_size.y {
            return Circle::new(self.half_size.x).local_ray_cast(ray, max_distance, solid);
        }

        // Normalize the ray origin to the ellipse's half-size.
        let inv_half_size = self.half_size.recip();
        let origin_n = ray.origin * inv_half_size;

        // First, if the ellipse is solid, check if the ray origin is inside of it.
        if solid && origin_n.length_squared() < 1.0 {
            return Some(RayHit2d::new(0.0, -ray.direction));
        }

        // Normalize the ray direction to the ellipse's half-size.
        let direction_n = *ray.direction * inv_half_size;

        // Compute the terms of the quadratic equation (see circle ray casting),
        // but modified to simplify the computations.
        let a = direction_n.length_squared();
        let b = origin_n.dot(direction_n);
        let c = origin_n.length_squared();

        // Discriminant (modified)
        let d = b * b - a * (c - 1.0);

        if d < 0.0 {
            // No solution, no intersection.
            return None;
        }

        let d_sqrt = d.sqrt();

        // Compute the second root of the quadratic equation, a potential intersection.
        let t2 = (-b - d_sqrt) / a;
        if t2 > 0.0 && t2 < max_distance {
            // The ray origin is outside of the ellipse and a hit was found.
            // The distance corresponding to the boundary hit is the second root.
            let hit_point = ray.get_point(t2);
            let normal = Dir2::new_unchecked(hit_point * inv_half_size);
            Some(RayHit2d::new(t2, normal))
        } else {
            // The ray origin is inside of the hollow ellipse.
            // The distance corresponding to the boundary hit is the first root.
            let t1 = (-b + d_sqrt) / a;
            if t1 > 0.0 && t1 < max_distance {
                let hit_point = ray.get_point(t1);
                let normal = Dir2::new_unchecked(-hit_point * inv_half_size);
                Some(RayHit2d::new(t1, normal))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_ellipse() {
        let ellipse = Ellipse::new(1.0, 0.5);

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.0), Vec2::NEG_X);
        let hit = ellipse.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::X)));

        // Ray origin is inside of the solid ellipse.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = ellipse.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_Y)));

        // Ray origin is inside of the hollow ellipse.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = ellipse.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::NEG_Y)));

        // Ray points away from the ellipse.
        assert!(!ellipse.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Vec2::NEG_Y);
        let hit = ellipse.local_ray_cast(ray, 1.0, true);
        assert!(hit.is_none());
    }
}
