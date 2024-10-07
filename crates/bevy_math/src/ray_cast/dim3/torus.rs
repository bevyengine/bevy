use ops::FloatPow;

use crate::prelude::*;

impl PrimitiveRayCast3d for Torus {
    fn local_ray_distance(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<f32> {
        let minor_radius_squared = self.minor_radius * self.minor_radius;

        let is_inside = (self.major_radius - ray.origin.xz().length()).squared()
            + ray.origin.y.squared()
            < minor_radius_squared;

        if solid && is_inside {
            return Some(0.0);
        }

        let major_radius_squared = self.major_radius * self.major_radius;

        torus_ray_distance(*self, minor_radius_squared, major_radius_squared, ray)
            .filter(|d| *d <= max_distance)
    }

    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<RayHit3d> {
        let minor_radius_squared = self.minor_radius * self.minor_radius;
        let major_radius_squared = self.major_radius * self.major_radius;

        let is_inside = (self.major_radius - ray.origin.xz().length()).squared()
            + ray.origin.y.squared()
            < minor_radius_squared;

        if solid && is_inside {
            return Some(RayHit3d::new(0.0, -ray.direction));
        }

        let distance = torus_ray_distance(*self, minor_radius_squared, major_radius_squared, ray)
            .filter(|d| *d <= max_distance)?;

        let point = ray.get_point(distance);

        // df(x)/dx
        let mut normal = Dir3::new(
            point
                * (point.length_squared()
                    - minor_radius_squared
                    - major_radius_squared * Vec3::new(1.0, -1.0, 1.0)),
        )
        .ok()?;

        if is_inside {
            normal = -normal;
        }

        Some(RayHit3d::new(distance, normal))
    }
}

#[inline]
fn torus_ray_distance(
    torus: Torus,
    minor_radius_squared: f32,
    major_radius_squared: f32,
    ray: Ray3d,
) -> Option<f32> {
    // Degree 4 equation
    // f(x) = (|x|^2 + R^2 - r^2)^2 - 4R^2 * (x^2 + y^2) = 0
    //
    // Adapted from Inigo Quilez's algorithm:
    //
    // - https://iquilezles.org/articles/intersectors/
    // - https://www.shadertoy.com/view/4sBGDy

    let mut po = 1.0;

    let origin_distance_squared = ray.origin.length_squared();
    let origin_dot_dir = ray.origin.dot(*ray.direction);

    // Bounding sphere
    let h = origin_dot_dir.squared() - origin_distance_squared
        + (torus.major_radius + torus.minor_radius).squared();
    if h < 0.0 {
        return None;
    }

    // Quartic equation
    let k = (origin_distance_squared - major_radius_squared - minor_radius_squared) * 0.5;
    let mut k3 = origin_dot_dir;
    let mut k2 = origin_dot_dir.squared() + major_radius_squared * ray.direction.y.squared() + k;
    let mut k1 = k * origin_dot_dir + major_radius_squared * ray.origin.y * ray.direction.y;
    let mut k0 = k * k + major_radius_squared * ray.origin.y.squared()
        - major_radius_squared * minor_radius_squared;

    // Prevent c1 from being too close to zero.
    if (k3 * (k3 * k3 - k2) + k1).abs() < 0.01 {
        po = -1.0;
        core::mem::swap(&mut k1, &mut k3);
        k0 = 1.0 / k0;
        k1 *= k0;
        k2 *= k0;
        k3 *= k0;
    }

    let mut c2 = 2.0 * k2 - 3.0 * k3 * k3;
    let mut c1 = k3 * (k3 * k3 - k2) + k1;
    let mut c0 = k3 * (k3 * (-3.0 * k3 * k3 + 4.0 * k2) - 8.0 * k1) + 4.0 * k0;

    c2 /= 3.0;
    c1 *= 2.0;
    c0 /= 3.0;

    let q = c2 * c2 + c0;
    let r = 3.0 * c0 * c2 - c2 * c2 * c2 - c1 * c1;

    let h = r * r - q * q * q;
    let mut z: f32;

    if h < 0.0 {
        // 4 intersections
        let q_sqrt = q.sqrt();
        z = 2.0 * q_sqrt * ops::cos(ops::acos(r / (q * q_sqrt)) / 3.0);
    } else {
        // 2 intersections
        let q_sqrt = ops::cbrt(h.sqrt() + r.abs());
        z = r.signum() * (q_sqrt + q / q_sqrt).abs();
    }

    z = c2 - z;

    let mut d1 = z - 3.0 * c2;
    let mut d2 = z * z - 3.0 * c0;

    if d1.abs() < 1.0e-4 {
        if d2 < 0.0 {
            return None;
        }
        d2 = d2.sqrt();
    } else {
        if d1 < 0.0 {
            return None;
        }
        d1 = (d1 / 2.0).sqrt();
        d2 = c1 / d1;
    }

    let mut distance = f32::MAX;

    let discriminant1 = d1 * d1 - z + d2;
    if discriminant1 > 0.0 {
        let d_sqrt = discriminant1.sqrt();
        let (t1, t2) = if po < 0.0 {
            (2.0 / (-d1 - d_sqrt - k3), 2.0 / (-d1 + d_sqrt - k3))
        } else {
            (-d1 - d_sqrt - k3, -d1 + d_sqrt - k3)
        };

        if t1 > 0.0 {
            distance = t1;
        }

        if t2 > 0.0 && t2 < distance {
            distance = t2;
        }
    }

    let discriminant2 = d1 * d1 - z - d2;
    if discriminant2 > 0.0 {
        let d_sqrt = discriminant2.sqrt();
        let (t1, t2) = if po < 0.0 {
            (2.0 / (d1 - d_sqrt - k3), 2.0 / (d1 + d_sqrt - k3))
        } else {
            (d1 - d_sqrt - k3, d1 + d_sqrt - k3)
        };

        if t1 > 0.0 && t1 < distance {
            distance = t1;
        }

        if t2 > 0.0 && t2 < distance {
            distance = t2;
        }
    }

    Some(distance)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn local_ray_cast_torus() {
        let torus = Torus::new(0.5, 1.0);

        // Ray origin is outside of the shape.
        let ray = Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::NEG_X);
        let hit = torus.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_relative_eq!(hit.distance, 1.0);
        assert_eq!(hit.normal, Dir3::X);

        // Ray origin is inside of the hole (smaller circle).
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let hit = torus.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_relative_eq!(hit.distance, 0.5, epsilon = 1.0e-6);
        assert_eq!(hit.normal, Dir3::NEG_X);

        // Ray origin is inside of the solid torus.
        let ray = Ray3d::new(Vec3::new(0.75, 0.0, 0.0), Vec3::X);
        let hit = torus.local_ray_cast(ray, f32::MAX, true).unwrap();
        assert_relative_eq!(hit.distance, 0.0);
        assert_eq!(hit.normal, Dir3::NEG_X);

        // Ray origin is inside of the hollow torus.
        let ray = Ray3d::new(Vec3::new(0.75, 0.0, 0.0), Vec3::X);
        let hit = torus.local_ray_cast(ray, f32::MAX, false).unwrap();
        assert_relative_eq!(hit.distance, 0.25);
        assert_eq!(hit.normal, Dir3::NEG_X);

        // Ray points away from the torus.
        let ray = Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::Y);
        assert!(!torus.intersects_local_ray(ray));

        // Hit distance exceeds max distance.
        let ray = Ray3d::new(Vec3::new(2.0, 0.0, 0.0), Vec3::NEG_Y);
        let hit = torus.local_ray_cast(ray, 0.5, true);
        assert!(hit.is_none());
    }
}
