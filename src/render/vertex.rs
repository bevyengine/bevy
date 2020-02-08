use crate::render::render_graph_2::VertexBufferDescriptor;
use std::convert::From;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub uv: [f32; 2],
}

impl Vertex {
    // TODO: generate from macro
    pub fn get_vertex_buffer_descriptor() -> VertexBufferDescriptor {
        VertexBufferDescriptor {
            stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: vec![
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float4,
                    offset: 4 * 4,
                    shader_location: 1,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float2,
                    offset: 8 * 4,
                    shader_location: 2,
                },
            ],
        }
    }
}

impl From<([f32; 4], [f32; 4], [f32; 2])> for Vertex {
    fn from((position, normal, uv): ([f32; 4], [f32; 4], [f32; 2])) -> Self {
        Vertex {
            position: position,
            normal: normal,
            uv: uv,
        }
    }
}

impl From<([f32; 3], [f32; 3], [f32; 2])> for Vertex {
    fn from((position, normal, uv): ([f32; 3], [f32; 3], [f32; 2])) -> Self {
        Vertex {
            position: [position[0], position[1], position[2], 1.0],
            normal: [normal[0], normal[1], normal[2], 0.0],
            uv: uv,
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
