use ops::FloatPow;

use crate::prelude::*;

impl PrimitiveRayCast2d for CircularSegment {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        let start = self.arc.left_endpoint();
        let end = self.arc.right_endpoint();

        // First, if the segment is solid, check if the ray origin is inside of it.
        let is_inside = ray.origin.length_squared() < self.radius().squared()
            && ray.origin.y >= start.y.min(end.y);
        if solid && is_inside {
            return Some(RayHit2d::new(0.0, -ray.direction));
        }

        // Check for intersection with the circular arc.
        let mut closest = None;
        if let Some(intersection) = self.arc.local_ray_cast(ray, max_distance, true) {
            closest = Some(intersection);
        }

        // Check if the segment connecting the arc's endpoints is intersecting the ray.
        let segment = Segment2d::new(Dir2::new(end - start).unwrap(), 2.0 * self.radius());

        if !is_inside && ray.origin.y >= start.y.min(end.y) {
            // The ray is above the segment and cannot intersect with the segment.
            return closest;
        }

        if let Some(intersection) = segment.ray_cast(
            Isometry2d::from_translation(start.midpoint(end)),
            ray,
            max_distance,
            true,
        ) {
            if closest.is_none() || intersection.distance < closest.unwrap().distance {
                closest = Some(intersection);
            }
        }

        closest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn local_ray_cast_segment() {
        let segment = CircularSegment::new(1.0, PI / 4.0);

        // Ray points away from the circular segment.
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(2.0, 0.25), Vec2::NEG_X)));
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 0.5), Vec2::NEG_Y)));
        assert!(!segment.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.1), Vec2::Y)));

        // Ray hits the circular segment.
        assert!(segment.intersects_local_ray(Ray2d::new(Vec2::new(2.0, 0.75), Vec2::NEG_X)));
        assert!(segment.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 0.9), Vec2::Y)));
        assert!(segment.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.1), Vec2::NEG_Y)));

        // Check correct hit distance and normal for outside hits.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = segment.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(segment.apothem(), Dir2::NEG_Y)));

        let ray = Ray2d::new(Vec2::new(0.0, 1.5), Vec2::NEG_Y);
        let hit = segment.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::Y)));

        // Interior hit for solid segment.
        let ray = Ray2d::new(Vec2::new(0.0, segment.apothem()), Vec2::Y);
        let hit = segment.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_Y)));

        // Interior hit for hollow segment.
        let ray = Ray2d::new(Vec2::new(0.0, segment.apothem() + 0.01), Vec2::Y);
        let hit = segment.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(
            hit,
            Some(RayHit2d::new(segment.sagitta() - 0.01, Dir2::NEG_Y))
        );

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = segment.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
