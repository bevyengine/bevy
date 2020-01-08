pub mod camera;
pub mod shader;
pub mod mesh;
pub mod render_resources;
pub mod passes;

mod light;
mod render_graph;
mod material;

pub use light::*;
pub use shader::*;
pub use render_graph::*;
pub use material::*;
pub use mesh::*;
pub use camera::*;

use std::mem;
use crate::vertex::Vertex;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub size: u64,
}

impl UniformBuffer {
    pub fn get_binding_resource<'a>(&'a self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::Buffer {
            buffer: &self.buffer,
            range: 0 .. self.size,
        }
    }
}

pub fn get_vertex_buffer_descriptor<'a>() -> wgpu::VertexBufferDescriptor<'a> {
    let vertex_size = mem::size_of::<Vertex>();
    wgpu::VertexBufferDescriptor {
        stride: vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &[
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
        ],
    }
}