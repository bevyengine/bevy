use glam::Vec2;
use bevy_render::Color;
use bevy_derive::Uniforms;
use zerocopy::AsBytes;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Uniforms, AsBytes)]
#[module(meta = "false")]
pub struct Rect {
    pub position: Vec2,
    pub size: Vec2,
    pub color: Color,
    pub z_index: f32,
}