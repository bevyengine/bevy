use bevy_render::{Color, PerspectiveCamera, CameraProjection};
use bevy_transform::components::Translation;
use bevy_property::Properties;
use glam::Mat4;
use std::ops::Range;
use zerocopy::{AsBytes, FromBytes};

#[derive(Properties)]
pub struct Light {
    pub color: Color,
    pub fov: f32,
    pub depth: Range<f32>,
}

impl Default for Light {
    fn default() -> Self {
        Light {
            color: Color::rgb(1.0, 1.0, 1.0),
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
    pub fn from(light: &Light, transform: &Mat4, translation: &Translation) -> LightRaw {
        let perspective = PerspectiveCamera {
            fov: light.fov,
            aspect_ratio: 1.0,
            near: light.depth.start,
            far: light.depth.end,
        };

        let proj = perspective.get_view_matrix() * *transform;
        let (x, y, z) = translation.0.into();
        LightRaw {
            proj: proj.to_cols_array_2d(),
            pos: [x, y, z, 1.0],
            color: light.color.into(),
        }
    }
}
