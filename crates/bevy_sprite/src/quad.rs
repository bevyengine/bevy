use bevy_derive::{Uniform, Bytes};
use glam::Vec2;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Uniform, Bytes)]
pub struct Quad {
    pub position: Vec2,
    pub size: Vec2,
    pub z_index: f32,
}