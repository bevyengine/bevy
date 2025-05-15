use crate::frame_graph::RenderContext;

use super::{ResourceBinding, TextureView};

#[derive(Clone)]
pub struct DepthStencilAttachment {
    pub view: TextureView,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl ResourceBinding for DepthStencilAttachment {
    type Resource = DepthStencilAttachmentOwner;

    fn make_resource<'a>(&self, render_context: &RenderContext<'a>) -> Self::Resource {
        let view = self.view.make_resource(render_context);

        DepthStencilAttachmentOwner {
            view,
            depth_ops: self.depth_ops,
            stencil_ops: self.stencil_ops,
        }
    }
}

pub struct DepthStencilAttachmentOwner {
    pub view: wgpu::TextureView,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl DepthStencilAttachmentOwner {
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
