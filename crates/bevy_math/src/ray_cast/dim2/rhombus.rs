use crate::prelude::*;

impl RayCast2d for Rhombus {
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        // First, if the segment is solid, check if the ray origin is inside of it.
        if solid
            && ray.origin.x.abs() / self.half_diagonals.x
                + ray.origin.y.abs() / self.half_diagonals.y
                <= 1.0
        {
            return Some(RayHit2d::new(0.0, -ray.direction));
        }

        let mut closest: Option<RayHit2d> = None;

        let top = Vec2::new(0.0, self.half_diagonals.y);
        let bottom = Vec2::new(0.0, -self.half_diagonals.y);
        let left = Vec2::new(-self.half_diagonals.x, 0.0);
        let right = Vec2::new(self.half_diagonals.x, 0.0);

        let edges = [(top, left), (bottom, right), (top, right), (bottom, left)];
        let mut hit_any = false;

        // Check edges for intersections. There can be either zero or two intersections.
        for (start, end) in edges.into_iter() {
            let difference = end - start;
            let length = difference.length();
            let segment = Segment2d::new(Dir2::new_unchecked(difference / length), length);

            if let Some(intersection) = segment.ray_cast(
                Isometry2d::from_translation(start.midpoint(end)),
                ray,
                max_distance,
                true,
            ) {
                if closest.is_none() || intersection.distance < closest.unwrap().distance {
                    closest = Some(intersection);

                    if hit_any {
                        // This is the second intersection, the exit point.
                        // There can be no more intersections.
                        break;
                    }

                    hit_any = true;
                }
            }
        }

        closest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use core::f32::consts::SQRT_2;

    #[test]
    fn local_ray_cast_rhombus() {
        let rhombus = Rhombus::new(2.0, 2.0);

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.5), Vec2::NEG_X);
        let hit = rhombus.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_eq!(hit.distance, 1.5);
        assert_relative_eq!(hit.normal, Dir2::NORTH_EAST);

        // Ray origin is inside of the solid rhombus.
        let ray = Ray2d::new(Vec2::ZERO, *Dir2::NORTH_EAST);
        let hit = rhombus.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_eq!(hit.distance, 0.0);
        assert_relative_eq!(hit.normal, Dir2::SOUTH_WEST);

        // Ray origin is inside of the hollow rhombus.
        let ray = Ray2d::new(Vec2::ZERO, *Dir2::NORTH_EAST);
        let hit = rhombus.local_ray_cast(ray, f32::MAX, false).unwrap();
        assert_eq!(hit.distance, SQRT_2 / 2.0);
        assert_relative_eq!(hit.normal, Dir2::SOUTH_WEST);

        // Ray points away from the rhombus.
        assert!(!rhombus.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Vec2::NEG_Y);
        let hit = rhombus.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
