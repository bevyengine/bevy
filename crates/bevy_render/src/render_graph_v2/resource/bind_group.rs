use wgpu::{BindGroupEntry, BindGroupLayoutEntry, Label};

use crate::{
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{AsBindGroup, BindGroup, BindGroupLayout},
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, IntoRenderResource, RenderHandle, RenderResource,
};

impl RenderResource for BindGroupLayout {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext<'a>,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_bind_group_layout(resource)
    }
}

impl DescribedRenderResource for BindGroupLayout {
    type Descriptor = Box<[BindGroupLayoutEntry]>;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Option<Self::Descriptor>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_descriptor<'g>(
        graph: &RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'g Self::Descriptor> {
        todo!()
    }
}

impl RenderResource for BindGroup {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext<'a>,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

pub struct RenderGraphBindGroup<'g, F: FnOnce(NodeContext<'g>) -> &'g [BindGroupEntry<'g>] + 'g> {
    label: Label<'g>,
    layout: RenderHandle<'g, BindGroupLayout>,
    bind_group: F,
}

impl<'g, F: FnOnce(NodeContext<'g>) -> &'g [BindGroupEntry<'g>] + 'g> IntoRenderResource<'g>
    for RenderGraphBindGroup<'g, F>
{
    type Resource = BindGroup;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}

pub struct AsRenderGraphBindGroup<'g, T: AsBindGroup, F: FnOnce(NodeContext<'g>) -> T + 'g> {
    label: Label<'g>,
    bind_group: F,
}

impl<'g, T: AsBindGroup, F: FnOnce(NodeContext<'g>) -> T + 'g> IntoRenderResource<'g>
    for AsRenderGraphBindGroup<'g, T, F>
{
    type Resource = BindGroup;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}
