use zerocopy::{AsBytes, FromBytes};
use crate::math;

pub struct Material {
    pub color: math::Vec4,
    pub bind_group: Option<wgpu::BindGroup>,
    pub uniform_buf: Option<wgpu::Buffer>,
}

pub struct Instanced;

impl Material {
    pub fn new(color: math::Vec4) -> Self {
        Material {
            color,
            bind_group: None,
            uniform_buf: None, 
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct RenderedUniforms {
    pub transform: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct MaterialUniforms {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct SimpleMaterialUniforms {
    pub position: [f32; 3],
    pub color: [f32; 4],
}