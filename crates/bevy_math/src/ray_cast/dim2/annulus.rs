use ops::FloatPow;

use crate::prelude::*;

impl PrimitiveRayCast2d for Annulus {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        let length_squared = ray.origin.length_squared();
        let inner_radius_squared = self.inner_circle.radius.squared();

        // Squared distance between ray origin and inner circle boundary
        let inner_circle_distance_squared = length_squared - inner_radius_squared;

        if inner_circle_distance_squared < 0.0 {
            // The ray origin is inside of the inner circle, the "hole".
            //
            // This is equivalent to a ray-circle intersection test where the ray origin
            // is inside of the hollow circle. See the `Circle` ray casting implementation.

            let b = ray.origin.dot(*ray.direction);
            let d = b.squared() - inner_circle_distance_squared;
            let t = -b + d.sqrt();

            if t < max_distance {
                let intersection = ray.get_point(t);
                let direction = Dir2::new_unchecked(-intersection / self.inner_circle.radius);
                return Some(RayHit2d::new(t, direction));
            }
        } else if length_squared < self.outer_circle.radius.squared() {
            // The ray origin is inside of the annulus, in the area between the inner and outer circle.
            if solid {
                return Some(RayHit2d::new(0.0, -ray.direction));
            } else if let Some(hit) = self.inner_circle.local_ray_cast(ray, max_distance, solid) {
                return Some(hit);
            }
        }

        self.outer_circle.local_ray_cast(ray, max_distance, solid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_annulus() {
        let annulus = Annulus::new(0.5, 1.0);

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.0), Vec2::NEG_X);
        let hit = annulus.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::X)));

        // Ray origin is inside of the hole (smaller circle).
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = annulus.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::NEG_X)));

        // Ray origin is inside of the solid annulus.
        let ray = Ray2d::new(Vec2::new(0.75, 0.0), Vec2::X);
        let hit = annulus.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_X)));

        // Ray origin is inside of the hollow annulus.
        let ray = Ray2d::new(Vec2::new(0.75, 0.0), Vec2::X);
        let hit = annulus.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(0.25, Dir2::NEG_X)));

        // Ray points away from the annulus.
        assert!(!annulus.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Vec2::NEG_Y);
        let hit = annulus.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
