use bevy_ecs::world::World;
use wgpu::{BufferDescriptor, BufferUsages};

use crate::{
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::Buffer,
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, IntoRenderResource, RenderHandle, RenderResource,
    UsagesRenderResource, WriteRenderResource,
};

impl RenderResource for Buffer {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl WriteRenderResource for Buffer {}

impl DescribedRenderResource for Buffer {
    type Descriptor = BufferDescriptor<'static>;

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

impl UsagesRenderResource for Buffer {
    type Usages = BufferUsages;

    fn get_descriptor_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Descriptor> {
        todo!()
    }

    fn has_usages<'g>(descriptor: &Self::Descriptor, usages: &Self::Usages) -> bool {
        descriptor.usage.contains(*usages)
    }

    fn add_usages<'g>(descriptor: &mut Self::Descriptor, usages: Self::Usages) {
        descriptor.usage.insert(usages)
    }
}

impl<'g> IntoRenderResource<'g> for BufferDescriptor<'static> {
    type Resource = Buffer;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}
