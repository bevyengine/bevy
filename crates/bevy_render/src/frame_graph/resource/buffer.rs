use std::borrow::Cow;
use std::sync::Arc;

use wgpu::{BufferAddress, COPY_BUFFER_ALIGNMENT};

use crate::frame_graph::{TransientResource, TransientResourceDescriptor, IntoArcTransientResource};

use super::{AnyTransientResource, AnyFrameGraphResourceDescriptor, ArcTransientResource};

pub struct TransientBuffer {
    pub resource: wgpu::Buffer,
    pub desc: BufferInfo,
}

impl IntoArcTransientResource for TransientBuffer {
    fn into_arc_transient_resource(self: Arc<Self>) -> ArcTransientResource {
        ArcTransientResource::Buffer(self)
    }
}

impl From<BufferInfo> for AnyFrameGraphResourceDescriptor {
    fn from(value: BufferInfo) -> Self {
        AnyFrameGraphResourceDescriptor::Buffer(value)
    }
}

impl TransientResourceDescriptor for BufferInfo {
    type Resource = TransientBuffer;
}

impl TransientResource for TransientBuffer {
    type Descriptor = BufferInfo;

    fn borrow_resource(res: &AnyTransientResource) -> &Self {
        match res {
            AnyTransientResource::OwnedBuffer(res) => res,
            AnyTransientResource::ImportedBuffer(res) => res,
            _ => {
                unimplemented!()
            }
        }
    }

    fn get_desc(&self) -> &Self::Descriptor {
        &self.desc
    }
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
