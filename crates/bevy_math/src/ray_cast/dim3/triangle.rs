use crate::prelude::*;

impl RayCast3d for Triangle3d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, _solid: bool) -> Option<RayHit3d> {
        // Adapted from:
        // - Inigo Quilez's algorithm: https://iquilezles.org/articles/intersectors/
        // - Möller-Trumbore ray-triangle intersection algorithm: https://en.wikipedia.org/wiki/Möller-Trumbore_intersection_algorithm
        //
        // NOTE: This implementation does not handle rays that are coplanar with the triangle.

        let [a, b, c] = self.vertices;

        // Edges from vertex A to B and C
        let ab = b - a;
        let ac = c - a;

        // Triangle normal using right-hand rule, assuming CCW winding.
        let n = ab.cross(ac);
        let det = n.dot(*ray.direction);

        // This check is important for robustness, and also seems to improve performance.
        if det == 0.0 {
            // The triangle normal and ray direction are perpendicular.
            return None;
        }

        // Note: Here we could check whether the ray intersects the half-space defined by the triangle,
        //       but the branching just seems to regress performance.

        let ao = ray.origin - a;

        // Note: For some reason, the compiler produces significantly more optimized instructions
        //       with these specific operations instead of ao.cross(*ray.direction).
        let ray_normal = -ray.direction.cross(ao);

        // To check if there is an intersection, we compute the barycentric coordinates (u, v, w).
        // w can be computed based on u and v because u + v + w = 1.
        let inv_det = det.recip();
        let u = -inv_det * ac.dot(ray_normal);
        let v = inv_det * ab.dot(ray_normal);

        // All barycentric coordinates of a point must be positive for it to be within the shape.
        if u < 0.0 || v < 0.0 || u + v > 1.0 {
            return None;
        }

        // Minimum signed distance between the ray origin and the triangle plane.
        let signed_origin_distance = ao.dot(n);

        // Take the ray direction into account.
        let distance = -inv_det * signed_origin_distance;

        // Note: Computing this here seems to be faster than doing it inside of the branch below.
        let normal = Dir3::new(signed_origin_distance.signum() * n).ok()?;

        (distance > 0.0 && distance <= max_distance).then_some(RayHit3d::new(distance, normal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::SQRT_2;

    #[test]
    fn local_ray_cast_triangle_3d() {
        let triangle = Triangle3d::new(
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
        );

        // Hit from above.
        let ray = Ray3d::new(Vec3::new(-0.5, 1.0, 0.0), Vec3::NEG_Y);
        let hit = triangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::Y)));

        let ray = Ray3d::new(Vec3::new(0.5, 1.0, 0.0), Vec3::new(-1.0, -1.0, 0.0));
        let hit = triangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(SQRT_2, Dir3::Y)));

        // Hit from below.
        let ray = Ray3d::new(Vec3::new(-0.5, -1.0, 0.0), Vec3::Y);
        let hit = triangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::NEG_Y)));

        let ray = Ray3d::new(Vec3::new(-1.5, -1.0, 0.0), Vec3::new(1.0, 1.0, 0.0));
        let hit = triangle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(SQRT_2, Dir3::NEG_Y)));

        // Ray points away from the triangle.
        assert!(!triangle.intersects_local_ray(Ray3d::new(Vec3::new(0.6, 1.0, 0.0), Vec3::NEG_Y)));

        // Hit distance exceeds max distance.
        let ray = Ray3d::new(Vec3::new(-0.5, 1.0, 0.0), Vec3::NEG_Y);
        let hit = triangle.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
