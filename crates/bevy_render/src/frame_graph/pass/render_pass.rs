use crate::frame_graph::{
    ColorAttachmentRef, FrameGraphError, RenderContext, RenderPassInfo, TrackedRenderPass,
};

use super::PassTrait;

#[derive(Default)]
pub struct RenderPass {
    render_pass_info: RenderPassInfo,
    render_draw_functions: Vec<Box<dyn RenderDrawFunction>>,
}

impl RenderPass {
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
    fn draw(&self, tracked_render_pass: &mut TrackedRenderPass) -> Result<(), FrameGraphError>;
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
