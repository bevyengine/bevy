use bevy_ecs::world::World;
use wgpu::BindGroupLayoutEntry;

use crate::{
    render_graph_v2::{seal, RenderGraph},
    render_resource::BindGroupLayout,
    renderer::RenderDevice,
};

use super::{CachedRenderStore, RenderResource};

impl seal::Super for BindGroupLayout {}

impl RenderResource for BindGroupLayout {
    type Descriptor = Box<[BindGroupLayoutEntry]>;

    type Data = BindGroupLayout;

    type Store<'g> = CachedRenderStore<'g, Self>;

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
