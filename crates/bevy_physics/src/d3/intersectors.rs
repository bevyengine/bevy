use super::{geometry::*, ray::*};
use glam::Vec3;

pub struct RayHit {
    t: f32,
    point: Vec3,
}

impl RayHit {
    pub fn new(t: f32, point: Vec3) -> Self {
        Self { t, point }
    }

    pub fn t(&self) -> &f32 {
        &self.t
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
    fn intersect_ray(&self, _ray: &Ray) -> Option<RayHit> {
        unimplemented!()
    }
}
