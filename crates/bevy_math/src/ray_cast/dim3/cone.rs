use crate::prelude::*;

// NOTE: This is largely a copy of the `ConicalFrustum` implementation, but simplified for only one base.
impl PrimitiveRayCast3d for Cone {
    #[inline]
    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<RayHit3d> {
        // Adapted from:
        // - Inigo Quilez's capped cone ray intersection algorithm: https://iquilezles.org/articles/intersectors/
        // - http://lousodrome.net/blog/light/2017/01/03/intersection-of-a-ray-and-a-cone/

        let half_height = self.height * 0.5;
        let height_squared = self.height * self.height;
        let radius_squared = self.radius * self.radius;

        let a = Vec3::new(0.0, half_height, 0.0);
        let b = -a;
        let ba = b - a;
        let oa = ray.origin - a;
        let ob = ray.origin - b;

        let oa_dot_ba = oa.dot(ba);
        let ob_dot_ba = ob.dot(ba);

        // The ray origin is inside of the cone if both of the following are true:
        // 1. The origin is between the top and bottom.
        // 2. The origin is within the circular slice determined by the distance from the base.
        let is_inside = oa_dot_ba >= 0.0 && oa_dot_ba <= height_squared && {
            // Compute the radius of the circular slice.
            // Derived geometrically from the triangular cross-section.
            //
            // let y = ob_dot_ba / self.height;
            // let slope = self.height / self.radius;
            //
            // let delta_radius = y / slope
            //                  = y / (self.height / self.radius)
            //                  = y * self.radius / self.height
            //                  = ob_dot_ba / self.height * self.radius / self.height
            //                  = ob_dot_ba * self.radius / (self.height * self.height);
            // let radius = self.radius + delta_radius;
            let delta_radius = ob_dot_ba * self.radius / height_squared;
            let radius = self.radius + delta_radius;

            // The squared orthogonal distance from the cone axis
            let ortho_distance_squared = ray.origin.xz().length_squared();

            ortho_distance_squared < radius * radius
        };

        if is_inside {
            if solid {
                return Some(RayHit3d::new(0.0, -ray.direction));
            }

            let dir_dot_ba = ray.direction.dot(ba);
            let dir_dot_oa = ray.direction.dot(oa);
            let top_distance_squared = oa.length_squared();

            // Base
            if oa_dot_ba >= 0.0 && dir_dot_ba > 0.0 {
                let distance = -ob_dot_ba / dir_dot_ba;

                if distance <= max_distance {
                    // Check if the point of intersection is within the bottom circle.
                    let distance_at_bottom_squared =
                        (ob + ray.direction * distance).length_squared();
                    if distance_at_bottom_squared < radius_squared {
                        // The ray hit the bottom of the cone.
                        let normal = -Dir3::new_unchecked(ba / self.height);
                        return Some(RayHit3d::new(distance, normal));
                    }
                }
            }

            // The ray hit the lateral surface of the cone.
            // Because the ray is known to be inside of the shape, no further checks are needed.

            let height_pow_4 = height_squared * height_squared;
            let hypot_squared = height_squared + radius_squared;

            // Quadratic equation coefficients a, b, c
            let a = height_pow_4 - dir_dot_ba * dir_dot_ba * hypot_squared;
            let b = height_pow_4 * dir_dot_oa - oa_dot_ba * dir_dot_ba * hypot_squared;
            let c = height_pow_4 * top_distance_squared - oa_dot_ba * oa_dot_ba * hypot_squared;
            let discriminant = b * b - a * c;

            // Two roots:
            // t1 = (-b - discriminant.sqrt()) / a
            // t2 = (-b + discriminant.sqrt()) / a
            // For the inside case, we want t2.
            let distance = (-b + discriminant.sqrt()) / a;

            if distance < 0.0 || distance > max_distance {
                return None;
            }

            // Squared distance from top along cone axis at the point of intersection
            let hit_y_squared = oa_dot_ba + distance * dir_dot_ba;

            let normal = -Dir3::new(
                height_pow_4 * (oa + distance * ray.direction) - ba * hypot_squared * hit_y_squared,
            )
            .ok()?;

            Some(RayHit3d::new(distance, normal))
        } else {
            // The ray origin is outside of the cone.

            let dir_dot_ba = ray.direction.dot(ba);
            let dir_dot_oa = ray.direction.dot(oa);
            let top_distance_squared = oa.length_squared();

            // Base
            if ob_dot_ba > 0.0 && dir_dot_ba < 0.0 {
                let distance = -ob_dot_ba / dir_dot_ba;

                if distance <= max_distance {
                    // Check if the point of intersection is within the bottom circle.
                    let distance_at_bottom_squared =
                        (ob + ray.direction * distance).length_squared();
                    if distance_at_bottom_squared < radius_squared {
                        // The ray hit the bottom of the cone.
                        let normal = Dir3::new_unchecked(ba / self.height);
                        return Some(RayHit3d::new(distance, normal));
                    }
                }
            }

            // Check for intersections with the lateral surface of the cone.

            let height_pow_4 = height_squared * height_squared;
            let hypot_squared = height_squared + radius_squared;

            // Quadratic equation coefficients a, b, c
            let a = height_pow_4 - dir_dot_ba * dir_dot_ba * hypot_squared;
            let b = height_pow_4 * dir_dot_oa - oa_dot_ba * dir_dot_ba * hypot_squared;
            let c = height_pow_4 * top_distance_squared - oa_dot_ba * oa_dot_ba * hypot_squared;
            let discriminant = b * b - a * c;

            if discriminant < 0.0 {
                return if ray.direction.y == -1.0 {
                    // Edge case: The ray is pointing straight down at the tip.
                    Some(RayHit3d::new(top_distance_squared.sqrt(), Dir3::Y))
                } else {
                    None
                };
            }

            // Two roots:
            // t1 = (-b - discriminant.sqrt()) / a
            // t2 = (-b + discriminant.sqrt()) / a
            // For the outside case, we want t1.
            let distance = (-b - discriminant.sqrt()) / a;

            if distance < 0.0 || distance > max_distance {
                return None;
            }

            // Squared distance from top along cone axis at the point of intersection
            let hit_y_squared = oa_dot_ba + distance * dir_dot_ba;

            if hit_y_squared < 0.0 || hit_y_squared > height_squared {
                // The point of intersection is outside of the height of the cone.
                return None;
            }

            let normal = Dir3::new(
                height_pow_4 * (oa + distance * ray.direction) - ba * hypot_squared * hit_y_squared,
            )
            .ok()?;

            Some(RayHit3d::new(distance, normal))
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn local_ray_cast_cone() {
        let cone = Cone::new(1.0, 2.0);

        // Ray origin is outside of the shape.
        let ray = Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::NEG_X);
        let hit = cone.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_eq!(hit.distance, 1.5);
        assert_relative_eq!(hit.normal, Dir3::from_xyz(2.0, 1.0, 0.0).unwrap());

        // Ray origin is inside of the solid cone.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = cone.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(0.0, Dir3::NEG_X)));

        // Ray origin is inside of the hollow cone.
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = cone.local_ray_cast(ray, f32::MAX, false).unwrap();
        assert_eq!(hit.distance, 0.5);
        assert_relative_eq!(hit.normal, Dir3::from_xyz(-2.0, -1.0, 0.0).unwrap());
        let ray = Ray3d::new(Vec3::ZERO, Vec3::NEG_Y);
        let hit = cone.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(1.0, Dir3::Y)));

        // Ray hits the cone.
        assert!(cone.intersects_local_ray(Ray3d::new(Vec3::new(0.0, 0.9, 0.0), Vec3::Y)));
        assert!(cone.intersects_local_ray(Ray3d::new(Vec3::new(0.4, 0.0, 0.0), Vec3::X)));

        // Ray points away from the cone.
        assert!(!cone.intersects_local_ray(Ray3d::new(Vec3::new(0.0, 1.1, 0.0), Vec3::Y)));
        assert!(!cone.intersects_local_ray(Ray3d::new(Vec3::new(0.6, 0.0, 0.0), Vec3::X)));

        // Edge case: The ray is pointing straight down at the tip.
        let ray = Ray3d::new(Vec3::new(0.0, 1.1, 0.0), Vec3::NEG_Y);
        let hit = cone.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_relative_eq!(hit.distance, 0.1);
        assert_eq!(hit.normal, Dir3::Y);

        // Hit distance exceeds max distance.
        let ray = Ray3d::new(Vec3::new(0.0, 2.0, 0.0), Vec3::NEG_Y);
        let hit = cone.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
