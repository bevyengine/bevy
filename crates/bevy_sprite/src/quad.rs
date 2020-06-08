use bevy_core::bytes::Bytes;
use bevy_render::render_resource::{RenderResource, RenderResources};
use glam::Vec2;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, RenderResources, RenderResource, Bytes)]
#[render_resources(from_self)]
pub struct Quad {
    pub position: Vec2,
    pub size: Vec2,
    pub z_index: f32,
}
