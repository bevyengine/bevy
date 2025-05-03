use crate::{
    camera::Viewport,
    frame_graph::{
        BluePrint, ColorAttachment, ColorAttachmentBluePrint, DepthStencilAttachmentBluePrint,
        FrameGraphError, RenderContext, RenderPassBlutPrint, RenderPassCommand, RenderPassContextExecutor,
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

    pub fn set_viewport(&mut self, viewport: Option<Viewport>) {
        if let Some(viewport) = viewport {
            let size = viewport.physical_size;
            let position = viewport.physical_position;
            self.set_scissor_rect(position.x, position.y, size.x, size.y);
        }
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
}

impl RenderPassContextExecutor for RenderPass {
    fn add_render_pass_command(&mut self, value: RenderPassCommand) {
        self.commands.push(value);
    }

    fn get_render_pass_commands(&self) -> &[RenderPassCommand] {
        &self.commands
    }
}

impl PassTrait for RenderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        let render_pass_info = self.render_pass.make(render_context)?;
        let render_pass_context = render_context.begin_render_pass(&render_pass_info)?;

        RenderPassContextExecutor::execute(self, render_pass_context)?;

        Ok(())
    }

    fn set_pass_name(&mut self, name: &str) {
        self.render_pass.label = Some(name.to_string().into());
    }
}
