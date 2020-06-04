use crate::math::Vec3;
use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;
use std::fmt;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy, Properties)]
#[shrinkwrap(mutable)]
pub struct NonUniformScale(pub Vec3);

impl NonUniformScale {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
}

impl Default for NonUniformScale {
    fn default() -> Self {
        NonUniformScale(Vec3::new(1.0, 1.0, 1.0))
    }
}

impl From<Vec3> for NonUniformScale {
    fn from(scale: Vec3) -> Self {
        Self(scale)
    }
}

impl From<&Vec3> for NonUniformScale {
    fn from(scale: &Vec3) -> Self {
        Self(*scale)
    }
}

impl From<&mut Vec3> for NonUniformScale {
    fn from(scale: &mut Vec3) -> Self {
        Self(*scale)
    }
}

impl fmt::Display for NonUniformScale {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (x, y, z) = self.0.into();
        write!(f, "NonUniformScale({}, {}, {})", x, y, z)
    }
}
