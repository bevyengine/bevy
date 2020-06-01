use bevy_core::bytes::GetBytes;
use bevy_derive::Uniform;
use glam::Vec2;
use zerocopy::AsBytes;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Uniform, AsBytes)]
pub struct Quad {
    pub position: Vec2,
    pub size: Vec2,
    pub z_index: f32,
}

impl GetBytes for Quad {
    fn get_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_bytes())
    }
}
