use std::ops::Range;

use crate::{
    frame_graph::{
        BindGroupRef, ColorAttachmentRef, DepthStencilAttachmentRef, FrameGraphError,
        RenderContext, RenderPassContext, RenderPassInfo,
    },
    render_resource::CachedRenderPipelineId,
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

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.add_draw_function((vertices, instances));
    }

    pub fn set_render_pipeline(&mut self, id: CachedRenderPipelineId) {
        self.add_draw_function(id);
    }

    pub fn set_bind_group(&mut self, index: u32, bind_group_ref: &BindGroupRef, offsets: &[u32]) {
        self.add_draw_function((index, bind_group_ref.clone(), offsets.to_vec()));
    }

    pub(crate) fn add_draw_function<T: RenderDrawFunction>(&mut self, draw_function: T) {
        self.render_draw_functions.push(Box::new(draw_function));
    }
}

impl RenderDrawFunction for (Range<u32>, Range<u32>) {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.draw(self.0.clone(), self.1.clone());

        Ok(())
    }
}

impl RenderDrawFunction for CachedRenderPipelineId {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_render_pipeline(*self)
    }
}

impl RenderDrawFunction for (u32, BindGroupRef, Vec<u32>) {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_bind_group(self.0, &self.1, &self.2)
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
