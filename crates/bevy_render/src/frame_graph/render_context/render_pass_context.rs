use super::{
    BindGroupBluePrint, BluePrint, DrawIndexedParameter, DrawParameter, FrameGraphBuffer, FrameGraphError, RenderContext, ResourceRead, ResourceRef, SetBindGroupParameter, SetIndexBufferParameter, SetRawBindGroupParameter, SetRenderPipelineParameter, SetScissorRectParameter, SetVertexBufferParameter
};
use crate::render_resource::{BindGroup, CachedRenderPipelineId};
use core::ops::Range;
use std::ops::Deref;

pub trait RenderPassContextExecutor {
    fn add_render_pass_command(&mut self, value: RenderPassCommand);

    fn get_render_pass_commands(&self) -> &[RenderPassCommand];

    fn execute(&self, mut render_pass_context: RenderPassContext) -> Result<(), FrameGraphError> {
        for command in self.get_render_pass_commands() {
            command.draw(&mut render_pass_context)?;
        }

        render_pass_context.end();

        Ok(())
    }

    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.add_render_pass_command(RenderPassCommand::new(DrawIndexedParameter {
            indices,
            base_vertex,
            instances,
        }));
    }

    fn set_raw_bind_group(&mut self, index: u32, bind_group: Option<&BindGroup>, offsets: &[u32]) {
        self.add_render_pass_command(RenderPassCommand::new(SetRawBindGroupParameter {
            index,
            bind_group: bind_group.map(|bind_group| bind_group.clone()),
            offsets: offsets.to_vec(),
        }));
    }

    fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.add_render_pass_command(RenderPassCommand::new(SetScissorRectParameter {
            x,
            y,
            width,
            height,
        }));
    }

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.add_render_pass_command(RenderPassCommand::new(DrawParameter {
            vertices,
            instances,
        }));
    }

    fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(SetIndexBufferParameter {
            buffer_ref: buffer_ref.clone(),
            index_format,
        }));
    }

    fn set_render_pipeline(&mut self, id: CachedRenderPipelineId) {
        self.add_render_pass_command(RenderPassCommand::new(SetRenderPipelineParameter { id }));
    }

    fn set_bind_group(&mut self, index: u32, bind_group: &BindGroupBluePrint, offsets: &[u32]) {
        self.add_render_pass_command(RenderPassCommand::new(SetBindGroupParameter {
            index,
            bind_group: bind_group.clone(),
            offsets: offsets.to_vec(),
        }));
    }

    fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(SetVertexBufferParameter {
            slot,
            buffer_ref: buffer_ref.clone(),
        }));
    }
}

pub struct RenderPassCommand(Box<dyn ErasedRenderPassCommand>);

impl RenderPassCommand {
    pub fn new<T: ErasedRenderPassCommand>(value: T) -> Self {
        Self(Box::new(value))
    }

    pub fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        self.0.draw(render_pass_context)
    }
}

pub trait ErasedRenderPassCommand: Sync + Send + 'static {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError>;
}

pub struct RenderPassContext<'a, 'b> {
    command_encoder: wgpu::CommandEncoder,
    render_pass: wgpu::RenderPass<'static>,
    render_context: &'b mut RenderContext<'a>,
}

impl<'a, 'b> RenderPassContext<'a, 'b> {
    pub fn new(
        command_encoder: wgpu::CommandEncoder,
        render_pass: wgpu::RenderPass<'static>,
        render_context: &'b mut RenderContext<'a>,
    ) -> Self {
        RenderPassContext {
            command_encoder,
            render_pass,
            render_context,
        }
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.render_pass.set_scissor_rect(x, y, width, height);
    }

    pub fn set_raw_bind_group(
        &mut self,
        index: u32,
        bind_group: Option<&BindGroup>,
        offsets: &[u32],
    ) -> Result<(), FrameGraphError> {
        self.render_pass.set_bind_group(
            index,
            bind_group.map(|bind_group| bind_group.deref()),
            offsets,
        );

        Ok(())
    }

    pub fn set_bind_group(
        &mut self,
        index: u32,
        bind_group_ref: &BindGroupBluePrint,
        offsets: &[u32],
    ) -> Result<(), FrameGraphError> {
        let bind_group = bind_group_ref.make(&self.render_context)?;
        self.render_pass.set_bind_group(index, &bind_group, offsets);

        Ok(())
    }

    pub fn set_render_pipeline(
        &mut self,
        id: CachedRenderPipelineId,
    ) -> Result<(), FrameGraphError> {
        let pipeline = self.render_context.get_render_pipeline(id)?;
        self.render_pass.set_pipeline(pipeline);

        Ok(())
    }

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
        let buffer = self.render_context.get_resource(buffer_ref)?;
        self.render_pass
            .set_vertex_buffer(slot, buffer.resource.slice(0..));

        Ok(())
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(buffer_ref)?;

        self.render_pass
            .set_index_buffer(buffer.resource.slice(0..), index_format);

        Ok(())
    }

    fn end(self) {
        drop(self.render_pass);
        let command_buffer = self.command_encoder.finish();
        self.render_context.add_command_buffer(command_buffer);
    }
}
