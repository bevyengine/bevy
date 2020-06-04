use crate::{render_resource::BufferUsage, texture::TextureDescriptor};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BufferInfo {
    pub size: usize,
    pub buffer_usage: BufferUsage,
}

impl Default for BufferInfo {
    fn default() -> Self {
        BufferInfo {
            size: 0,
            buffer_usage: BufferUsage::empty(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ResourceInfo {
    Buffer(BufferInfo),
    Texture(TextureDescriptor),
    Sampler,
}
