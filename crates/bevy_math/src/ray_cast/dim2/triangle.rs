use crate::prelude::*;

impl PrimitiveRayCast2d for Triangle2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        let [a, b, c] = self.vertices;

        if solid {
            // First, check if the ray starts inside the triangle.
            let ab = b - a;
            let bc = c - b;
            let ca = a - c;

            // Compute the dot products between the edge normals and the offset from the ray origin to each corner.
            // If the dot product for an edge is positive, the ray origin is on the interior triangle side relative to that edge.
            let dot1 = ab.perp_dot(ray.origin - a);
            let dot2 = bc.perp_dot(ray.origin - b);
            let dot3 = ca.perp_dot(ray.origin - c);

            // If all three dot products are positive, the ray origin is guaranteed to be inside of the triangle.
            if dot1 > 0.0 && dot2 > 0.0 && dot3 > 0.0 {
                return Some(RayHit2d::new(0.0, -ray.direction));
            }
        }

        let mut closest_intersection: Option<RayHit2d> = None;

        // Ray cast against each edge to find the closest intersection, if one exists.
        for (start, end) in [(a, b), (b, c), (c, a)] {
            let segment = Segment2d::new(Dir2::new(end - start).unwrap(), start.distance(end));

            if let Some(intersection) = segment.ray_cast(
                Isometry2d::from_translation(start.midpoint(end)),
                ray,
                max_distance,
                true,
            ) {
                if let Some(ref closest) = closest_intersection {
                    if intersection.distance < closest.distance {
                        closest_intersection = Some(intersection);
                    }
                } else {
                    closest_intersection = Some(intersection);
                }
            }
        }

        closest_intersection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::SQRT_2;

    #[test]
    fn local_ray_cast_triangle_2d() {
        let triangle = Triangle2d::new(
            Vec2::new(0.0, 2.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
        );

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Vec2::X);
        let hit = triangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(2.0, Dir2::NEG_X)));

        let ray = Ray2d::new(Vec2::new(2.0, 2.0), *Dir2::SOUTH_WEST);
        let hit = triangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(SQRT_2, Dir2::NORTH_EAST)));

        // Ray origin is inside of the solid triangle.
        let ray = Ray2d::new(Vec2::splat(0.5), Vec2::X);
        let hit = triangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_X)));

        // Ray origin is inside of the hollow triangle.
        let ray = Ray2d::new(Vec2::new(0.5, 0.5), Vec2::NEG_Y);
        let hit = triangle.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::Y)));

        // Ray points away from the triangle.
        assert!(!triangle.intersects_local_ray(Ray2d::new(Vec2::new(1.0, -1.0), Vec2::NEG_Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(-1.0, 1.0), Vec2::X);
        let hit = triangle.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
