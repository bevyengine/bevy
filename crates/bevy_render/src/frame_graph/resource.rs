use alloc::{borrow::Cow, sync::Arc};
use wgpu::{BufferAddress, COPY_BUFFER_ALIGNMENT};

use crate::{render_resource::SurfaceTexture, renderer::RenderDevice};

use super::{FrameGraph, GraphResource, GraphResourceNodeHandle};

pub trait ResourceMaterial {
    type ResourceType: GraphResource;

    fn imported(&self, frame_graph: &mut FrameGraph)
        -> GraphResourceNodeHandle<Self::ResourceType>;
}

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

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct BufferInfo {
    pub label: Option<Cow<'static, str>>,
    pub size: BufferAddress,
    pub usage: wgpu::BufferUsages,
    pub mapped_at_creation: bool,
}

impl BufferInfo {
    pub fn from_buffer_init_desc(desc: &wgpu::util::BufferInitDescriptor) -> Self {
        if desc.contents.is_empty() {
            BufferInfo {
                label: desc.label.map(|label| label.to_string().into()),

                size: 0,
                usage: desc.usage,
                mapped_at_creation: false,
            }
        } else {
            let unpadded_size = desc.contents.len() as BufferAddress;
            // Valid vulkan usage is
            // 1. buffer size must be a multiple of COPY_BUFFER_ALIGNMENT.
            // 2. buffer size must be greater than 0.
            // Therefore we round the value up to the nearest multiple, and ensure it's at least COPY_BUFFER_ALIGNMENT.
            let align_mask = COPY_BUFFER_ALIGNMENT - 1;
            let padded_size =
                ((unpadded_size + align_mask) & !align_mask).max(COPY_BUFFER_ALIGNMENT);

            BufferInfo {
                label: desc.label.map(|label| label.to_string().into()),
                size: padded_size,
                usage: desc.usage,
                mapped_at_creation: false,
            }
        }
    }

    pub fn from_buffer_desc(desc: &wgpu::BufferDescriptor) -> Self {
        Self {
            label: desc.label.map(|label| label.to_string().into()),
            size: desc.size,
            usage: desc.usage,
            mapped_at_creation: desc.mapped_at_creation,
        }
    }

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

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
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
    pub fn from_texture_desc(desc: &wgpu::TextureDescriptor) -> Self {
        TextureInfo {
            label: desc.label.map(|label| label.to_string().into()),
            size: desc.size,
            mip_level_count: desc.mip_level_count,
            sample_count: desc.sample_count,
            dimension: desc.dimension,
            format: desc.format,
            usage: desc.usage,
            view_formats: desc.view_formats.to_vec(),
        }
    }

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
