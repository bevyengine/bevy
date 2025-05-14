use std::num::NonZero;

use crate::{
    frame_graph::{
        FrameGraphBuffer, FrameGraphTexture, ResourceRead, ResourceRef, TextureViewInfo,
    },
    render_resource::Sampler,
};

#[derive(Clone)]
pub struct BindGroupEntryBinding {
    pub binding: u32,
    pub resource: BindingResourceRef,
}

#[derive(Clone)]
pub enum BindingResourceRef {
    Buffer(BindingResourceBufferRef),
    Sampler(Sampler),
    TextureView {
        texture: ResourceRef<FrameGraphTexture, ResourceRead>,
        texture_view_info: TextureViewInfo,
    },
    TextureViewArray(Vec<BindingResourceTextureViewRef>),
}

#[derive(Clone)]
pub struct BindingResourceBufferRef {
    pub buffer: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub size: Option<NonZero<u64>>,
}

#[derive(Clone)]
pub struct BindingResourceTextureViewRef {
    pub texture: ResourceRef<FrameGraphTexture, ResourceRead>,
    pub texture_view_info: TextureViewInfo,
}

pub trait IntoBindingResourceRef {
    fn into_binding(self) -> BindingResourceRef;
}

impl IntoBindingResourceRef for BindingResourceBufferRef {
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::Buffer(self)
    }
}

impl IntoBindingResourceRef for &Sampler {
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
