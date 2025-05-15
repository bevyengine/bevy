use crate::frame_graph::RenderContext;

use super::{ResourceBinding, TextureViewDrawing};

#[derive(Clone)]
pub struct ColorAttachmentDrawing {
    pub view: TextureViewDrawing,
    pub resolve_target: Option<TextureViewDrawing>,
    pub ops: wgpu::Operations<wgpu::Color>,
}

#[derive(Clone)]
pub struct ColorAttachment {
    pub view: wgpu::TextureView,
    pub resolve_target: Option<wgpu::TextureView>,
    pub ops: wgpu::Operations<wgpu::Color>,
}

impl ColorAttachment {
    pub fn get_render_pass_color_attachment(&self) -> wgpu::RenderPassColorAttachment {
        wgpu::RenderPassColorAttachment {
            view: &self.view,
            resolve_target: self.resolve_target.as_ref(),
            ops: self.ops,
        }
    }
}

impl ResourceBinding for ColorAttachmentDrawing {
    type Resource = ColorAttachment;

    fn make_resource<'a>(&self, render_context: &RenderContext<'a>) -> Self::Resource {
        let view = self.view.make_resource(render_context);

        if let Some(resolve_target) = &self.resolve_target {
            let resolve_target = resolve_target.make_resource(render_context);

            ColorAttachment {
                view,
                resolve_target: Some(resolve_target),
                ops: self.ops,
            }
        } else {
            ColorAttachment {
                view,
                resolve_target: None,
                ops: self.ops,
            }
        }
    }
}
