use alloc::borrow::Cow;

use crate::frame_graph::{
    FrameGraphError, FrameGraphTexture, RenderContext, ResourceRef, ResourceWrite,
};

use super::ResourceDrawing;

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

#[derive(Clone)]
pub struct TextureViewDrawing {
    pub texture: ResourceRef<FrameGraphTexture, ResourceWrite>,
    pub desc: TextureViewInfo,
}

impl ResourceDrawing for TextureViewDrawing {
    type Resource = wgpu::TextureView;

    fn make_resource<'a>(
        &self,
        render_context: &RenderContext<'a>,
    ) -> Result<Self::Resource, FrameGraphError> {
        render_context
            .resource_table
            .get_resource(&self.texture)
            .map(|texture| {
                texture
                    .resource
                    .create_view(&self.desc.get_texture_view_desc())
            })
            .ok_or(FrameGraphError::ResourceNotFound)
    }
}
