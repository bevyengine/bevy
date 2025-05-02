use std::ops::Range;

use crate::{
    camera::Viewport,
    frame_graph::{
        BindGroupBluePrint, BluePrint, ColorAttachment, ColorAttachmentBluePrint,
        DepthStencilAttachmentBluePrint, FrameGraphBuffer, FrameGraphError, RenderContext,
        RenderPassBlutPrint, RenderPassContext, ResourceRead, ResourceRef,
    },
    render_resource::{BindGroup, CachedRenderPipelineId},
};

use super::PassTrait;

#[derive(Default)]
pub struct RenderPass {
    render_pass: RenderPassBlutPrint,
    commands: Vec<RenderPassCommand>,
    vaild: bool,
}

pub enum RenderPassCommand {
    SetScissorRect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
    SetRenderPipeline(CachedRenderPipelineId),
    SetBindGroup {
        index: u32,
        bind_group_ref: BindGroupBluePrint,
        offsets: Vec<u32>,
    },
    SetVertexBuffer {
        slot: u32,
        buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    },
    SetIndexBuffer {
        buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    },
    SetRawBindGroup {
        index: u32,
        bind_group: Option<BindGroup>,
        offsets: Vec<u32>,
    },
    DrawIndexed {
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    },
}

impl RenderPassCommand {
    pub fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        match &self {
            RenderPassCommand::SetScissorRect {
                x,
                y,
                width,
                height,
            } => {
                render_pass_context.set_scissor_rect(*x, *y, *width, *height);
            }
            RenderPassCommand::Draw {
                vertices,
                instances,
            } => {
                render_pass_context.draw(vertices.clone(), instances.clone());
            }
            RenderPassCommand::SetRenderPipeline(id) => {
                render_pass_context.set_render_pipeline(*id)?;
            }
            RenderPassCommand::SetBindGroup {
                index,
                bind_group_ref,
                offsets,
            } => {
                render_pass_context.set_bind_group(*index, bind_group_ref, offsets)?;
            }
            RenderPassCommand::SetVertexBuffer { slot, buffer_ref } => {
                render_pass_context.set_vertex_buffer(*slot, buffer_ref)?;
            }
            RenderPassCommand::SetIndexBuffer {
                buffer_ref,
                index_format,
            } => {
                render_pass_context.set_index_buffer(buffer_ref, *index_format)?;
            }
            RenderPassCommand::SetRawBindGroup {
                index,
                bind_group,
                offsets,
            } => {
                render_pass_context.set_raw_bind_group(*index, bind_group.as_ref(), offsets)?;
            }
            RenderPassCommand::DrawIndexed {
                indices,
                base_vertex,
                instances,
            } => {
                render_pass_context.draw_indexed(indices.clone(), *base_vertex, instances.clone());
            }
        }

        Ok(())
    }
}

impl RenderPass {
    pub fn is_vaild(&self) -> bool {
        self.vaild
    }

    pub fn set_viewport(&mut self, viewport: Option<Viewport>) {
        if let Some(viewport) = viewport {
            let size = viewport.physical_size;
            let position = viewport.physical_position;
            self.set_scissor_rect(position.x, position.y, size.x, size.y);
        }
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.commands.push(RenderPassCommand::DrawIndexed {
            indices,
            base_vertex,
            instances,
        });
    }

    pub fn set_raw_bind_group(
        &mut self,
        index: u32,
        bind_group: Option<&BindGroup>,
        offsets: &[u32],
    ) {
        self.commands.push(RenderPassCommand::SetRawBindGroup {
            index,
            bind_group: bind_group.map(|bind_group| bind_group.clone()),
            offsets: offsets.to_vec(),
        });
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.commands.push(RenderPassCommand::SetScissorRect {
            x,
            y,
            width,
            height,
        });
    }

    pub fn set_depth_stencil_attachment(
        &mut self,
        depth_stencil_attachment: DepthStencilAttachmentBluePrint,
    ) {
        self.render_pass.depth_stencil_attachment = Some(depth_stencil_attachment);
    }

    pub fn add_raw_color_attachment(&mut self, color_attachment: ColorAttachment) {
        self.render_pass
            .raw_color_attachments
            .push(color_attachment);

        self.vaild = true;
    }

    pub fn add_color_attachment(&mut self, color_attachment: ColorAttachmentBluePrint) {
        self.render_pass.color_attachments.push(color_attachment);

        self.vaild = true;
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.commands.push(RenderPassCommand::Draw {
            vertices,
            instances,
        });
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    ) {
        self.commands.push(RenderPassCommand::SetIndexBuffer {
            buffer_ref: buffer_ref.clone(),
            index_format,
        });
    }

    pub fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
    ) {
        self.commands.push(RenderPassCommand::SetVertexBuffer {
            slot,
            buffer_ref: buffer_ref.clone(),
        });
    }

    pub fn set_render_pipeline(&mut self, id: CachedRenderPipelineId) {
        self.commands.push(RenderPassCommand::SetRenderPipeline(id));
    }

    pub fn set_bind_group(&mut self, index: u32, bind_group_ref: &BindGroupBluePrint, offsets: &[u32]) {
        self.commands.push(RenderPassCommand::SetBindGroup {
            index,
            bind_group_ref: bind_group_ref.clone(),
            offsets: offsets.to_vec(),
        });
    }
}

impl PassTrait for RenderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        let render_pass_info = self.render_pass.make(render_context)?;

        let mut tracked_render_pass = render_context.begin_render_pass(&render_pass_info)?;

        for command in self.commands.iter() {
            command.draw(&mut tracked_render_pass)?;
        }

        tracked_render_pass.end();
        Ok(())
    }

    fn set_pass_name(&mut self, name: &str) {
        self.render_pass.label = Some(name.to_string().into());
    }
}
