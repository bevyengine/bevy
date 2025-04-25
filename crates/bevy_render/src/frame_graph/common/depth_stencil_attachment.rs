use crate::frame_graph::{ExtraResource, FrameGraphError, RenderContext};

use super::TextureViewRef;

pub struct DepthStencilAttachmentRef {
    pub view_ref: TextureViewRef,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl ExtraResource for DepthStencilAttachmentRef {
    type Resource = DepthStencilAttachment;

    fn extra_resource(
        &self,
        resource_context: &RenderContext,
    ) -> Result<Self::Resource, FrameGraphError> {
        let view = self.view_ref.extra_resource(resource_context)?;

        Ok(DepthStencilAttachment {
            view,
            depth_ops: self.depth_ops.clone(),
            stencil_ops: self.stencil_ops.clone(),
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
            depth_ops: self.depth_ops.clone(),
            stencil_ops: self.stencil_ops.clone(),
        }
    }
}
