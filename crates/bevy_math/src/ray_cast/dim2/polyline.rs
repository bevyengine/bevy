use crate::prelude::*;

// TODO: Polylines should probably have their own type for this along with a BVH acceleration structure.

impl<const N: usize> RayCast2d for Polyline2d<N> {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, _solid: bool) -> Option<RayHit2d> {
        local_ray_cast_polyline(&self.vertices, ray, max_distance)
    }
}

impl RayCast2d for BoxedPolyline2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, _solid: bool) -> Option<RayHit2d> {
        local_ray_cast_polyline(&self.vertices, ray, max_distance)
    }
}

#[inline]
fn local_ray_cast_polyline(vertices: &[Vec2], ray: Ray2d, max_distance: f32) -> Option<RayHit2d> {
    let mut closest_intersection: Option<RayHit2d> = None;

    // Iterate through vertices to create edges
    for i in 0..(vertices.len() - 1) {
        let start = vertices[i];
        let end = vertices[i + 1];

        // Create the edge
        let segment = Segment2d::new(Dir2::new(end - start).unwrap(), start.distance(end));

        // Cast the ray against the edge
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_polyline_2d() {
        let polyline = BoxedPolyline2d::new([
            Vec2::new(-6.0, -2.0),
            Vec2::new(-2.0, 2.0),
            Vec2::new(2.0, -2.0),
            Vec2::new(6.0, 2.0),
        ]);

        // Hit from above.
        let ray = Ray2d::new(Vec2::new(-4.0, 4.0), Vec2::NEG_Y);
        let hit = polyline.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(4.0, Dir2::NORTH_WEST)));

        let ray = Ray2d::new(Vec2::new(0.0, 4.0), Vec2::NEG_Y);
        let hit = polyline.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(4.0, Dir2::NORTH_EAST)));

        // Hit from below.
        let ray = Ray2d::new(Vec2::new(-4.0, -4.0), Vec2::Y);
        let hit = polyline.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(4.0, Dir2::SOUTH_EAST)));

        let ray = Ray2d::new(Vec2::new(0.0, -4.0), Vec2::Y);
        let hit = polyline.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(4.0, Dir2::SOUTH_WEST)));

        // Hit from the side.
        let ray = Ray2d::new(Vec2::new(-2.0, 0.0), Vec2::X);
        let hit = polyline.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(2.0, Dir2::SOUTH_WEST)));

        // Ray goes past the left endpoint.
        assert!(!polyline.intersects_local_ray(Ray2d::new(Vec2::new(-7.0, 2.0), Vec2::NEG_Y)));

        // Ray goes past the right endpoint.
        assert!(!polyline.intersects_local_ray(Ray2d::new(Vec2::new(7.0, -2.0), Vec2::Y)));

        // Ray points away from the polyline.
        assert!(!polyline.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 0.2), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Vec2::NEG_Y);
        let hit = polyline.local_ray_cast(ray, 2.5, true);
        assert!(hit.is_none());
    }
}
