use glam::Vec2;
use bevy_render::{shader::Uniform, render_resource::{RenderResources, RenderResource}};
use bevy_core::bytes::Bytes;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Uniform, RenderResources, RenderResource, Bytes)]
#[render_resources(from_self)]
pub struct Quad {
    pub position: Vec2,
    pub size: Vec2,
    pub z_index: f32,
}
