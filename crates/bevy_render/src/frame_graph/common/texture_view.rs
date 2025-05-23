use alloc::borrow::Cow;

use crate::frame_graph::{TransientTexture, RenderContext, Ref, ResourceWrite};

use super::ResourceBinding;

#[derive(Default, Clone, Debug)]
pub struct TextureViewInfo {
    pub label: Option<Cow<'static, str>>,
    pub format: Option<wgpu::TextureFormat>,
    pub dimension: Option<wgpu::TextureViewDimension>,
    pub usage: Option<wgpu::TextureUsages>,
    pub aspect: wgpu::TextureAspect,
    pub base_mip_level: u32,
    pub mip_level_count: Option<u32>,
    pub base_array_layer: u32,
    pub array_layer_count: Option<u32>,
}

impl From<wgpu::TextureViewDescriptor<'_>> for TextureViewInfo {
    fn from(value: wgpu::TextureViewDescriptor) -> Self {
        TextureViewInfo {
            label: value
                .label
                .map(|label| label.to_string())
                .map(|label| label.into()),
            format: value.format,
            dimension: value.dimension,
            usage: value.usage,
            aspect: value.aspect,
            base_mip_level: value.base_mip_level,
            mip_level_count: value.mip_level_count,
            base_array_layer: value.base_array_layer,
            array_layer_count: value.array_layer_count,
        }
    }
}

impl TextureViewInfo {
    pub fn get_texture_view_desc(&self) -> wgpu::TextureViewDescriptor {
        wgpu::TextureViewDescriptor {
            label: self.label.as_deref(),
            format: self.format,
            dimension: self.dimension,
            usage: self.usage,
            aspect: self.aspect,
            base_mip_level: self.base_mip_level,
            mip_level_count: self.mip_level_count,
            base_array_layer: self.base_array_layer,
            array_layer_count: self.array_layer_count,
        }
    }
}

#[derive(Clone)]
pub struct TextureView {
    pub texture: Ref<TransientTexture, ResourceWrite>,
    pub desc: TextureViewInfo,
}

impl ResourceBinding for TextureView {
    type Resource = wgpu::TextureView;

    fn make_resource<'a>(&self, render_context: &RenderContext<'a>) -> Self::Resource {
        render_context
            .get_resource(&self.texture)
            .resource
            .create_view(&self.desc.get_texture_view_desc())
    }
}
