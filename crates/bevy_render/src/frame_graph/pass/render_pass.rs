use std::mem::take;

use crate::{
    camera::Viewport,
    frame_graph::{
        ColorAttachment, ColorAttachmentDrawing, DepthStencilAttachmentDrawing, FrameGraphError,
        RenderContext, RenderPassCommand, RenderPassCommandBuilder, RenderPassDrawing,
        ResourceDrawing,
    },
};

use super::PassTrait;

#[derive(Default)]
pub struct RenderPass {
    logic_render_passes: Vec<LogicRenderPass>,
    current_logic_render_pass: LogicRenderPass,
}

#[derive(Default)]
pub struct LogicRenderPass {
    render_pass_drawing: RenderPassDrawing,
    commands: Vec<RenderPassCommand>,
    vaild: bool,
}

impl RenderPass {
    pub fn is_vaild(&self) -> bool {
        if self.logic_render_passes.is_empty() {
            return false;
        } else {
            return true;
        }
    }

    pub fn finish(&mut self) {
        let sub_render_pass = take(&mut self.current_logic_render_pass);

        if self.current_logic_render_pass.vaild {
            self.logic_render_passes.push(sub_render_pass);
        }
    }

    pub fn set_camera_viewport(&mut self, viewport: Option<Viewport>) {
        if let Some(viewport) = viewport {
            self.set_viewport(
                viewport.physical_position.x as f32,
                viewport.physical_position.y as f32,
                viewport.physical_size.x as f32,
                viewport.physical_size.y as f32,
                viewport.depth.start,
                viewport.depth.end,
            );
        }
    }

    pub fn set_depth_stencil_attachment(
        &mut self,
        depth_stencil_attachment: DepthStencilAttachmentDrawing,
    ) {
        self.current_logic_render_pass
            .render_pass_drawing
            .depth_stencil_attachment = Some(depth_stencil_attachment);
    }

    pub fn add_raw_color_attachment(&mut self, color_attachment: ColorAttachment) {
        self.current_logic_render_pass
            .render_pass_drawing
            .raw_color_attachments
            .push(color_attachment);

        self.current_logic_render_pass.vaild = true;
    }

    pub fn add_color_attachment(&mut self, color_attachment: ColorAttachmentDrawing) {
        self.current_logic_render_pass
            .render_pass_drawing
            .color_attachments
            .push(color_attachment);

        self.current_logic_render_pass.vaild = true;
    }
}

impl RenderPassCommandBuilder for RenderPass {
    fn add_render_pass_command(&mut self, value: RenderPassCommand) {
        self.current_logic_render_pass.commands.push(value);
    }
}

impl PassTrait for RenderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        let mut command_encoder = render_context.create_command_encoder();

        for logic_render_pass in self.logic_render_passes.iter() {
            let render_pass_info = logic_render_pass
                .render_pass_drawing
                .make_resource(render_context)?;
            let render_pass_context =
                render_context.begin_render_pass(&mut command_encoder, &render_pass_info)?;

            render_pass_context.execute(&logic_render_pass.commands)?;
        }

        let command_buffer = command_encoder.finish();

        render_context.add_command_buffer(command_buffer);
        Ok(())
    }
}
