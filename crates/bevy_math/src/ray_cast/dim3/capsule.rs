use crate::prelude::*;

// This is mostly the same as `Capsule2d`, but with 3D types.
impl RayCast3d for Capsule3d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<RayHit3d> {
        // Adapted from Inigo Quilez's ray-capsule intersection algorithm: https://iquilezles.org/articles/intersectors/

        let radius_squared = self.radius * self.radius;

        let ba = 2.0 * self.half_length;
        let oa = Vec3::new(ray.origin.x, ray.origin.y + self.half_length, ray.origin.z);

        let baba = ba * ba;
        let bard = ba * ray.direction.y;
        let baoa = ba * oa.y;
        let rdoa = ray.direction.dot(oa);
        let oaoa = oa.dot(oa);

        // Note: We use `f32::EPSILON` to avoid division by zero later for rays parallel to the capsule's axis.
        let a = (baba - bard * bard).max(f32::EPSILON);
        let b = baba * rdoa - baoa * bard;
        let c = baba * oaoa - baoa * baoa - radius_squared * baba;
        let d = b * b - a * c;

        if d >= 0.0 {
            let is_inside_cylinder_horizontal = c < 0.0;
            let is_inside_cylinder_vertical = ray.origin.y.abs() < self.half_length;
            let intersects_hemisphere = is_inside_cylinder_horizontal && {
                // The ray origin intersects one of the hemispheres if the distance
                // between the ray origin and hemisphere center is negative.
                Vec2::new(ray.origin.x, self.half_length - ray.origin.y.abs()).length_squared()
                    < radius_squared
            };
            let is_origin_inside = intersects_hemisphere
                || (is_inside_cylinder_horizontal && is_inside_cylinder_vertical);

            if solid && is_origin_inside {
                return Some(RayHit3d::new(0.0, -ray.direction));
            }

            let cylinder_distance = if is_origin_inside {
                (-b + d.sqrt()) / a
            } else {
                (-b - d.sqrt()) / a
            };

            let y = baoa + cylinder_distance * bard;

            // Check if the ray hit the cylindrical part.
            let hit_rectangle = y > 0.0 && y < baba;
            if hit_rectangle && cylinder_distance > 0.0 {
                if cylinder_distance > max_distance {
                    return None;
                }
                // The ray hit the side of the rectangle.
                let point = ray.get_point(cylinder_distance);
                let radius_recip = self.radius.recip();
                let normal = Dir3::new_unchecked(Vec3::new(
                    point.x.copysign(-ray.direction.x) * radius_recip,
                    0.0,
                    point.z.copysign(-ray.direction.z) * radius_recip,
                ));
                return Some(RayHit3d::new(cylinder_distance, normal));
            }

            // Next, we check the hemispheres for intersections.
            // It's enough to only check one hemisphere and just take the side into account.

            // Offset between the ray origin and the center of the hit hemisphere.
            let offset_ray = Ray3d {
                origin: if y <= 0.0 {
                    oa
                } else {
                    Vec3::new(ray.origin.x, ray.origin.y - self.half_length, ray.origin.z)
                },
                direction: ray.direction,
            };

            // See `Sphere` ray casting implementation.

            let b = offset_ray.origin.dot(*ray.direction);
            let c = offset_ray.origin.length_squared() - radius_squared;

            // No intersections if the ray direction points away from the ball and the ray origin is outside of the ball.
            if c > 0.0 && b > 0.0 {
                return None;
            }

            let d = b * b - c;

            if d < 0.0 {
                // No solution, no intersection.
                return None;
            }

            let d_sqrt = d.sqrt();

            let t2 = if is_origin_inside {
                -b + d_sqrt
            } else {
                -b - d_sqrt
            };

            if t2 > 0.0 && t2 <= max_distance {
                // The ray origin is outside of the hemisphere that was hit.
                let dir = if is_origin_inside {
                    Dir3::new_unchecked(-offset_ray.get_point(t2) / self.radius)
                } else {
                    Dir3::new_unchecked(offset_ray.get_point(t2) / self.radius)
                };
                return Some(RayHit3d::new(t2, dir));
            }

            // The ray hit the hemisphere that the ray origin is in.
            // The distance corresponding to the boundary hit is the first root.
            let t1 = -b + d_sqrt;

            if t1 > max_distance {
                return None;
            }

            let dir = Dir3::new_unchecked(-offset_ray.get_point(t1) / self.radius);
            return Some(RayHit3d::new(t1, dir));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use core::f32::consts::SQRT_2;

    #[test]
    fn local_ray_cast_capsule_3d() {
        let capsule = Capsule3d::new(1.0, 2.0);

        // The Y coordinate corresponding to the angle PI/4 on a circle with the capsule's radius.
        let circle_frac_pi_4_y = capsule.radius * SQRT_2 / 2.0;

        // Ray origin is outside of the shape.
        let ray = Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::NEG_X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::X)));

        let ray = Ray3d::new(Vec3::new(-2.0, 1.0 + circle_frac_pi_4_y, 0.0), Vec3::X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_eq!(hit.distance, 1.0 + capsule.radius - circle_frac_pi_4_y);
        assert_relative_eq!(hit.normal, Dir3::from_xyz(-1.0, 1.0, 0.0).unwrap());

        // Ray origin is inside of the solid capsule.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(0.0, Dir3::NEG_X)));

        // Ray origin is inside of the hollow capsule.
        // Test three cases: inside the rectangle, inside the top hemisphere, and inside the bottom hemisphere.

        // Inside the rectangle.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::NEG_X)));

        let ray = Ray3d::new(Vec3::ZERO, Vec3::Y);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(2.0, Dir3::NEG_Y)));

        // Inside the top hemisphere.
        let ray = Ray3d::new(
            Vec3::new(0.0, 1.0, 0.0),
            *Dir3::from_xyz(1.0, 1.0, 0.0).unwrap(),
        );
        let hit = capsule.local_ray_cast(ray, f32::MAX, false).unwrap();
        assert_eq!(hit.distance, 1.0);
        assert_relative_eq!(hit.normal, Dir3::from_xyz(-1.0, -1.0, 0.0).unwrap());

        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::NEG_Y);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(3.0, Dir3::Y)));

        // Inside the bottom hemisphere.
        let ray = Ray3d::new(
            Vec3::new(0.0, -1.0, 0.0),
            *Dir3::from_xyz(-1.0, -1.0, 0.0).unwrap(),
        );
        let hit = capsule.local_ray_cast(ray, f32::MAX, false).unwrap();
        assert_eq!(hit.distance, 1.0);
        assert_relative_eq!(hit.normal, Dir3::from_xyz(1.0, 1.0, 0.0).unwrap());

        let ray = Ray3d::new(Vec3::new(0.0, -1.0, 0.0), Vec3::Y);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(3.0, Dir3::NEG_Y)));

        // Ray points away from the capsule.
        assert!(!capsule.intersects_local_ray(Ray3d::new(Vec3::new(0.0, 2.1, 0.0), Vec3::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray3d::new(Vec3::new(0.0, 2.6, 0.0), Vec3::NEG_Y);
        let hit = capsule.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
