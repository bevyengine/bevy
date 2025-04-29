use crate::frame_graph::{
    ColorAttachmentRef, DepthStencilAttachmentRef, FrameGraphError, RenderContext, RenderPassContext, RenderPassInfo
};

use super::PassTrait;

#[derive(Default)]
pub struct RenderPass {
    render_pass_info: RenderPassInfo,
    render_draw_functions: Vec<Box<dyn RenderDrawFunction>>,
}

impl RenderPass {
    pub fn set_depth_stencil_attachment(
        &mut self,
        depth_stencil_attachment: DepthStencilAttachmentRef,
    ) {
        self.render_pass_info.depth_stencil_attachment = Some(depth_stencil_attachment);
    }

    pub fn add_color_attachment(&mut self, color_attachment: ColorAttachmentRef) {
        self.render_pass_info
            .color_attachments
            .push(color_attachment);
    }

    pub fn add_draw_function<T: RenderDrawFunction>(&mut self, draw_function: T) {
        self.render_draw_functions.push(Box::new(draw_function));
    }
}

pub trait RenderDrawFunction: 'static + Send + Sync {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError>;
}

impl PassTrait for RenderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        let mut tracked_render_pass = render_context.begin_render_pass(&self.render_pass_info)?;

        for render_draw_function in self.render_draw_functions.iter() {
            render_draw_function.draw(&mut tracked_render_pass)?;
        }

        tracked_render_pass.end();
        Ok(())
    }
}
