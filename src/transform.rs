use crate::math;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub local: math::Mat4,
    pub global: math::Mat4,
}

impl Transform {
    pub fn new() -> Transform {
        Transform {
            local: math::identity(),
            global: math::identity(),
        }
    }
}