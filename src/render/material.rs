use zerocopy::{AsBytes, FromBytes};
use crate::math;

pub struct Material {
    pub color: math::Vec4,
    pub bind_group: Option<wgpu::BindGroup>,
    pub uniform_buf: Option<wgpu::Buffer>,
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