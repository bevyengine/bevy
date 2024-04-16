use bevy_ecs::world::World;
use wgpu::BufferDescriptor;

use crate::{render_graph_v2::RenderGraph, render_resource::Buffer, renderer::RenderDevice};

use super::{
    IntoRenderResource, RenderResource, RenderResourceId, RenderResourceInit, RenderResourceMeta,
    RetainedRenderResource, SimpleResourceStore, WriteRenderResource,
};

impl RenderResource for Buffer {
    type Descriptor = BufferDescriptor<'static>;
    type Data = Buffer;
    type Store = SimpleResourceStore<Self>;

    fn get_store(graph: &RenderGraph) -> &Self::Store {
        &graph.buffers
    }

    fn get_store_mut(graph: &mut RenderGraph) -> &mut Self::Store {
        &mut graph.buffers
    }

    fn from_data<'a>(data: &'a Self::Data, _world: &'a World) -> Option<&'a Self> {
        Some(data)
    }
}

impl WriteRenderResource for Buffer {}

impl RetainedRenderResource for Buffer {}

impl IntoRenderResource for BufferDescriptor<'static> {
    type Resource = Buffer;

    fn into_render_resource(
        self,
        _world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        let buf = render_device.create_buffer(&self);
        let meta = RenderResourceMeta {
            descriptor: Some(self),
            resource: buf,
        };
        RenderResourceInit::Eager(meta)
    }
}
