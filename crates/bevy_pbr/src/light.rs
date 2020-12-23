use bevy_core::Byteable;
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_render::{
    camera::{CameraProjection, PerspectiveProjection},
    color::Color,
};
use bevy_transform::components::GlobalTransform;
use std::ops::Range;

/// A point light
#[derive(Debug, Reflect)]
#[reflect(Component)]
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
#[derive(Debug, Clone, Copy)]
pub(crate) struct LightRaw {
    pub proj: [[f32; 4]; 4],
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

unsafe impl Byteable for LightRaw {}

impl LightRaw {
    pub fn from(light: &Light, global_transform: &GlobalTransform) -> LightRaw {
        let perspective = PerspectiveProjection {
            fov: light.fov,
            aspect_ratio: 1.0,
            near: light.depth.start,
            far: light.depth.end,
        };

        let proj = perspective.get_projection_matrix() * global_transform.compute_matrix();
        let (x, y, z) = global_transform.translation.into();
        LightRaw {
            proj: proj.to_cols_array_2d(),
            pos: [x, y, z, 1.0],
            color: light.color.into(),
        }
    }
}

// Ambient light color.
#[derive(Debug)]
pub struct AmbientLight {
    pub color: Color,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Color::rgb(0.05, 0.05, 0.05),
        }
    }
}
