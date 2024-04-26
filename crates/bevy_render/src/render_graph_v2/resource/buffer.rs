use bevy_ecs::world::World;
use wgpu::BufferDescriptor;

use crate::{
    render_graph_v2::{seal, RenderGraph},
    render_resource::Buffer,
    renderer::RenderDevice,
};

use super::{
    IntoRenderResource, RenderResource, RenderResourceInit, SimpleRenderStore, WriteRenderResource,
};

impl seal::Super for Buffer {}

impl RenderResource for Buffer {
    type Descriptor = BufferDescriptor<'static>;
    type Data = Buffer;
    type Store<'g> = SimpleRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        todo!()
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        todo!()
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        todo!()
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        todo!()
    }
}

impl WriteRenderResource for Buffer {}

// impl RetainedRenderResource for Buffer {}

impl<'g> IntoRenderResource<'g> for BufferDescriptor<'static> {
    type Resource = Buffer;

    fn into_render_resource(
        self,
        world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(self)
    }
}
