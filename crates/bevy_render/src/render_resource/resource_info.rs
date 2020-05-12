use crate::{texture::TextureDescriptor, render_resource::BufferUsage};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BufferInfo {
    pub size: usize,
    pub buffer_usage: BufferUsage,
}

impl Default for BufferInfo {
    fn default() -> Self {
        BufferInfo {
            size: 0,
            buffer_usage: BufferUsage::NONE,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ResourceInfo {
    Buffer(BufferInfo),
    Texture(TextureDescriptor),
    Sampler,
}
