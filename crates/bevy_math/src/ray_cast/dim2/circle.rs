use ops::FloatPow;

use crate::prelude::*;

impl RayCast2d for Circle {
    #[inline]
    fn local_ray_distance(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<f32> {
        local_ray_distance_with_circle(self.radius, ray, solid)
            .and_then(|(distance, _)| (distance <= max_distance).then_some(distance))
    }

    #[inline]
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        local_ray_distance_with_circle(self.radius, ray, solid).and_then(|(distance, is_inside)| {
            if solid && is_inside {
                Some(RayHit2d::new(0.0, -ray.direction))
            } else if distance <= max_distance {
                let point = ray.get_point(distance);
                let normal = if is_inside {
                    Dir2::new_unchecked(-point / self.radius)
                } else {
                    Dir2::new_unchecked(point / self.radius)
                };
                Some(RayHit2d::new(distance, normal))
            } else {
                None
            }
        })
    }
}

#[inline]
fn local_ray_distance_with_circle(radius: f32, ray: Ray2d, solid: bool) -> Option<(f32, bool)> {
    // The function representing any point on a ray is:
    //
    // P(t) = O + tD
    //
    // where O is the ray origin and D is the ray direction. We need to find the value t
    // that represents the distance at which the ray intersects the sphere.
    //
    // Spherical shapes can be represented with the following implicit equations:
    //
    // Circle: x^2 + y^2 = R^2
    // Sphere: x^2 + y^2 + z^2 = R^2
    //
    // Representing the coordinates with a point P, we get an implicit function:
    //
    // length_squared(P) - R^2 = 0
    //
    // Substituting P for the equation of a ray:
    //
    // length_squared(O + tD) - R^2 = 0
    //
    // Expanding this equation, we get:
    //
    // length_squared(D) * t^2 + 2 * dot(O, D) * t + length_squared(O) - R^2 = 0
    //
    // This is a quadratic equation with:
    //
    // a = length_squared(D) = 1 (the ray direction is normalized)
    // b = 2 * dot(O, D)
    // c = length_squared(O) - R^2
    //
    // The discriminant is d = b^2 - 4ac = b^2 - 4c.
    //
    // 1. If d < 0, there is no valid solution, and the ray does not intersect the sphere.
    // 2. If d = 0, there is one root given by t = -b / 2a. With limited precision, we can ignore this case.
    // 3. If d > 0, we get two roots.
    //
    // The two roots for case (3) are:
    //
    // t1 = (-b + sqrt(d)) / 2a = (-b + sqrt(d)) / 2
    // t2 = (-b - sqrt(d)) / 2a = (-b - sqrt(d)) / 2
    //
    // If a root is negative, the intersection is behind the ray's origin and therefore ignored.
    //
    // We can actually simplify the computations further with:
    //
    // b = dot(O, D)
    // d = b^2 - c
    // t1 = -b + sqrt(d)
    // t2 = -b - sqrt(d)
    //
    // Proof, denoting the original variables with _o and the simplified versions with _s:
    //
    //                                                t1_o = t1_s
    //                              (-b_o + sqrt(d_o)) / 2 = -b_s + sqrt(d_s)
    // (-2 * dot(O, D) + sqrt((2 * dot(O, D))^2 - 4c)) / 2 = -dot(O, D) + sqrt(dot(O, D)^2 - c)
    //         -2 * dot(O, D) + sqrt(4 * dot(O, D)^2 - 4c) = -2 * dot(O, D) + 2 * sqrt(dot(O, D)^2 - c)
    //                          sqrt(4 * dot(O, D)^2 - 4c) = 2 * sqrt(dot(O, D)^2 - c)
    //                          sqrt(4 * dot(O, D)^2 - 4c) = sqrt(4 * (dot(O, D)^2 - c))
    //                          sqrt(4 * dot(O, D)^2 - 4c) = sqrt(4 * dot(O, D)^2 - 4c)

    // The squared distance between the ray origin and the boundary of the circle.
    let c = ray.origin.length_squared() - radius.squared();

    if c > 0.0 {
        // The ray origin is outside of the ball.
        let b = ray.origin.dot(*ray.direction);

        if b > 0.0 {
            // The ray points away from the circle, so there can be no hits.
            return None;
        }

        // The distance corresponding to the boundary hit is the second root.
        let d = b.squared() - c;
        let t2 = -b - d.sqrt();

        Some((t2, false))
    } else if solid {
        // The ray origin is inside of the solid circle.
        Some((0.0, true))
    } else {
        // The ray origin is inside of the hollow circle.
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
    fn local_ray_cast_circle() {
        let circle = Circle::new(1.0);

        // Ray origin is outside of the shape.
        let ray = Ray2d::new(Vec2::new(2.0, 0.0), Vec2::NEG_X);
        let hit = circle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::X)));

        // Ray origin is inside of the solid circle.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = circle.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit2d::new(0.0, Dir2::NEG_X)));

        // Ray origin is inside of the hollow circle.
        let ray = Ray2d::new(Vec2::ZERO, Vec2::X);
        let hit = circle.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit2d::new(1.0, Dir2::NEG_X)));

        // Ray points away from the circle.
        assert!(!circle.intersects_local_ray(Ray2d::new(Vec2::new(0.0, 2.0), Vec2::Y)));

        // Hit distance exceeds max distance.
        let ray = Ray2d::new(Vec2::new(0.0, 2.0), Vec2::NEG_Y);
        let hit = circle.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
