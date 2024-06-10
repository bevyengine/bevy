use std::borrow::Cow;

use bevy_render::{
    render_resource::{
        encase::internal::WriteInto, Buffer, BufferDescriptor, BufferUsages, ImageDataLayout,
        ShaderType, UniformBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
};

use crate::core::{Label, NodeContext, RenderGraphBuilder};

use super::{
    IntoRenderResource, RenderHandle, RenderResource, ResourceType, UsagesRenderResource,
    WriteRenderResource,
};

impl RenderResource for Buffer {
    const RESOURCE_TYPE: ResourceType = ResourceType::Buffer;
    type Meta<'g> = RenderGraphBufferMeta;

    #[inline]
    fn import_resource<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_buffer(meta, resource)
    }

    #[inline]
    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_buffer(resource)
    }

    #[inline]
    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_buffer_meta(resource)
    }

    #[inline]
    fn meta_label<'g>(meta: &Self::Meta<'g>) -> Label<'g> {
        let label = meta.descriptor.label?;
        Some(Cow::Borrowed(label))
    }
}

impl WriteRenderResource for Buffer {}

impl UsagesRenderResource for Buffer {
    type Usages = BufferUsages;

    #[inline]
    fn get_meta_mut<'a, 'b: 'a, 'g: 'b>(
        graph: &'a mut RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Meta<'g>> {
        graph.get_buffer_meta_mut(resource)
    }

    #[inline]
    fn has_usages(meta: &Self::Meta<'_>, usages: &Self::Usages) -> bool {
        meta.descriptor.usage.contains(*usages)
    }

    #[inline]
    fn add_usages(meta: &mut Self::Meta<'_>, usages: Self::Usages) {
        meta.descriptor.usage.insert(usages);
    }
}

#[derive(Clone)]
pub struct RenderGraphBufferMeta {
    pub descriptor: BufferDescriptor<'static>,
    pub layout: Option<ImageDataLayout>,
}

impl From<BufferDescriptor<'static>> for RenderGraphBufferMeta {
    fn from(descriptor: BufferDescriptor<'static>) -> Self {
        RenderGraphBufferMeta {
            descriptor,
            layout: None,
        }
    }
}

impl<'g> IntoRenderResource<'g> for RenderGraphBufferMeta {
    type Resource = Buffer;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_buffer(self)
    }
}

impl<'g> IntoRenderResource<'g> for BufferDescriptor<'static> {
    type Resource = Buffer;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_buffer(self.into())
    }
}

impl<'g, T: ShaderType + WriteInto> IntoRenderResource<'g> for UniformBuffer<T> {
    type Resource = Buffer;

    fn into_render_resource(
        mut self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        let render_device = graph.world_resource::<RenderDevice>();
        let render_queue = graph.world_resource::<RenderQueue>();
        self.write_buffer(render_device, render_queue);
        graph.into_resource(self.descriptor().into(), self.buffer().unwrap().clone())
    }
}
