use zerocopy::AsBytes;
use bevy_derive::Uniforms;

#[repr(C)]
#[derive(Clone, Copy, AsBytes, Uniforms)]
#[module(bevy_render = "crate")]
pub struct Vertex {
    #[uniform(vertex)]
    pub position: [f32; 3],
    #[uniform(vertex)]
    pub normal: [f32; 3],
    #[uniform(vertex)]
    pub uv: [f32; 2],
}
