use crate::pipeline::AsVertexBufferDescriptor;
use bevy_core::bytes::Byteable;

#[repr(C)]
#[derive(Clone, Copy, AsVertexBufferDescriptor)]
#[module(bevy_render = "crate")]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

// SAFE: Vertex is repr(C) containing primitives
unsafe impl Byteable for Vertex {}
