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
    pub position: [f32; 4],
    #[uniform(vertex)]
    pub normal: [f32; 4],
    #[uniform(vertex)]
    pub uv: [f32; 2],
}

impl From<([f32; 4], [f32; 4], [f32; 2])> for Vertex {
    fn from((position, normal, uv): ([f32; 4], [f32; 4], [f32; 2])) -> Self {
        Vertex {
            position,
            normal,
            uv,
        }
    }
}

impl From<([f32; 3], [f32; 3], [f32; 2])> for Vertex {
    fn from((position, normal, uv): ([f32; 3], [f32; 3], [f32; 2])) -> Self {
        Vertex {
            position: [position[0], position[1], position[2], 1.0],
            normal: [normal[0], normal[1], normal[2], 0.0],
            uv,
        }
    }
}

impl From<([i8; 4], [i8; 4], [i8; 2])> for Vertex {
    fn from((position, normal, uv): ([i8; 4], [i8; 4], [i8; 2])) -> Self {
        Vertex {
            position: [
                position[0] as f32,
                position[1] as f32,
                position[2] as f32,
                position[3] as f32,
            ],
            normal: [
                normal[0] as f32,
                normal[1] as f32,
                normal[2] as f32,
                normal[3] as f32,
            ],
            uv: [uv[0] as f32, uv[1] as f32],
        }
    }
}

impl From<([i8; 3], [i8; 3], [i8; 2])> for Vertex {
    fn from((position, normal, uv): ([i8; 3], [i8; 3], [i8; 2])) -> Self {
        Vertex {
            position: [
                position[0] as f32,
                position[1] as f32,
                position[2] as f32,
                1.0,
            ],
            normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32, 0.0],
            uv: [uv[0] as f32, uv[1] as f32],
        }
    }
}
