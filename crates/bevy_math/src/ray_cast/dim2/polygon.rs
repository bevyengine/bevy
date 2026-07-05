use crate::prelude::*;

// TODO: Polygons should probably have their own type for this along with a BVH acceleration structure.

impl PrimitiveRayCast2d for Polygon {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        local_ray_cast_polygon(&self.vertices, ray, max_distance, solid)
    }
}

impl PrimitiveRayCast2d for RegularPolygon {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        let rot = Rot2::radians(self.external_angle_radians());

        let mut vertex1 = Vec2::new(0.0, self.circumradius());
        let mut vertex2;

        let mut closest_hit: Option<RayHit2d> = None;
        let mut hit_any = false;

        for _ in 0..self.sides {
            vertex2 = rot * vertex1;
            let segment = Segment2d::new(vertex1, vertex2);
            if let Some(hit) = segment.local_ray_cast(ray, max_distance, solid) {
                if closest_hit.is_none() || hit.distance < closest_hit.unwrap().distance {
                    closest_hit = Some(hit);
                }

                if hit_any {
                    // This is the second intersection.
                    // There can be no more intersections.
                    return closest_hit;
                }

                hit_any = true;
            }
            vertex1 = vertex2;
        }

        // There are either zero or one intersections.
        if solid && hit_any {
            Some(RayHit2d::new(0.0, -ray.direction))
        } else {
            closest_hit
        }
    }
}

#[inline]
fn local_ray_cast_polygon(
    vertices: &[Vec2],
    ray: Ray2d,
    max_distance: f32,
    solid: bool,
) -> Option<RayHit2d> {
    let (closest_intersection, intersection_count) = vertices
        .array_windows::<2>()
        .copied()
        .chain(vertices.last().zip(vertices.first()).map(|(a, b)| [*a, *b]))
        .map(|[start, end]| Segment2d::new(start, end))
        .filter_map(|segment| segment.local_ray_cast(ray, max_distance, true))
        .fold((None::<RayHit2d>, 0), |(closest, hit_count), hit| {
            let closest_hit = if closest.is_some_and(|h| h.distance < hit.distance) {
                closest
            } else {
                Some(hit)
            };
            (closest_hit, hit_count + 1)
        });

    // check if the ray is inside the polygon
    if solid && intersection_count % 2 == 1 {
        Some(RayHit2d::new(0.0, -ray.direction))
    } else {
        closest_intersection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_polygon() {
        // Same as the rectangle test, but with a polygon shape.
        let polygon = Polygon::new([
            Vec2::new(1.0, 0.5),
            Vec2::new(-1.0, 0.5),
            Vec2::new(-1.0, -0.5),
            Vec2::new(1.0, -0.5),
        ]);

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.0), Dir2::NEG_X);
        let hit = polygon.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::X)));

        // Ray origin is inside of the solid polygon.
        let ray = Ray2d::new(Vec2::ZERO, Dir2::X);
        let hit = polygon.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_X)));

        // Ray origin is inside of the hollow polygon.
        let ray = Ray2d::new(Vec2::ZERO, Dir2::X);
        let hit = polygon.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::NEG_X)));

        let ray = Ray2d::new(Vec2::ZERO, Dir2::Y);
        let hit = polygon.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::NEG_Y)));

        // Ray points away from the polygon.
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Dir2::Y)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.0), Dir2::X)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(0.0, -1.0), Dir2::X)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(2.0, 0.0), Dir2::Y)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(-2.0, 0.0), Dir2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Dir2::NEG_Y);
        let hit = polygon.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
