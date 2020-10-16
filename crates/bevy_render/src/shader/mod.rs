#[allow(clippy::module_inception)]
mod shader;
mod shader_defs;

#[cfg(not(target_arch = "wasm32"))]
mod shader_reflect;

#[cfg(target_arch = "wasm32")]
#[path = "shader_reflect_wasm.rs"]
mod shader_reflect;

pub use shader::*;
pub use shader_defs::*;
pub use shader_reflect::*;

use crate::pipeline::{BindGroupDescriptor, VertexBufferDescriptor};

/// Defines the memory layout of a shader
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderLayout {
    pub bind_groups: Vec<BindGroupDescriptor>,
    pub vertex_buffer_descriptors: Vec<VertexBufferDescriptor>,
    pub entry_point: String,
}

pub const GL_VERTEX_INDEX: &str = "gl_VertexIndex";
