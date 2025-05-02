use crate::frame_graph::{BluePrint, FrameGraphError, RenderContext};

use super::TextureViewBluePrint;

pub struct ColorAttachmentBluePrint {
    pub view_ref: TextureViewBluePrint,
    pub resolve_target: Option<TextureViewBluePrint>,
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

impl BluePrint for ColorAttachmentBluePrint {
    type Product = ColorAttachment;

    fn make(&self, render_context: &RenderContext) -> Result<Self::Product, FrameGraphError> {
        let view = self.view_ref.make(render_context)?;

        if let Some(resolve_target) = &self.resolve_target {
            let resolve_target = resolve_target.make(render_context)?;

            Ok(ColorAttachment {
                view,
                resolve_target: Some(resolve_target),
                ops: self.ops,
            })
        } else {
            Ok(ColorAttachment {
                view,
                resolve_target: None,
                ops: self.ops,
            })
        }
    }
}
