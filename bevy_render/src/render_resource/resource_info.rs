use crate::render_resource::BufferUsage;

#[derive(Default, Debug, Clone)]
pub struct BufferArrayInfo {
    pub item_size: usize,
    pub item_capacity: usize,
}

#[derive(Debug, Clone)]
pub struct BufferInfo {
    pub size: usize,
    pub buffer_usage: BufferUsage,
    pub array_info: Option<BufferArrayInfo>,
    pub is_dynamic: bool,
}

impl Default for BufferInfo {
    fn default() -> Self {
        BufferInfo {
            size: 0,
            buffer_usage: BufferUsage::NONE,
            array_info: None,
            is_dynamic: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResourceInfo {
    Buffer(BufferInfo),
    Texture,
    Sampler,
}
