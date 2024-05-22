use std::borrow::Cow;

use bevy_render::render_resource::{Buffer, BufferDescriptor, BufferUsages};

use crate::core::{NodeContext, RenderGraphBuilder};

use super::{
    IntoRenderResource, RenderHandle, RenderResource, ResourceType, UsagesRenderResource,
    WriteRenderResource,
};

impl RenderResource for Buffer {
    const RESOURCE_TYPE: ResourceType = ResourceType::Buffer;
    type Meta<'g> = BufferDescriptor<'static>;

    fn import<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_buffer(meta, resource)
    }

    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_buffer(resource)
    }

    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_buffer_meta(resource)
    }
}

impl WriteRenderResource for Buffer {}

impl UsagesRenderResource for Buffer {
    type Usages = BufferUsages;

    fn get_meta_mut<'a, 'b: 'a, 'g: 'b>(
        graph: &'a mut RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Meta<'g>> {
        graph.get_buffer_meta_mut(resource)
    }

    fn has_usages<'g>(descriptor: &Self::Meta<'g>, usages: &Self::Usages) -> bool {
        descriptor.usage.contains(*usages)
    }

    fn add_usages<'g>(descriptor: &mut Self::Meta<'g>, usages: Self::Usages) {
        descriptor.usage.insert(usages);
    }
}

impl<'g> IntoRenderResource<'g> for BufferDescriptor<'static> {
    type Resource = Buffer;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_buffer(self)
    }
}
