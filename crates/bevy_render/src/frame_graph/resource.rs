use alloc::{borrow::Cow, sync::Arc};

use crate::renderer::RenderDevice;

pub trait FrameGraphResourceCreator {
    fn create_texture(&self, desc: &TextureInfo) -> FrameGraphTexture;

    fn create_buffer(&self, desc: &BufferInfo) -> FrameGraphBuffer;

    fn create_resource(&self, desc: &AnyFrameGraphResourceDescriptor) -> AnyFrameGraphResource {
        match desc {
            AnyFrameGraphResourceDescriptor::Texture(info) => {
                let texture = self.create_texture(info);
                AnyFrameGraphResource::OwnedTexture(texture)
            }
            AnyFrameGraphResourceDescriptor::Buffer(info) => {
                let buffer = self.create_buffer(info);
                AnyFrameGraphResource::OwnedBuffer(buffer)
            }
        }
    }
}

impl FrameGraphResourceCreator for RenderDevice {
    fn create_texture(&self, desc: &TextureInfo) -> FrameGraphTexture {
        let resource = self.wgpu_device().create_texture(&desc.get_texture_desc());
        FrameGraphTexture {
            resource,
            desc: desc.clone(),
        }
    }

    fn create_buffer(&self, desc: &BufferInfo) -> FrameGraphBuffer {
        let resource = self.wgpu_device().create_buffer(&desc.get_buffer_desc());

        FrameGraphBuffer {
            resource,
            desc: desc.clone(),
        }
    }
}

#[derive(Clone)]
pub enum ImportedResource {
    Buffer(Arc<FrameGraphBuffer>),
    Texture(Arc<FrameGraphTexture>),
}

pub enum AnyFrameGraphResource {
    OwnedBuffer(FrameGraphBuffer),
    ImportedBuffer(Arc<FrameGraphBuffer>),
    OwnedTexture(FrameGraphTexture),
    ImportedTexture(Arc<FrameGraphTexture>),
}

pub struct FrameGraphBuffer {
    pub resource: wgpu::Buffer,
    pub desc: BufferInfo,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct BufferInfo {
    pub label: Option<Cow<'static, str>>,
    pub size: wgpu::BufferAddress,
    pub usage: wgpu::BufferUsages,
    pub mapped_at_creation: bool,
}

impl BufferInfo {
    pub fn get_buffer_desc(&self) -> wgpu::BufferDescriptor {
        wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: self.size,
            usage: self.usage,
            mapped_at_creation: self.mapped_at_creation,
        }
    }
}

pub struct FrameGraphTexture {
    pub resource: wgpu::Texture,
    pub desc: TextureInfo,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct TextureInfo {
    pub label: Option<Cow<'static, str>>,
    pub size: wgpu::Extent3d,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: wgpu::TextureDimension,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub view_formats: Vec<wgpu::TextureFormat>,
}

impl TextureInfo {
    pub fn get_texture_desc(&self) -> wgpu::TextureDescriptor {
        wgpu::TextureDescriptor {
            label: self.label.as_deref(),
            size: self.size,
            mip_level_count: self.mip_level_count,
            sample_count: self.sample_count,
            dimension: self.dimension,
            format: self.format,
            usage: self.usage,
            view_formats: &self.view_formats,
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum AnyFrameGraphResourceDescriptor {
    Texture(TextureInfo),
    Buffer(BufferInfo),
}
