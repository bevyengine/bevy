use std::convert::From;
use zerocopy::{AsBytes, FromBytes};

use bevy_asset;
use bevy_core;
use bevy_derive::Uniforms;

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes, Uniforms)]
#[module(meta = false, bevy_render = "crate")]
pub struct Vertex {
    #[uniform(vertex)]
    pub position: [f32; 3],
    #[uniform(vertex)]
    pub normal: [f32; 3],
    #[uniform(vertex)]
    pub uv: [f32; 2],
}