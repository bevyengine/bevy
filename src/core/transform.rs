use crate::math;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub value: math::Mat4,
}

impl Transform {
    pub fn new() -> Transform {
        Transform {
            value: math::identity(),
        }
    }
}