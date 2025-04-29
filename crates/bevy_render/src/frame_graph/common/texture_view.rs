use alloc::borrow::Cow;

use crate::frame_graph::{
    ExtraResource, FrameGraphError, FrameGraphTexture, RenderContext, ResourceRead, ResourceRef,
};

#[derive(Default, Clone)]
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

pub struct TextureViewRef {
    pub texture_ref: ResourceRef<FrameGraphTexture, ResourceRead>,
    pub desc: TextureViewInfo,
}

impl ExtraResource for TextureViewRef {
    type Resource = wgpu::TextureView;
    fn extra_resource(
        &self,
        resource_context: &RenderContext,
    ) -> Result<Self::Resource, FrameGraphError> {
        resource_context
            .resource_table
            .get_resource::<FrameGraphTexture>(&self.texture_ref)
            .map(|texture| {
                texture
                    .resource
                    .create_view(&self.desc.get_texture_view_desc())
            })
            .ok_or(FrameGraphError::ResourceNotFound)
    }
}
