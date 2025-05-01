use crate::frame_graph::{BluePrint, FrameGraphError, RenderContext};

use super::TextureViewRef;

#[derive(Clone)]
pub struct DepthStencilAttachmentRef {
    pub view_ref: TextureViewRef,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl BluePrint for DepthStencilAttachmentRef {
    type Product = DepthStencilAttachment;

    fn make(
        &self,
        resource_context: &RenderContext,
    ) -> Result<Self::Product, FrameGraphError> {
        let view = self.view_ref.make(resource_context)?;

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
