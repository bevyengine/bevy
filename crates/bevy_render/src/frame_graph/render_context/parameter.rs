use std::ops::Range;

use crate::{
    frame_graph::{
        BindGroupBluePrint, FrameGraphBuffer, FrameGraphError, RenderPassContext, ResourceRead,
        ResourceRef,
    },
    render_resource::{BindGroup, CachedRenderPipelineId},
};

use super::ErasedRenderPassCommand;

pub struct DrawIndexedParameter {
    pub indices: Range<u32>,
    pub base_vertex: i32,
    pub instances: Range<u32>,
}

impl ErasedRenderPassCommand for DrawIndexedParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.draw_indexed(
            self.indices.clone(),
            self.base_vertex,
            self.instances.clone(),
        );
        Ok(())
    }
}

pub struct SetRawBindGroupParameter {
    pub index: u32,
    pub bind_group: Option<BindGroup>,
    pub offsets: Vec<u32>,
}

impl ErasedRenderPassCommand for SetRawBindGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_raw_bind_group(
            self.index,
            self.bind_group.as_ref(),
            &self.offsets,
        )?;
        Ok(())
    }
}

pub struct SetScissorRectParameter {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ErasedRenderPassCommand for SetScissorRectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_scissor_rect(self.x, self.y, self.width, self.height);
        Ok(())
    }
}

pub struct DrawParameter {
    pub vertices: Range<u32>,
    pub instances: Range<u32>,
}

impl ErasedRenderPassCommand for DrawParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.draw(self.vertices.clone(), self.instances.clone());
        Ok(())
    }
}

pub struct SetIndexBufferParameter {
    pub buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub index_format: wgpu::IndexFormat,
}

impl ErasedRenderPassCommand for SetIndexBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_index_buffer(&self.buffer_ref, self.index_format)?;
        Ok(())
    }
}

pub struct SetVertexBufferParameter {
    pub slot: u32,
    pub buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
}

impl ErasedRenderPassCommand for SetVertexBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_vertex_buffer(self.slot, &self.buffer_ref)?;
        Ok(())
    }
}

pub struct SetRenderPipelineParameter {
    pub id: CachedRenderPipelineId,
}

impl ErasedRenderPassCommand for SetRenderPipelineParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_render_pipeline(self.id)?;
        Ok(())
    }
}

pub struct SetBindGroupParameter {
    pub index: u32,
    pub bind_group: BindGroupBluePrint,
    pub offsets: Vec<u32>,
}

impl ErasedRenderPassCommand for SetBindGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_bind_group(self.index, &self.bind_group, &self.offsets)?;
        Ok(())
    }
}
