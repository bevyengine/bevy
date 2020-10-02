use glam::Vec3;

pub struct Plane {
    center: Vec3,
    normal: Vec3,
}

impl Plane {
    pub fn new(center: Vec3, normal: Vec3) -> Self {
        normal.normalize();
        Self { center, normal }
    }

    pub fn center(&self) -> &Vec3 {
        &self.center
    }

    pub fn center_mut(&mut self) -> &mut Vec3 {
        &mut self.center
    }

    pub fn normal(&self) -> &Vec3 {
        &self.normal
    }

    pub fn set_normal(&mut self, normal: Vec3) {
        normal.normalize();
        self.normal = normal;
    }
}

impl Default for Plane {
    fn default() -> Self {
        Plane {
            center: Vec3::zero(),
            normal: Vec3::unit_y(),
        }
    }
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Default for Sphere {
    fn default() -> Self {
        Sphere {
            center: Vec3::zero(),
            radius: 1.0,
        }
    }
}

pub struct Triangle {
    pub vertices: [Vec3; 3],
}
