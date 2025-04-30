use std::ops::Deref;

use alloc::{borrow::Cow, sync::Arc};

use crate::{
    render_resource::{Buffer, SurfaceTexture, Texture},
    renderer::RenderDevice,
};

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

impl FrameGraphBuffer {
    pub fn new_arc_with_buffer(buffer: &Buffer) -> Arc<FrameGraphBuffer> {
        Arc::new(FrameGraphBuffer {
            desc: BufferInfo {
                label: None,
                size: buffer.size(),
                usage: buffer.usage(),
                mapped_at_creation: false,
            },
            resource: buffer.deref().clone(),
        })
    }
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

impl FrameGraphTexture {
    pub fn new_arc_with_texture(texture: &Texture) -> Arc<FrameGraphTexture> {
        Arc::new(FrameGraphTexture {
            desc: TextureInfo {
                label: None,
                size: wgpu::Extent3d {
                    width: texture.width(),
                    height: texture.height(),
                    depth_or_array_layers: texture.depth_or_array_layers(),
                },
                mip_level_count: texture.mip_level_count(),
                sample_count: texture.sample_count(),
                dimension: texture.dimension(),
                format: texture.format(),
                usage: texture.usage(),
                view_formats: vec![],
            },
            resource: texture.deref().clone(),
        })
    }

    pub fn new_arc_with_surface(surface: &SurfaceTexture) -> Arc<FrameGraphTexture> {
        Arc::new(FrameGraphTexture {
            desc: TextureInfo {
                label: None,
                size: wgpu::Extent3d {
                    width: surface.texture.width(),
                    height: surface.texture.height(),
                    depth_or_array_layers: surface.texture.depth_or_array_layers(),
                },
                mip_level_count: surface.texture.mip_level_count(),
                sample_count: surface.texture.sample_count(),
                dimension: surface.texture.dimension(),
                format: surface.texture.format(),
                usage: surface.texture.usage(),
                view_formats: vec![],
            },
            resource: surface.texture.clone(),
        })
    }
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
