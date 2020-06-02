use bevy_derive::Uniforms;
use bevy_core::bytes::Byteable;

#[repr(C)]
#[derive(Clone, Copy, Uniforms)]
#[module(bevy_render = "crate")]
pub struct Vertex {
    #[uniform(vertex)]
    pub position: [f32; 3],
    #[uniform(vertex)]
    pub normal: [f32; 3],
    #[uniform(vertex)]
    pub uv: [f32; 2],
}


// SAFE: Vertex is repr(C) containing primitives
unsafe impl Byteable for Vertex {} 