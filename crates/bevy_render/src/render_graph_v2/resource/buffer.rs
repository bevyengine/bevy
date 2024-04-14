use bevy_ecs::world::World;
use wgpu::{util::BufferInitDescriptor, BufferDescriptor};

use crate::{
    render_graph_v2::RenderGraph,
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};

use super::{
    IntoRenderResource, LastFrameRenderResource, RenderResource, RenderResourceId,
    RenderResourceInit, RenderResourceMeta, WriteRenderResource,
};

impl RenderResource for Buffer {
    type Descriptor = BufferDescriptor<'static>;
    type Data = Buffer;

    fn insert_data<'a>(
        graph: &mut RenderGraph,
        key: RenderResourceId,
        data: RenderResourceInit<Self>,
    ) {
        graph.buffers.insert(key, data);
    }

    fn get_data<'a>(
        graph: &'a RenderGraph,
        _world: &'a World,
        key: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<Self>> {
        graph.buffers.get_data(key)
    }

    fn from_data<'a>(data: &'a Self::Data, _world: &'a World) -> Option<&'a Self> {
        Some(data)
    }
}

impl WriteRenderResource for Buffer {}

impl LastFrameRenderResource for Buffer {
    fn send_next_frame(graph: &mut RenderGraph, key: RenderResourceId) {
        todo!()
    }

    fn get_last_frame(
        graph: &RenderGraph,
        label: crate::render_graph::InternedRenderLabel,
    ) -> RenderResourceMeta<Self> {
        todo!()
    }
}

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
