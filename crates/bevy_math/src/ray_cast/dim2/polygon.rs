use crate::prelude::*;

// TODO: Polygons should probably have their own type for this along with a BVH acceleration structure.

impl<const N: usize> RayCast2d for Polygon<N> {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        local_ray_cast_polygon(&self.vertices, ray, max_distance, solid)
    }
}

impl RayCast2d for BoxedPolygon {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        local_ray_cast_polygon(&self.vertices, ray, max_distance, solid)
    }
}

impl RayCast2d for RegularPolygon {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        let rot = Rot2::radians(self.external_angle_radians());

        let mut vertex1 = Vec2::new(0.0, self.circumradius());
        let mut vertex2;

        let mut closest_hit: Option<RayHit2d> = None;
        let mut hit_any = false;

        for _ in 0..self.sides {
            vertex2 = rot * vertex1;
            let (segment, translation) = Segment2d::from_points(vertex1, vertex2);
            if let Some(hit) = segment.ray_cast(
                Isometry2d::from_translation(translation),
                ray,
                max_distance,
                solid,
            ) {
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
    let mut closest_intersection: Option<RayHit2d> = None;
    let mut intersection_count = 0;

    // Iterate through vertices to create edges
    for i in 0..vertices.len() {
        let start = vertices[i];
        let end = if i == vertices.len() - 1 {
            // Connect the last vertex to the first vertex to close the polygon
            vertices[0]
        } else {
            vertices[i + 1]
        };

        // Create the edge
        let segment = Segment2d::new(Dir2::new(end - start).unwrap(), start.distance(end));

        // Cast the ray against the edge
        if let Some(intersection) = segment.ray_cast(
            Isometry2d::from_translation(start.midpoint(end)),
            ray,
            max_distance,
            true,
        ) {
            intersection_count += 1;
            if let Some(ref closest) = closest_intersection {
                if intersection.distance < closest.distance {
                    closest_intersection = Some(intersection);
                }
            } else {
                closest_intersection = Some(intersection);
            }
        }
    }

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
        let polygon = BoxedPolygon::new([
            Vec2::new(1.0, 0.5),
            Vec2::new(-1.0, 0.5),
            Vec2::new(-1.0, -0.5),
            Vec2::new(1.0, -0.5),
        ]);

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.0), Vec2::NEG_X);
        let hit = polygon.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::X)));

        // Ray origin is inside of the solid polygon.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = polygon.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_X)));

        // Ray origin is inside of the hollow polygon.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = polygon.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::NEG_X)));

        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = polygon.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(0.5, Dir2::NEG_Y)));

        // Ray points away from the polygon.
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Vec2::Y)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 1.0), Vec2::X)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(0.0, -1.0), Vec2::X)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(2.0, 0.0), Vec2::Y)));
        assert!(!polygon.intersects_local_ray(Ray2d::new(Vec2::new(-2.0, 0.0), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Vec2::NEG_Y);
        let hit = polygon.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
