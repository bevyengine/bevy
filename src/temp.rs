use std::{sync::Arc};
use zerocopy::{AsBytes, FromBytes};

pub struct CubeEnt {
    pub rotation_speed: f32,
    pub color: wgpu::Color,
    pub vertex_buf: Arc<wgpu::Buffer>,
    pub index_buf: Arc<wgpu::Buffer>,
    pub index_count: usize,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buf: wgpu::Buffer,
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct EntityUniforms {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}