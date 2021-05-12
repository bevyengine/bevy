#[allow(clippy::module_inception)]
mod shader;
mod shader_defs;

#[cfg(not(target_arch = "wasm32"))]
mod shader_reflect;

pub use shader::*;
pub use shader_defs::*;

#[cfg(not(target_arch = "wasm32"))]
pub use shader_reflect::*;

use crate::pipeline::{BindGroupDescriptor, VertexBufferLayout};

/// Defines the memory layout of a shader
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderLayout {
    pub bind_groups: Vec<BindGroupDescriptor>,
    pub vertex_buffer_layout: Vec<VertexBufferLayout>,
    pub entry_point: String,
}

pub const GL_VERTEX_INDEX: &str = "gl_VertexIndex";
pub const GL_INSTANCE_INDEX: &str = "gl_InstanceIndex";
pub const GL_FRONT_FACING: &str = "gl_FrontFacing";
