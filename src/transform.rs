use nalgebra_glm as glm;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub local: glm::Mat4,
    pub global: glm::Mat4,
}

impl Transform {
    pub fn new() -> Transform {
        Transform {
            local: glm::identity(),
            global: glm::identity(),
        }
    }
}