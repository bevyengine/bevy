use bevy_ecs::world::World;
use wgpu::BufferDescriptor;

use crate::{
    render_graph_v2::{seal, RenderGraph, RenderGraphPersistentResources},
    render_resource::Buffer,
    renderer::RenderDevice,
};

use super::{
    IntoRenderResource, RenderResource, RenderResourceInit, RenderStore, SimpleRenderStore,
    WriteRenderResource,
};

impl seal::Super for Buffer {}

impl RenderResource for Buffer {
    type Descriptor = BufferDescriptor<'static>;
    type Data = Buffer;
    type Store<'g> = SimpleRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        &graph.buffers
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        &mut graph.buffers
    }

    fn get_persistent_store(
        persistent_resources: &RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &<Self::Store<'static> as RenderStore<'static, Self>>::PersistentStore {
        &persistent_resources.dummy
    }

    fn get_persistent_store_mut<'g>(
        persistent_resources: &mut RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &mut <Self::Store<'static> as RenderStore<'static, Self>>::PersistentStore {
        &mut persistent_resources.dummy
    }

    fn from_data<'a>(data: &'a Self::Data, _world: &'a World) -> Option<&'a Self> {
        Some(data)
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        _world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        render_device.create_buffer(descriptor)
    }
}

impl WriteRenderResource for Buffer {}

impl<'g> IntoRenderResource<'g> for BufferDescriptor<'static> {
    type Resource = Buffer;

    fn into_render_resource(
        self,
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(self)
    }
}
