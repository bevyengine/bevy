use super::{
    FrameGraphBuffer, FrameGraphError, GraphResource, RenderPassInfo, ResourceRead, ResourceRef,
    ResourceTable,
};
use crate::renderer::RenderDevice;
use std::ops::Range;

pub trait ExtraResource {
    type Resource;
    fn extra_resource(
        &self,
        resource_context: &RenderContext,
    ) -> Result<Self::Resource, FrameGraphError>;
}

pub struct RenderContext {
    render_device: RenderDevice,
    pub(crate) resource_table: ResourceTable,
    command_buffer_queue: Vec<wgpu::CommandBuffer>,
}

impl RenderContext {
    pub fn begin_render_pass<'a>(
        &'a mut self,
        render_pass_info: &RenderPassInfo,
    ) -> Result<TrackedRenderPass<'a>, FrameGraphError> {
        let mut command_encoder = self
            .render_device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let render_pass =
            render_pass_info.create_render_pass(&self, &mut command_encoder)?;

        Ok(TrackedRenderPass {
            command_encoder,
            render_pass,
            render_context: self,
        })
    }

    pub fn get_resource<ResourceType: GraphResource>(
        &self,
        resource_ref: &ResourceRef<ResourceType, ResourceRead>,
    ) -> Result<&ResourceType, FrameGraphError> {
        self.resource_table
            .get_resource(resource_ref)
            .ok_or(FrameGraphError::ResourceNotFound)
    }

    pub fn add_command_buffer(&mut self, command_buffer: wgpu::CommandBuffer) {
        self.command_buffer_queue.push(command_buffer);
    }
}

pub struct TrackedRenderPass<'a> {
    command_encoder: wgpu::CommandEncoder,
    render_pass: wgpu::RenderPass<'static>,
    render_context: &'a mut RenderContext,
}

impl<'a> TrackedRenderPass<'a> {
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.render_pass.draw(vertices, instances);
    }

    pub fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(&buffer_ref)?;
        self.render_pass
            .set_vertex_buffer(slot, buffer.resource.slice(0..));

        Ok(())
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(&buffer_ref)?;

        self.render_pass
            .set_index_buffer(buffer.resource.slice(0..), index_format);

        Ok(())
    }

    pub fn end(self) {
        drop(self.render_pass);
        let command_buffer = self.command_encoder.finish();
        self.render_context.add_command_buffer(command_buffer);
    }
}
