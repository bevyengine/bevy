use crate::frame_graph::{FrameGraph, FrameGraphBuffer, FrameGraphTexture, GraphResource, GraphResourceNodeHandle};

use super::ResourceMaterial;

pub struct ResourceMeta<ResourceType: GraphResource> {
    pub key: String,
    pub desc: <ResourceType as GraphResource>::Descriptor,
}

impl ResourceMaterial for ResourceMeta<FrameGraphTexture> {
    type ResourceType = FrameGraphTexture;

    fn make_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> GraphResourceNodeHandle<Self::ResourceType> {
        frame_graph.get_or_create(&self.key, self.desc.clone())
    }
}

impl ResourceMaterial for ResourceMeta<FrameGraphBuffer> {
    type ResourceType = FrameGraphBuffer;

    fn make_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> GraphResourceNodeHandle<Self::ResourceType> {
        frame_graph.get_or_create(&self.key, self.desc.clone())
    }
}
