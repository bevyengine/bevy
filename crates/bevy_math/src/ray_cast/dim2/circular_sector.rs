use ops::FloatPow;

use crate::prelude::*;

impl PrimitiveRayCast2d for CircularSector {
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        // First, if the sector is solid, check if the ray origin is inside of it.
        if solid
            && ray.origin.length_squared() < self.radius().squared()
            && ops::abs(ray.origin.angle_to(Vec2::Y)) < self.arc.half_angle
        {
            return Some(RayHit2d::new(0.0, -ray.direction));
        }

        // Check for intersections with the circular arc.
        let mut closest = self.arc.local_ray_cast(ray, max_distance, true);

        // Check for intersection with the line segment between the origin and the arc's first endpoint.
        let left_endpoint = self.arc.left_endpoint();
        let left_segment = Segment2d::new(Vec2::ZERO, left_endpoint);

        if let Some(intersection) = left_segment.local_ray_cast(ray, max_distance, true) {
            if let Some(closest) = closest.filter(|_| self.arc.is_minor()) {
                // If the arc is at most half of the circle and the ray is intersecting both the arc and the line segment,
                // we can return early with the closer hit, as the ray cannot also be intersecting the second line segment.
                return if closest.distance <= intersection.distance {
                    Some(closest)
                } else {
                    Some(intersection)
                };
            }
            closest = Some(intersection);
        }

        // Check for intersection with the line segment between the origin and the arc's second endpoint.
        // We can just flip the segment about the Y axis since the sides are symmetrical.
        let right_endpoint = self.arc.right_endpoint();
        let right_segment = Segment2d::new(Vec2::ZERO, right_endpoint);

        if let Some(intersection) = right_segment.local_ray_cast(ray, max_distance, true)
            && (closest.is_none() || intersection.distance < closest.unwrap().distance)
        {
            closest = Some(intersection);
        }

        closest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn local_ray_cast_sector() {
        let sector = CircularSector::new(1.0, PI / 4.0);

        // Ray points away from the circular sector.
        assert!(!sector.intersects_local_ray(Ray2d::new(Vec2::new(0.5, 0.2), Dir2::X)));
        assert!(!sector.intersects_local_ray(Ray2d::new(Vec2::new(0.0, -0.1), Dir2::NEG_Y)));
        assert!(!sector.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.1), Dir2::Y)));

        // Ray hits the circular sector.
        assert!(sector.intersects_local_ray(Ray2d::new(Vec2::new(0.5, 0.2), Dir2::NEG_X)));
        assert!(sector.intersects_local_ray(Ray2d::new(Vec2::new(0.0, -0.1), Dir2::Y)));
        assert!(sector.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.1), Dir2::NEG_Y)));

        // Check correct hit distance and normal for outside hits.
        let ray = Ray2d::new(Vec2::new(0.0, 0.0), Dir2::Y);
        let hit = sector.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::SOUTH_WEST)));

        let ray = Ray2d::new(Vec2::new(0.0, 1.5), Dir2::NEG_Y);
        let hit = sector.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::Y)));

        let ray = Ray2d::new(Vec2::new(-1.0, 0.0), Dir2::NORTH_EAST);
        let hit = sector.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(
            hit,
            Some(RayHit2d::new(
                // Half the distance between the leftmost and topmost points on a circle.
                ops::hypot(sector.radius(), sector.radius()) / 2.0,
                Dir2::SOUTH_WEST
            ))
        );

        // Interior hit for solid sector.
        let ray = Ray2d::new(Vec2::new(0.0, sector.apothem()), Dir2::Y);
        let hit = sector.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_Y)));

        // Interior hits for hollow sector.
        let ray = Ray2d::new(Vec2::new(0.0, 0.5), Dir2::Y);
        let hit = sector.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::NEG_Y)));

        let ray = Ray2d::new(Vec2::new(0.0, 1.0), Dir2::SOUTH_EAST);
        let hit = sector.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(
            hit,
            Some(RayHit2d::new(
                // Half the distance between the topmost and rightmost points on a circle.
                ops::hypot(sector.radius(), sector.radius()) / 2.0,
                Dir2::NORTH_WEST
            ))
        );

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Dir2::NEG_Y);
        let hit = sector.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
