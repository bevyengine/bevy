use wgpu::{BindGroupEntry, BindGroupLayoutEntry, Label};

use crate::{
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{AsBindGroup, BindGroup, BindGroupLayout},
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, FromDescriptorRenderResource, IntoRenderResource,
    RenderDependencies, RenderHandle, RenderResource,
};

impl RenderResource for BindGroupLayout {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_layout_direct(None, resource)
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
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_layout_direct(Some(descriptor), resource)
    }

    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        todo!()
    }
}

impl FromDescriptorRenderResource for BindGroupLayout {
    fn new_from_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_layout_descriptor(descriptor)
    }
}

pub struct RenderGraphBindGroups {}

impl RenderResource for BindGroup {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_direct(RenderDependencies::new(), resource)
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
