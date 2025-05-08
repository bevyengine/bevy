use std::num::NonZero;

use crate::frame_graph::{
    FrameGraphBuffer, FrameGraphTexture, ResourceRead, ResourceRef, SamplerInfo, TextureViewInfo,
};

#[derive(Clone)]
pub struct BindGroupEntryRef {
    pub binding: u32,
    pub resource: BindingResourceRef,
}

#[derive(Clone)]
pub enum BindingResourceRef {
    Buffer {
        buffer: ResourceRef<FrameGraphBuffer, ResourceRead>,
        size: Option<NonZero<u64>>,
    },
    Sampler(SamplerInfo),
    TextureView {
        texture: ResourceRef<FrameGraphTexture, ResourceRead>,
        texture_view_info: TextureViewInfo,
    },
}

pub trait IntoBindingResourceRef {
    fn into_binding(self) -> BindingResourceRef;
}

impl IntoBindingResourceRef for &ResourceRef<FrameGraphBuffer, ResourceRead> {
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::Buffer {
            buffer: self.clone(),
            size: None,
        }
    }
}

impl IntoBindingResourceRef for &SamplerInfo {
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::Sampler(self.clone())
    }
}

impl IntoBindingResourceRef for &ResourceRef<FrameGraphTexture, ResourceRead> {
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::TextureView {
            texture: self.clone(),
            texture_view_info: TextureViewInfo::default(),
        }
    }
}

impl IntoBindingResourceRef for &BindingResourceRef {
    fn into_binding(self) -> BindingResourceRef {
        self.clone()
    }
}

impl IntoBindingResourceRef
    for (
        &ResourceRef<FrameGraphTexture, ResourceRead>,
        &TextureViewInfo,
    )
{
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::TextureView {
            texture: self.0.clone(),
            texture_view_info: self.1.clone(),
        }
    }
}
