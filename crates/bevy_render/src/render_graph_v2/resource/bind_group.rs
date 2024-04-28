use bevy_ecs::world::World;
use wgpu::BindGroupLayoutEntry;

use crate::{
    render_graph_v2::{seal, RenderGraph, RenderGraphPersistentResources},
    render_resource::BindGroupLayout,
    renderer::RenderDevice,
};

use super::{CachedRenderStore, RenderResource, RenderStore};

impl seal::Super for BindGroupLayout {}

impl RenderResource for BindGroupLayout {
    type Descriptor = (&'static str, &'static [BindGroupLayoutEntry]);
    type Data = BindGroupLayout;
    type Store<'g> = CachedRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        &graph.bind_group_layouts
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        &mut graph.bind_group_layouts
    }

    fn get_persistent_store<'g>(
        persistent_resources: &RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &<Self::Store<'g> as RenderStore<'g, Self>>::PersistentStore {
        &persistent_resources.bind_group_layouts
    }

    fn get_persistent_store_mut<'g>(
        persistent_resources: &mut RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &mut <Self::Store<'g> as RenderStore<'g, Self>>::PersistentStore {
        &mut persistent_resources.bind_group_layouts
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        Some(data)
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        render_device.create_bind_group_layout(descriptor.0, descriptor.1)
    }
}
