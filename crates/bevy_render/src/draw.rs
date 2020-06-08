use crate::{render_resource::RenderResourceId, pipeline::PipelineDescriptor};
use bevy_asset::Handle;
use std::ops::Range;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DrawType {
    Instanced {
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VertexBufferBinding {
    pub slot: u32,
    pub vertex_buffer: RenderResourceId,
    pub offset: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IndexBufferBinding {
    pub vertex_buffer: RenderResourceId,
    pub offset: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BindGroupBinding {
    pub vertex_buffer: RenderResourceId,
    pub offset: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DrawCall {
    pub pipeline: Handle<PipelineDescriptor>,
    pub draw_type: DrawType,
    pub vertex_buffers: Vec<VertexBufferBinding>,
    pub index_buffer: Option<IndexBufferBinding>,
}

pub struct Draw {}
