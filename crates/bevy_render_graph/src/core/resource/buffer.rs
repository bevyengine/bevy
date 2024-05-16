use std::borrow::Cow;

use bevy_render::render_resource::{Buffer, BufferDescriptor, BufferUsages};

use crate::core::{NodeContext, RenderGraphBuilder};

use super::{
    DescribedRenderResource, FromDescriptorRenderResource, IntoRenderResource, RenderHandle,
    RenderResource, ResourceType, UsagesRenderResource, WriteRenderResource,
};

impl RenderResource for Buffer {
    const RESOURCE_TYPE: ResourceType = ResourceType::Buffer;
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_buffer_direct(None, resource)
    }

    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_buffer(resource)
    }
}

impl WriteRenderResource for Buffer {}

impl DescribedRenderResource for Buffer {
    type Descriptor = BufferDescriptor<'static>;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_buffer_direct(Some(descriptor), resource)
    }

    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        graph.get_buffer_descriptor(resource)
    }
}

impl FromDescriptorRenderResource for Buffer {
    fn new_from_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
    ) -> RenderHandle<'g, Self> {
        graph.new_buffer_descriptor(descriptor)
    }
}

impl UsagesRenderResource for Buffer {
    type Usages = BufferUsages;

    fn get_descriptor_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Descriptor> {
        graph.get_buffer_descriptor_mut(resource)
    }

    fn has_usages<'g>(descriptor: &Self::Descriptor, usages: &Self::Usages) -> bool {
        descriptor.usage.contains(*usages)
    }

    fn add_usages<'g>(descriptor: &mut Self::Descriptor, usages: Self::Usages) {
        descriptor.usage.insert(usages);
    }
}

impl<'g> IntoRenderResource<'g> for BufferDescriptor<'static> {
    type Resource = Buffer;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_buffer_descriptor(self)
    }
}
