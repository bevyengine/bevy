use crate::prelude::*;

// This is mostly the same as `Capsule3d`, but with 2D types.
impl PrimitiveRayCast2d for Capsule2d {
    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        // Adapted from Inigo Quilez's ray-capsule intersection algorithm: https://iquilezles.org/articles/intersectors/

        let radius_squared = self.radius * self.radius;

        let ba = 2.0 * self.half_length;
        let oa = Vec2::new(ray.origin.x, ray.origin.y + self.half_length);

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
            let is_inside_rect_horizontal = c < 0.0;
            let is_inside_rect_vertical = ray.origin.y.abs() < self.half_length;
            let intersects_hemisphere = is_inside_rect_horizontal && {
                // The ray origin intersects one of the hemicircles if the distance
                // between the ray origin and hemicircle center is negative.
                Vec2::new(ray.origin.x, self.half_length - ray.origin.y.abs()).length_squared()
                    < radius_squared
            };
            let is_origin_inside =
                intersects_hemisphere || (is_inside_rect_horizontal && is_inside_rect_vertical);

            if solid && is_origin_inside {
                return Some(RayHit2d::new(0.0, -ray.direction));
            }

            let t = if is_origin_inside {
                (-b + d.sqrt()) / a
            } else {
                (-b - d.sqrt()) / a
            };

            let y = baoa + t * bard;

            // Check if the ray hit the rectangular part.
            let hit_rectangle = y > 0.0 && y < baba;
            if hit_rectangle && t > 0.0 {
                if t > max_distance {
                    return None;
                }

                // The ray hit the side of the rectangle.
                let normal = Dir2::new_unchecked(Vec2::new(-ray.direction.x.signum(), 0.0));
                return Some(RayHit2d::new(t, normal));
            }

            // Next, we check the hemicircles for intersections.
            // It's enough to only check one hemicircle and just take the side into account.

            // Offset between the ray origin and the center of the hit hemicircle.
            let offset_ray = Ray2d {
                origin: if y <= 0.0 {
                    oa
                } else {
                    Vec2::new(ray.origin.x, ray.origin.y - self.half_length)
                },
                direction: ray.direction,
            };

            // See `Circle` ray casting implementation.

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
                    Dir2::new_unchecked(-offset_ray.get_point(t2) / self.radius)
                } else {
                    Dir2::new_unchecked(offset_ray.get_point(t2) / self.radius)
                };
                return Some(RayHit2d::new(t2, dir));
            }

            // The ray hit the hemisphere that the ray origin is in.
            // The distance corresponding to the boundary hit is the first root.
            let t1 = -b + d_sqrt;

            if t1 > max_distance {
                return None;
            }

            let dir = Dir2::new_unchecked(-offset_ray.get_point(t1) / self.radius);
            return Some(RayHit2d::new(t1, dir));
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
    fn local_ray_cast_capsule_2d() {
        let capsule = Capsule2d::new(1.0, 2.0);

        // The Y coordinate corresponding to the angle PI/4 on a circle with the capsule's radius.
        let circle_frac_pi_4_y = capsule.radius * SQRT_2 / 2.0;

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.0), Vec2::NEG_X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::X)));

        let ray = Ray2d::new(Vec2::new(-2.0, 1.0 + circle_frac_pi_4_y), Vec2::X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_eq!(hit.distance, 1.0 + capsule.radius - circle_frac_pi_4_y);
        assert_relative_eq!(hit.normal, Dir2::NORTH_WEST);

        // Ray origin is inside of the solid capsule.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_X)));

        // Ray origin is inside of the hollow capsule.
        // Test three cases: inside the rectangle, inside the top hemicircle, and inside the bottom hemicircle.

        // Inside the rectangle.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::NEG_X)));

        let ray = Ray2d::new(Vec2::ZERO, Vec2::Y);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(2.0, Dir2::NEG_Y)));

        // Inside the top hemicircle.
        let ray = Ray2d::new(Vec2::new(0.0, 1.0), *Dir2::NORTH_EAST);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false).unwrap();
        assert_eq!(hit.distance, 1.0);
        assert_relative_eq!(hit.normal, Dir2::SOUTH_WEST);

        let ray = Ray2d::new(Vec2::new(0.0, 1.0), Vec2::NEG_Y);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(3.0, Dir2::Y)));

        // Inside the bottom hemicircle.
        let ray = Ray2d::new(Vec2::new(0.0, -1.0), *Dir2::SOUTH_WEST);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false).unwrap();
        assert_eq!(hit.distance, 1.0);
        assert_relative_eq!(hit.normal, Dir2::NORTH_EAST);

        let ray = Ray2d::new(Vec2::new(0.0, -1.0), Vec2::Y);
        let hit = capsule.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(3.0, Dir2::NEG_Y)));

        // Ray points away from the capsule.
        assert!(!capsule.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.1), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.6), Vec2::NEG_Y);
        let hit = capsule.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
