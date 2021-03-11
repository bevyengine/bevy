use bevy_math::{Quat, Vec3};

pub trait Primitive3d {}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Sphere {
    pub origin: Vec3,
    pub radius: f32,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Box {
    pub maximums: Vec3,
    pub minimums: Vec3,
    pub orientation: Quat,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AxisAlignedBox {
    pub maximums: Vec3,
    pub minimums: Vec3,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
}
