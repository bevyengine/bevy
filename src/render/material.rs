use crate::{
    asset::{Handle, Texture},
    math,
};
use zerocopy::{AsBytes, FromBytes};

pub enum Albedo {
    Color(math::Vec4),
    Texture(Handle<Texture>),
}

pub struct Material {
    pub albedo: Albedo,
    pub bind_group: Option<wgpu::BindGroup>,
    pub uniform_buf: Option<wgpu::Buffer>,
}

pub struct Instanced;

impl Material {
    pub fn new(albedo: Albedo) -> Self {
        Material {
            albedo,
            bind_group: None,
            uniform_buf: None,
        }
    }

    pub fn get_color(&self) -> math::Vec4 {
        match self.albedo {
            Albedo::Color(color) => color,
            _ => math::vec4(1.0, 0.0, 1.0, 1.0),
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
