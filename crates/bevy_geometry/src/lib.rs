use bevy_math::{Quat, Vec3};

pub trait Primitive3d {}

pub struct Sphere {
    pub origin: Vec3,
    pub radius: f32,
}

pub struct Box {
    pub maximums: Vec3,
    pub minimums: Vec3,
    pub orientation: Quat,
}

pub struct AxisAlignedBox {
    pub maximums: Vec3,
    pub minimums: Vec3,
}

pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
}
