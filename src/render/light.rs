use crate::{math, render::camera, Translation};
use std::ops::Range;
use zerocopy::{AsBytes, FromBytes};


pub struct Light {
    pub color: wgpu::Color,
    pub fov: f32,
    pub depth: Range<f32>,
    pub target_view: Option<wgpu::TextureView>,
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct LightRaw {
    pub proj: [[f32; 4]; 4],
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

impl LightRaw {
    pub fn from(light: &Light, transform: &math::Mat4, translation: &Translation) -> LightRaw {
        let proj = camera::get_projection_matrix(light.fov, 1.0, light.depth.start, light.depth.end) * *transform;
        let (x, y, z) = translation.0.into();
        LightRaw {
            proj: proj.to_cols_array_2d(),
            pos: [x, y, z, 1.0],
            color: [
                light.color.r as f32,
                light.color.g as f32,
                light.color.b as f32,
                1.0,
            ],
        }
    }
}