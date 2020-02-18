use crate::{math, math::Vec4, prelude::Translation, render::camera};
use std::ops::Range;
use zerocopy::{AsBytes, FromBytes};

pub struct Light {
    pub color: Vec4,
    pub fov: f32,
    pub depth: Range<f32>,
}

impl Default for Light {
    fn default() -> Self {
        Light {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            depth: 0.1..50.0,
            fov: f32::to_radians(60.0),
        }
    }
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
        let proj = camera::get_perspective_projection_matrix(
            light.fov,
            1.0,
            light.depth.start,
            light.depth.end,
        ) * *transform;
        let (x, y, z) = translation.0.into();
        LightRaw {
            proj: proj.to_cols_array_2d(),
            pos: [x, y, z, 1.0],
            color: light.color.into(),
        }
    }
}
