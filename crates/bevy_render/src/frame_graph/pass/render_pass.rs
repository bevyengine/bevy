use std::mem::take;

use wgpu::CommandEncoder;

use crate::{
    camera::Viewport,
    frame_graph::{
        ColorAttachment, ColorAttachmentOwner, DepthStencilAttachment, RenderContext,
        RenderPassCommand, RenderPassCommandBuilder, RenderPassInfo, ResourceBinding,
    },
};

use super::EncoderExecutor;

#[derive(Default)]
pub struct RenderPass {
    logic_render_passes: Vec<LogicRenderPass>,
    current_logic_render_pass: LogicRenderPass,
}

#[derive(Default)]
pub struct LogicRenderPass {
    render_pass_drawing: RenderPassInfo,
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
        let logic_render_pass = take(&mut self.current_logic_render_pass);

        if logic_render_pass.vaild {
            self.logic_render_passes.push(logic_render_pass);
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

    pub fn set_pass_name(&mut self, name: &str) {
        self.current_logic_render_pass.render_pass_drawing.label = Some(name.to_string().into());
    }

    pub fn set_depth_stencil_attachment(
        &mut self,
        depth_stencil_attachment: DepthStencilAttachment,
    ) {
        self.current_logic_render_pass
            .render_pass_drawing
            .depth_stencil_attachment = Some(depth_stencil_attachment);
    }

    pub fn add_raw_color_attachment(&mut self, color_attachment: Option<ColorAttachmentOwner>) {
        self.current_logic_render_pass
            .render_pass_drawing
            .raw_color_attachments
            .push(color_attachment);

        self.current_logic_render_pass.vaild = true;
    }

    pub fn add_color_attachments(&mut self, mut color_attachments: Vec<Option<ColorAttachment>>) {
        self.current_logic_render_pass
            .render_pass_drawing
            .color_attachments
            .append(&mut color_attachments);

        self.current_logic_render_pass.vaild = true;
    }
    pub fn add_color_attachment(&mut self, color_attachment: Option<ColorAttachment>) {
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

impl EncoderExecutor for RenderPass {
    fn execute(&self, command_encoder: &mut CommandEncoder, render_context: &mut RenderContext) {
        for logic_render_pass in self.logic_render_passes.iter() {
            let render_pass_info = logic_render_pass
                .render_pass_drawing
                .make_resource(render_context);
            let render_pass_context =
                render_context.begin_render_pass(command_encoder, &render_pass_info);

            render_pass_context.execute(&logic_render_pass.commands);
        }
    }
}
