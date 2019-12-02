use crate::math::Vector3;
use shrinkwraprs::Shrinkwrap;
use std::fmt;

#[derive(Shrinkwrap, Debug, PartialEq, Clone, Copy)]
#[shrinkwrap(mutable)]
pub struct NonUniformScale(pub Vector3<f32>);

impl NonUniformScale {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vector3::new(x, y, z))
    }
}

impl From<Vector3<f32>> for NonUniformScale {
    fn from(scale: Vector3<f32>) -> Self {
        Self(scale)
    }
}

impl From<&Vector3<f32>> for NonUniformScale {
    fn from(scale: &Vector3<f32>) -> Self {
        Self(*scale)
    }
}

impl From<&mut Vector3<f32>> for NonUniformScale {
    fn from(scale: &mut Vector3<f32>) -> Self {
        Self(*scale)
    }
}

impl fmt::Display for NonUniformScale {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "NonUniformScale({}, {}, {})",
            self.0.x, self.0.y, self.0.z
        )
    }
}
