use crate::{
    camera::Viewport,
    frame_graph::{
        ColorAttachment, ColorAttachmentDrawing, DepthStencilAttachmentDrawing,
        FrameGraphError, RenderContext, RenderPassBlutPrint, RenderPassCommand,
        RenderPassCommandBuilder, ResourceDrawing,
    },
};

use super::PassTrait;

#[derive(Default)]
pub struct RenderPass {
    render_pass: RenderPassBlutPrint,
    commands: Vec<RenderPassCommand>,
    vaild: bool,
}

impl RenderPass {
    pub fn is_vaild(&self) -> bool {
        self.vaild
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
        self.render_pass.depth_stencil_attachment = Some(depth_stencil_attachment);
    }

    pub fn add_raw_color_attachment(&mut self, color_attachment: ColorAttachment) {
        self.render_pass
            .raw_color_attachments
            .push(color_attachment);

        self.vaild = true;
    }

    pub fn add_color_attachment(&mut self, color_attachment: ColorAttachmentDrawing) {
        self.render_pass.color_attachments.push(color_attachment);

        self.vaild = true;
    }
}

impl RenderPassCommandBuilder for RenderPass {
    fn add_render_pass_command(&mut self, value: RenderPassCommand) {
        self.commands.push(value);
    }
}

impl PassTrait for RenderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        let render_pass_info = self.render_pass.make_resource(render_context)?;
        let render_pass_context = render_context.begin_render_pass(&render_pass_info)?;

        render_pass_context.execute(&self.commands)?;

        Ok(())
    }

    fn set_pass_name(&mut self, name: &str) {
        self.render_pass.label = Some(name.to_string().into());
    }
}
