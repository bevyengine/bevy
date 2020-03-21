use crate::render::render_resource::BufferUsage;
use super::RenderResourceAssignmentsId;
use std::collections::HashMap;

#[derive(Default)]
pub struct BufferArrayInfo {
    pub item_count: u64,
    pub item_size: u64,
    pub item_capacity: u64,
}

#[derive(Default)]
pub struct BufferDynamicUniformInfo {
    pub offsets: HashMap<RenderResourceAssignmentsId, u32>,
}

pub struct BufferInfo {
    pub size: u64,
    pub buffer_usage: BufferUsage,
    pub array_info: Option<BufferArrayInfo>,
    pub dynamic_uniform_info: Option<BufferDynamicUniformInfo>,
}

impl Default for BufferInfo {
    fn default() -> Self {
        BufferInfo {
            size: 0,
            buffer_usage: BufferUsage::NONE,
            array_info: None,
            dynamic_uniform_info: None,
        }
    }
}

pub enum ResourceInfo {
    Buffer(BufferInfo),
    Texture,
    Sampler,
}
