use crate::prelude::*;

// TODO: Polylines should probably have their own type for this along with a BVH acceleration structure.

impl PrimitiveRayCast2d for Polyline2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, _solid: bool) -> Option<RayHit2d> {
        local_ray_cast_polyline(&self.vertices, ray, max_distance)
    }
}

#[inline]
fn local_ray_cast_polyline(vertices: &[Vec2], ray: Ray2d, max_distance: f32) -> Option<RayHit2d> {
    vertices
        .array_windows::<2>()
        .map(|[start, end]| Segment2d::new(*start, *end))
        .filter_map(|segment| segment.local_ray_cast(ray, max_distance, true))
        .min_by_key(|hit| crate::FloatOrd(hit.distance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_polyline_2d() {
        let polyline = Polyline2d::new([
            Vec2::new(-6.0, -2.0),
            Vec2::new(-2.0, 2.0),
            Vec2::new(2.0, -2.0),
            Vec2::new(6.0, 2.0),
        ]);

        // Hit from above.
        let ray = Ray2d::new(Vec2::new(-4.0, 4.0), Dir2::NEG_Y);
        let hit = polyline
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(4.0, Dir2::NORTH_WEST);
        assert!(ops::abs(hit.distance - expected_hit.distance) < 0.000_001);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        let ray = Ray2d::new(Vec2::new(0.0, 4.0), Dir2::NEG_Y);
        let hit = polyline
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(4.0, Dir2::NORTH_EAST);
        assert_eq!(hit.distance, expected_hit.distance);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        // Hit from below.
        let ray = Ray2d::new(Vec2::new(-4.0, -4.0), Dir2::Y);
        let hit = polyline
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(4.0, Dir2::SOUTH_EAST);
        assert_eq!(hit.distance, expected_hit.distance);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        let ray = Ray2d::new(Vec2::new(0.0, -4.0), Dir2::Y);
        let hit = polyline
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(4.0, Dir2::SOUTH_WEST);
        assert!(ops::abs(hit.distance - expected_hit.distance) < 0.000_001);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        // Hit from the side.
        let ray = Ray2d::new(Vec2::new(-2.0, 0.0), Dir2::X);
        let hit = polyline
            .local_ray_cast(ray, f32::MAX, true)
            .expect("hit exists");
        let expected_hit = RayHit2d::new(2.0, Dir2::SOUTH_WEST);
        assert_eq!(hit.distance, expected_hit.distance);
        assert!(ops::abs(hit.normal.distance(*expected_hit.normal)) < 0.000_001);

        // Ray goes past the left endpoint.
        assert!(!polyline.intersects_local_ray(Ray2d::new(Vec2::new(-7.0, 2.0), Dir2::NEG_Y)));

        // Ray goes past the right endpoint.
        assert!(!polyline.intersects_local_ray(Ray2d::new(Vec2::new(7.0, -2.0), Dir2::Y)));

        // Ray points away from the polyline.
        assert!(!polyline.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 0.2), Dir2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(-2.0, 1.0), Dir2::NEG_Y);
        let hit = polyline.local_ray_cast(ray, 2.5, true);
        assert!(hit.is_none());
    }
}
