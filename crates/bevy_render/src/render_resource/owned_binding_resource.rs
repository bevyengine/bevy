use crate::render_resource::{
    BindingResource, Buffer, BufferAddress, BufferBinding, BufferSize, Sampler, TextureView,
};

/// An owned binding resource of any type (ex: a [`Buffer`], [`TextureView`], etc).
/// This is used by types like [`crate::render_resource::PreparedBindGroup`] to hold a single list of all
/// render resources used by bindings.
pub enum OwnedBindingResource {
    TextureView(TextureView),
    Sampler(Sampler),
    Buffer {
        buffer: Buffer,
        offset: BufferAddress,
        size: Option<BufferSize>,
    },
    DynamicBuffer {
        buffer: Buffer,
        offset: BufferAddress,
        size: Option<BufferSize>,
        dynamic_offset: u32,
    },
}

impl OwnedBindingResource {
    pub fn new_from_buffer(buffer: Buffer) -> Self {
        Self::Buffer {
            buffer,
            offset: 0,
            size: None,
        }
    }

    pub fn with_dynamic_offset(self, dynamic_offset: u32) -> Self {
        match self {
            OwnedBindingResource::Buffer {
                buffer,
                offset,
                size,
            } => OwnedBindingResource::DynamicBuffer {
                buffer,
                offset,
                size,
                dynamic_offset,
            },
            _ => panic!("with_dynamic_offset should only be called on Buffer variant"),
        }
    }

    pub fn get_binding(&self) -> BindingResource {
        match self {
            OwnedBindingResource::TextureView(view) => BindingResource::TextureView(view),
            OwnedBindingResource::Sampler(sampler) => BindingResource::Sampler(sampler),
            OwnedBindingResource::Buffer {
                buffer,
                offset,
                size,
            }
            | OwnedBindingResource::DynamicBuffer {
                buffer,
                offset,
                size,
                ..
            } => BindingResource::Buffer(BufferBinding {
                buffer,
                offset: *offset,
                size: *size,
            }),
        }
    }

    pub fn dynamic_offset(&self) -> Option<u32> {
        match self {
            OwnedBindingResource::DynamicBuffer { dynamic_offset, .. } => Some(*dynamic_offset),
            _ => None,
        }
    }
}
