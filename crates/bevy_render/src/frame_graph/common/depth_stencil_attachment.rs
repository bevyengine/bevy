use crate::frame_graph::RenderContext;

use super::{ResourceBinding, TextureView};

#[derive(Clone)]
pub struct DepthStencilAttachmentDrawing {
    pub view: TextureView,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl ResourceBinding for DepthStencilAttachmentDrawing {
    type Resource = DepthStencilAttachment;

    fn make_resource<'a>(&self, render_context: &RenderContext<'a>) -> Self::Resource {
        let view = self.view.make_resource(render_context);

        DepthStencilAttachment {
            view,
            depth_ops: self.depth_ops,
            stencil_ops: self.stencil_ops,
        }
    }
}

pub struct DepthStencilAttachment {
    pub view: wgpu::TextureView,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl DepthStencilAttachment {
    pub fn get_render_pass_depth_stencil_attachment(
        &self,
    ) -> wgpu::RenderPassDepthStencilAttachment {
        wgpu::RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: self.depth_ops,
            stencil_ops: self.stencil_ops,
        }
    }
}
