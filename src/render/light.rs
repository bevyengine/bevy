use crate::{math, render::camera};
use std::ops::Range;
use zerocopy::{AsBytes, FromBytes};


pub struct Light {
    pub pos: math::Vec3,
    pub color: wgpu::Color,
    pub fov: f32,
    pub depth: Range<f32>,
    pub target_view: wgpu::TextureView,
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct LightRaw {
    pub proj: [[f32; 4]; 4],
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

impl Light {
    pub fn to_raw(&self) -> LightRaw {
        LightRaw {
            proj: camera::get_projection_view_matrix(&self.pos, self.fov, 1.0, self.depth.start, self.depth.end).into(),
            pos: [self.pos.x, self.pos.y, self.pos.z, 1.0],
            color: [
                self.color.r as f32,
                self.color.g as f32,
                self.color.b as f32,
                1.0,
            ],
        }
    }
}