use super::{geometry::*, ray::*};
use glam::Vec3;

pub struct RayHit {
    distance: f32,
    point: Vec3,
}

impl RayHit {
    pub fn new(distance: f32, point: Vec3) -> Self {
        Self { distance, point }
    }

    pub fn distance(&self) -> &f32 {
        &self.distance
    }

    pub fn point(&self) -> &Vec3 {
        &self.point
    }
}

pub trait RayIntersector {
    fn intersect_ray(&self, ray: &Ray) -> Option<RayHit>;
}

impl RayIntersector for Plane {
    fn intersect_ray(&self, ray: &Ray) -> Option<RayHit> {
        let d = self.normal().dot(*ray.direction());
        if d.abs() > f32::EPSILON {
            let t = (*self.center() - *ray.origin()).dot(*self.normal()) / d;
            if t > 0.0 {
                return Some(RayHit::new(t, *ray.origin() + *ray.direction() * t));
            }
        }

        None
    }
}

impl RayIntersector for Sphere {
    fn intersect_ray(&self, ray: &Ray) -> Option<RayHit> {
        let oc = *ray.origin() - self.center;
        let a = ray.direction().length_squared();
        let b = 2.0 * oc.dot(*ray.direction());
        let c = oc.length_squared() - self.radius.powi(2);

        let d = b.powi(2) - 4.0 * a * c;

        if d < 0.0 {
            None
        } else {
            let t = (-b - d.sqrt()) / (2.0 * a);

            Some(RayHit::new(t, *ray.origin() + *ray.direction() * t))
        }
    }
}

impl RayIntersector for Triangle {
    // using the Moeller-Trumbore intersection algorithm
    fn intersect_ray(&self, ray: &Ray) -> Option<RayHit> {
        let edges = (self.1 - self.0, self.2 - self.0);
        let h = ray.direction().cross(edges.1);
        let a = edges.0.dot(h);

        if a > -f32::EPSILON && a < f32::EPSILON {
            return None;
        }

        let f = 1.0 / a;
        let s = *ray.origin() - self.0;
        let u = f * s.dot(h);

        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = s.cross(edges.0);
        let v = f * ray.direction().dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edges.1.dot(q);

        if t > f32::EPSILON {
            Some(RayHit::new(t, *ray.origin() + *ray.direction() * t))
        } else {
            None
        }
    }
}
