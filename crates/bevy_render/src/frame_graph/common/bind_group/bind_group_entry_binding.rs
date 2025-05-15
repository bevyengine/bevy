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
    pub resource: BindGroupResourceBinding,
}

#[derive(Clone)]
pub enum BindGroupResourceBinding {
    Buffer(BindingResourceBuffer),
    Sampler(Sampler),
    TextureView(BindingResourceTextureView),
    TextureViewArray(Vec<BindingResourceTextureView>),
}

#[derive(Clone)]
pub struct BindingResourceBuffer {
    pub buffer: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub size: Option<NonZero<u64>>,
}

#[derive(Clone)]
pub struct BindingResourceTextureView {
    pub texture: ResourceRef<FrameGraphTexture, ResourceRead>,
    pub texture_view_info: TextureViewInfo,
}

pub trait IntoBindGroupResourceBinding {
    fn into_binding(self) -> BindGroupResourceBinding;
}

impl IntoBindGroupResourceBinding for BindingResourceBuffer {
    fn into_binding(self) -> BindGroupResourceBinding {
        BindGroupResourceBinding::Buffer(self)
    }
}

impl IntoBindGroupResourceBinding for &Sampler {
    fn into_binding(self) -> BindGroupResourceBinding {
        BindGroupResourceBinding::Sampler(self.clone())
    }
}

impl IntoBindGroupResourceBinding for &ResourceRef<FrameGraphTexture, ResourceRead> {
    fn into_binding(self) -> BindGroupResourceBinding {
        BindGroupResourceBinding::TextureView(BindingResourceTextureView {
            texture: self.clone(),
            texture_view_info: TextureViewInfo::default(),
        })
    }
}

impl IntoBindGroupResourceBinding for BindGroupResourceBinding {
    fn into_binding(self) -> BindGroupResourceBinding {
        self
    }
}

impl IntoBindGroupResourceBinding for &BindGroupResourceBinding {
    fn into_binding(self) -> BindGroupResourceBinding {
        self.clone()
    }
}

impl IntoBindGroupResourceBinding
    for (
        &ResourceRef<FrameGraphTexture, ResourceRead>,
        &TextureViewInfo,
    )
{
    fn into_binding(self) -> BindGroupResourceBinding {
        BindGroupResourceBinding::TextureView(BindingResourceTextureView {
            texture: self.0.clone(),
            texture_view_info: self.1.clone(),
        })
    }
}
