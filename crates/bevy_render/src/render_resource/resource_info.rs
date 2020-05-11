use crate::render_resource::BufferUsage;

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct BufferArrayInfo {
    pub item_size: usize,
    pub item_capacity: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BufferInfo {
    pub size: usize,
    pub buffer_usage: BufferUsage,
    // TODO: remove array info and is_dynamic
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ResourceInfo {
    Buffer(BufferInfo),
    Texture,
    Sampler,
}
