use crate::frame_graph::{BluePrint, FrameGraphError, RenderContext};

use super::TextureViewBluePrint;

#[derive(Clone)]
pub struct DepthStencilAttachmentBluePrint {
    pub view_ref: TextureViewBluePrint,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl BluePrint for DepthStencilAttachmentBluePrint {
    type Product = DepthStencilAttachment;

    fn make(
        &self,
        render_context: &RenderContext,
    ) -> Result<Self::Product, FrameGraphError> {
        let view = self.view_ref.make(render_context)?;

        Ok(DepthStencilAttachment {
            view,
            depth_ops: self.depth_ops,
            stencil_ops: self.stencil_ops,
        })
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
