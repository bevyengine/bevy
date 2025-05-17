use wgpu::{Origin3d, TextureAspect};

use crate::frame_graph::{
    FrameGraph, FrameGraphBuffer, FrameGraphTexture, GraphResource, GraphResourceNodeHandle,
    PassBuilder, PassNodeBuilder, ResourceMaterial, ResourceRead, ResourceWrite,
};

use super::{
    BindGroupResourceBinding, BindGroupResourceHandle, BindGroupResourceHandleHelper,
    BindGroupResourceHelper, BindingResourceTextureView, IntoBindGroupResourceHandle,
    TexelCopyTextureInfo, TextureViewInfo,
};

#[derive(Clone)]
pub struct TextureViewMeta {
    pub meta: ResourceMeta<FrameGraphTexture>,
    pub texture_view_info: TextureViewInfo,
}

impl BindGroupResourceHandleHelper for TextureViewMeta {
    fn make_bind_group_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupResourceHandle {
        let texture = self.meta.imported(frame_graph);

        (&texture, &self.texture_view_info).into_binding()
    }
}

pub struct ResourceMeta<ResourceType: GraphResource> {
    pub key: String,
    pub desc: <ResourceType as GraphResource>::Descriptor,
}

impl BindGroupResourceHandleHelper for ResourceMeta<FrameGraphTexture> {
    fn make_bind_group_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupResourceHandle {
        let texture = self.imported(frame_graph);

        texture.into_binding()
    }
}

impl BindGroupResourceHelper for ResourceMeta<FrameGraphTexture> {
    fn make_binding_group_resource_binding(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindGroupResourceBinding {
        let texture = pass_node_builder.read_material(self);

        BindGroupResourceBinding::TextureView(BindingResourceTextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        })
    }
}

impl ResourceMeta<FrameGraphTexture> {
    pub fn make_binding_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupResourceHandle {
        let texture = self.imported(frame_graph);
        texture.into_binding()
    }

    pub fn get_image_copy_read(
        &self,
        pass_builder: &mut PassBuilder,
    ) -> TexelCopyTextureInfo<ResourceRead> {
        let texture = pass_builder.read_material(self);
        TexelCopyTextureInfo {
            mip_level: 0,
            texture,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        }
    }

    pub fn get_image_copy_write(
        &self,
        pass_builder: &mut PassBuilder,
    ) -> TexelCopyTextureInfo<ResourceWrite> {
        let texture = pass_builder.write_material(self);
        TexelCopyTextureInfo {
            mip_level: 0,
            texture,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        }
    }
}

impl<ResourceType: GraphResource> Clone for ResourceMeta<ResourceType> {
    fn clone(&self) -> Self {
        ResourceMeta {
            key: self.key.clone(),
            desc: self.desc.clone(),
        }
    }
}

impl ResourceMaterial for ResourceMeta<FrameGraphTexture> {
    type ResourceType = FrameGraphTexture;

    fn imported(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> GraphResourceNodeHandle<Self::ResourceType> {
        frame_graph.get_or_create(&self.key, self.desc.clone())
    }
}

impl ResourceMaterial for ResourceMeta<FrameGraphBuffer> {
    type ResourceType = FrameGraphBuffer;

    fn imported(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> GraphResourceNodeHandle<Self::ResourceType> {
        frame_graph.get_or_create(&self.key, self.desc.clone())
    }
}
