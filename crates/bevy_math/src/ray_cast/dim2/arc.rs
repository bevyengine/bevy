use core::f32::consts::FRAC_PI_2;

use ops::FloatPow;

use crate::prelude::*;

impl PrimitiveRayCast2d for Arc2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, _solid: bool) -> Option<RayHit2d> {
        // Adapted from the `Circle` ray casting implementation.

        let b = ray.origin.dot(*ray.direction);
        let c = ray.origin.length_squared() - self.radius.squared();

        if c > 0.0 && b > 0.0 {
            // No intersections: The ray direction points away from the circle, and the ray origin is outside of the circle.
            return None;
        }

        let d = b.squared() - c;

        if d < 0.0 {
            // No solution, no intersections.
            return None;
        }

        let d_sqrt = d.sqrt();
        let t2 = -b - d_sqrt;

        if t2 > 0.0 && t2 <= max_distance {
            // The ray hit the outside of the arc.
            let p2 = ray.get_point(t2);
            let arc_bottom_y = self.radius * ops::sin(FRAC_PI_2 + self.half_angle);
            if p2.y >= arc_bottom_y {
                let normal = Dir2::new_unchecked(p2 / self.radius);
                return Some(RayHit2d::new(t2, normal));
            }
        }

        let t1 = -b + d_sqrt;
        if t1 <= max_distance {
            // The ray hit the inside of the arc.
            let p1 = ray.get_point(t1);
            let arc_bottom_y = self.radius * ops::sin(FRAC_PI_2 + self.half_angle);
            if p1.y >= arc_bottom_y {
                let normal = Dir2::new_unchecked(-p1 / self.radius);
                return Some(RayHit2d::new(t1, normal));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn local_ray_cast_arc() {
        let arc = Arc2d::new(1.0, PI / 4.0);

        // Ray points away from the arc.
        assert!(!arc.intersects_local_ray(Ray2d::new(Vec2::new(2.0, 0.25), Vec2::NEG_X)));
        assert!(!arc.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 0.9), Vec2::NEG_Y)));
        assert!(!arc.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.1), Vec2::Y)));

        // Ray hits the arc.
        assert!(arc.intersects_local_ray(Ray2d::new(Vec2::new(2.0, 0.75), Vec2::NEG_X)));
        assert!(arc.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 0.9), Vec2::Y)));
        assert!(arc.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.1), Vec2::NEG_Y)));

        // Check correct hit distance and normal.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = arc.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::NEG_Y)));

        let ray = Ray2d::new(Vec2::new(0.0, 1.5), Vec2::NEG_Y);
        let hit = arc.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = arc.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
