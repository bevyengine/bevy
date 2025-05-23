mod buffer;
mod texture;

pub use buffer::*;
pub use texture::*;

use super::{FrameGraph, GraphResourceNodeHandle};
use crate::renderer::RenderDevice;
use alloc::sync::Arc;

pub trait ResourceMaterial {
    type ResourceType: TransientResource;

    fn imported(&self, frame_graph: &mut FrameGraph)
        -> GraphResourceNodeHandle<Self::ResourceType>;
}

pub trait IntoArcTransientResource
where
    Self: Sized + TransientResource,
{
    fn into_arc_transient_resource(self: Arc<Self>) -> ArcTransientResource;
}

pub trait TransientResource: 'static {
    type Descriptor: TransientResourceDescriptor;

    fn borrow_resource(res: &AnyTransientResource) -> &Self;

    fn get_desc(&self) -> &Self::Descriptor;
}

pub trait TransientResourceDescriptor: 'static + Clone + Into<AnyFrameGraphResourceDescriptor> {
    type Resource: TransientResource;
}

pub trait TypeEquals {
    type Other;
    fn same(value: Self) -> Self::Other;
}

impl<T: Sized> TypeEquals for T {
    type Other = Self;
    fn same(value: Self) -> Self::Other {
        value
    }
}

pub trait FrameGraphResourceCreator {
    fn create_texture(&self, desc: &TextureInfo) -> TransientTexture;

    fn create_buffer(&self, desc: &BufferInfo) -> TransientBuffer;

    fn create_resource(&self, desc: &AnyFrameGraphResourceDescriptor) -> AnyTransientResource {
        match desc {
            AnyFrameGraphResourceDescriptor::Texture(info) => {
                let texture = self.create_texture(info);
                AnyTransientResource::OwnedTexture(texture)
            }
            AnyFrameGraphResourceDescriptor::Buffer(info) => {
                let buffer = self.create_buffer(info);
                AnyTransientResource::OwnedBuffer(buffer)
            }
        }
    }
}

impl FrameGraphResourceCreator for RenderDevice {
    fn create_texture(&self, desc: &TextureInfo) -> TransientTexture {
        let resource = self.wgpu_device().create_texture(&desc.get_texture_desc());
        TransientTexture {
            resource,
            desc: desc.clone(),
        }
    }

    fn create_buffer(&self, desc: &BufferInfo) -> TransientBuffer {
        let resource = self.wgpu_device().create_buffer(&desc.get_buffer_desc());

        TransientBuffer {
            resource,
            desc: desc.clone(),
        }
    }
}

#[derive(Clone)]
pub enum ArcTransientResource {
    Buffer(Arc<TransientBuffer>),
    Texture(Arc<TransientTexture>),
}

pub enum AnyTransientResource {
    OwnedBuffer(TransientBuffer),
    ImportedBuffer(Arc<TransientBuffer>),
    OwnedTexture(TransientTexture),
    ImportedTexture(Arc<TransientTexture>),
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum AnyFrameGraphResourceDescriptor {
    Texture(TextureInfo),
    Buffer(BufferInfo),
}
