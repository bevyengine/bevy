use nalgebra::Matrix4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub local: Matrix4<f32>,
    pub global: Matrix4<f32>,
}

impl Transform {
    pub fn new() -> Transform {
        Transform {
            local: Matrix4::<f32>::identity(),
            global: Matrix4::<f32>::identity(),
        }
    }
}