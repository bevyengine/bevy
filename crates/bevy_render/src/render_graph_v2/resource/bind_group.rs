use bevy_ecs::world::World;
use wgpu::BindGroupLayoutEntry;

use crate::{render_graph_v2::seal, render_resource::BindGroupLayout, renderer::RenderDevice};

use super::{CachedRenderStore, RenderResource};

impl seal::Super for BindGroupLayout {}

impl RenderResource for BindGroupLayout {
    type Descriptor = Box<[BindGroupLayoutEntry]>;
    type Data = Self;
    type Store = CachedRenderStore<Self>;

    fn get_store(
        graph: &crate::render_graph_v2::RenderGraph,
        _: crate::render_graph_v2::seal::Token,
    ) -> &Self::Store {
        &graph.bind_group_layouts
    }

    fn get_store_mut(
        graph: &mut crate::render_graph_v2::RenderGraph,
        _: crate::render_graph_v2::seal::Token,
    ) -> &mut Self::Store {
        &mut graph.bind_group_layouts
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        Some(data)
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        render_device.create_bind_group_layout(None, &descriptor)
    }
}
