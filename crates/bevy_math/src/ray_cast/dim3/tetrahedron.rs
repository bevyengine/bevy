use crate::prelude::*;

impl PrimitiveRayCast3d for Tetrahedron {
    #[inline]
    fn intersects_local_ray(&self, ray: Ray3d) -> bool {
        // Tetrahedron-ray intersection test using scalar triple products.
        // An alternative could use Plücker coordinates, but that can be less efficient and more complex.
        //
        // Reference: https://realtimecollisiondetection.net/blog/?p=13

        // Translate the ray and the tetrahedron such that the ray origin is at (0, 0, 0).
        let q = *ray.direction;
        let v = self.vertices;
        let a = v[0] - ray.origin;
        let b = v[1] - ray.origin;
        let c = v[2] - ray.origin;
        let d = v[3] - ray.origin;

        // Determine if the origin is inside the tetrahedron using triple scalar products.
        let abc = triple_scalar_product(a, b, c);
        let abd = triple_scalar_product(a, b, d);
        let acd = triple_scalar_product(a, c, d);
        let bcd = triple_scalar_product(b, c, d);

        let ab = b - a;
        let ac = c - a;
        let ad = d - a;
        let sign = triple_scalar_product(ab, ac, ad).signum();

        let is_inside = if sign == 1.0 {
            abc <= 0.0 && abd >= 0.0 && acd <= 0.0 && bcd >= 0.0
        } else {
            abc > 0.0 && abd < 0.0 && acd > 0.0 && bcd < 0.0
        };

        if is_inside {
            return true;
        }

        let qab = sign * triple_scalar_product(q, a, b);
        let qbc = sign * triple_scalar_product(q, b, c);
        let qac = sign * triple_scalar_product(q, a, c);

        // ABC
        if qab >= 0.0 && qbc >= 0.0 && qac < 0.0 {
            return true;
        }

        let qad = sign * triple_scalar_product(q, a, d);
        let qbd = sign * triple_scalar_product(q, b, d);

        // BAD
        if qab < 0.0 && qad >= 0.0 && qbd < 0.0 {
            return true;
        }

        let qcd = sign * triple_scalar_product(q, c, d);

        // CDA
        if qcd >= 0.0 && qad < 0.0 && qac >= 0.0 {
            return true;
        }

        // DCB
        if qcd < 0.0 && qbc < 0.0 && qbd >= 0.0 {
            return true;
        }

        false
    }

    #[inline]
    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<RayHit3d> {
        // Tetrahedron-ray intersection test using scalar triple products.
        // An alternative could use Plücker coordinates, but that can be less efficient and more complex.
        //
        // Note: The ray cast could be much more efficient if we could assume a specific triangle orientation and ignore interior cases.
        //       There's likely room for optimization here.
        //
        // Reference: https://realtimecollisiondetection.net/blog/?p=13

        // Translate the ray and the tetrahedron such that the ray origin is at (0, 0, 0).
        let q = *ray.direction;
        let v = self.vertices;
        let a = v[0] - ray.origin;
        let b = v[1] - ray.origin;
        let c = v[2] - ray.origin;
        let d = v[3] - ray.origin;

        // Determine if the origin is inside the tetrahedron using triple scalar products.
        let abc = triple_scalar_product(a, b, c);
        let abd = triple_scalar_product(a, b, d);
        let acd = triple_scalar_product(a, c, d);
        let bcd = triple_scalar_product(b, c, d);

        // Get the sign of the signed volume of the tetrahedron, which determines the orientation.
        let ab = b - a;
        let ac = c - a;
        let ad = d - a;
        let orientation = triple_scalar_product(ab, ac, ad).signum();

        let is_inside = if orientation == 1.0 {
            abc <= 0.0 && abd >= 0.0 && acd <= 0.0 && bcd >= 0.0
        } else {
            abc > 0.0 && abd < 0.0 && acd > 0.0 && bcd < 0.0
        };

        if solid && is_inside {
            return Some(RayHit3d::new(0.0, -ray.direction));
        }

        let sign = if is_inside { -orientation } else { orientation };

        // Now, we check each face for intersections using scalar triple products.
        // The ray intersects a face if and only if the ray lies clockwise to each edge of the face.

        let qab = sign * triple_scalar_product(q, a, b);
        let qbc = sign * triple_scalar_product(q, b, c);
        let qac = sign * triple_scalar_product(q, a, c);

        // ABC
        if qab >= 0.0 && qbc >= 0.0 && qac < 0.0 {
            return Triangle3d::new(v[0], v[1], v[2]).local_ray_cast(ray, max_distance, solid);
        }

        let qad = sign * triple_scalar_product(q, a, d);
        let qbd = sign * triple_scalar_product(q, b, d);

        // BAD
        if qab < 0.0 && qad >= 0.0 && qbd < 0.0 {
            return Triangle3d::new(v[1], v[0], v[3]).local_ray_cast(ray, max_distance, solid);
        }

        let qcd = sign * triple_scalar_product(q, c, d);

        // CDA
        if qcd >= 0.0 && qad < 0.0 && qac >= 0.0 {
            return Triangle3d::new(v[2], v[3], v[0]).local_ray_cast(ray, max_distance, solid);
        }

        // DCB
        if qcd < 0.0 && qbc < 0.0 && qbd >= 0.0 {
            return Triangle3d::new(v[3], v[2], v[1]).local_ray_cast(ray, max_distance, solid);
        }

        None
    }
}

#[inline]
fn triple_scalar_product(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    // Glam can optimize this better than a.dot(b.cross(c))
    Mat3::from_cols(a, b, c).determinant()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ray_cast_tetrahedron() {
        let tetrahedron = Tetrahedron::new(
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(-1.0, 2.0, -1.0),
        );

        // Hit from above.
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::NEG_Y);
        let hit = tetrahedron.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(
            hit,
            Some(RayHit3d::new(1.0, Dir3::from_xyz(1.0, 1.0, 1.0).unwrap()))
        );

        // Ray origin is inside of the solid tetrahedron.
        let ray = Ray3d::new(Vec3::new(-0.5, 0.25, -0.5), Vec3::NEG_X);
        let hit = tetrahedron.local_ray_cast(ray, f32::MAX, true);
        assert_eq!(hit, Some(RayHit3d::new(0.0, Dir3::X)));

        // Ray origin is inside of the hollow tetrahedron.
        let ray = Ray3d::new(Vec3::new(-0.5, 0.25, -0.5), Vec3::NEG_X);
        let hit = tetrahedron.local_ray_cast(ray, f32::MAX, false);
        assert_eq!(hit, Some(RayHit3d::new(0.5, Dir3::X)));

        // Ray points away from the tetrahedron.
        assert!(!tetrahedron.intersects_local_ray(Ray3d::new(
            Vec3::new(0.0, 1.1, 0.0),
            Vec3::new(1.0, -1.0, 1.0)
        )));

        // Hit distance exceeds max distance.
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::NEG_Y);
        let hit = tetrahedron.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
