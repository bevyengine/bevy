use ops::FloatPow;

use crate::prelude::*;

// This is the same as `Sphere`, but with 3D types.
impl PrimitiveRayCast3d for Sphere {
    #[inline]
    fn local_ray_distance(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<f32> {
        local_ray_distance_with_sphere(self.radius, ray, solid)
            .and_then(|(distance, _)| (distance <= max_distance).then_some(distance))
    }

    #[inline]
    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<RayHit3d> {
        local_ray_distance_with_sphere(self.radius, ray, solid).and_then(|(distance, is_inside)| {
            if solid && is_inside {
                Some(RayHit3d::new(0.0, -ray.direction))
            } else if distance <= max_distance {
                let point = ray.get_point(distance);
                let normal = if is_inside {
                    Dir3::new_unchecked(-point / self.radius)
                } else {
                    Dir3::new_unchecked(point / self.radius)
                };
                Some(RayHit3d::new(distance, normal))
            } else {
                None
            }
        })
    }
}

#[inline]
fn local_ray_distance_with_sphere(radius: f32, ray: Ray3d, solid: bool) -> Option<(f32, bool)> {
    // See `Circle` for the math and detailed explanation of how this works.

    // The squared distance between the ray origin and the boundary of the sphere.
    let c = ray.origin.length_squared() - radius.squared();

    if c > 0.0 {
        // The ray origin is outside of the sphere.
        let b = ray.origin.dot(*ray.direction);

        if b > 0.0 {
            // The ray points away from the sphere, so there can be no hits.
            return None;
        }

        // The distance corresponding to the boundary hit is the second root.
        let d = b.squared() - c;
        let t2 = -b - d.sqrt();

        Some((t2, false))
    } else if solid {
        // The ray origin is inside of the solid sphere.
        Some((0.0, true))
    } else {
        // The ray origin is inside of the hollow sphere.
        // The distance corresponding to the boundary hit is the first root.
        let b = ray.origin.dot(*ray.direction);
        let d = b.squared() - c;
        let t1 = -b + d.sqrt();
        Some((t1, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_sphere() {
        let sphere = Sphere::new(1.0);

        // Ray origin is outside of the shape.
        let ray = Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::NEG_X);
        let hit = sphere.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::X)));

        // Ray origin is inside of the solid sphere.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = sphere.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(0.0, Dir3::NEG_X)));

        // Ray origin is inside of the hollow sphere.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = sphere.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::NEG_X)));

        // Ray points away from the sphere.
        assert!(!sphere.intersects_local_ray(Ray3d::new(Vec3::new(0.0, 2.0, 0.0), Vec3::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray3d::new(Vec3::new(0.0, 2.0, 0.0), Vec3::NEG_Y);
        let hit = sphere.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
