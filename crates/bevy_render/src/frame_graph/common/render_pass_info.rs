use crate::frame_graph::{BluePrint, FrameGraphError, RenderContext};

use super::{ColorAttachmentRef, DepthStencilAttachmentRef};

#[derive(Default)]
pub struct RenderPassInfo {
    pub color_attachments: Vec<ColorAttachmentRef>,
    pub depth_stencil_attachment: Option<DepthStencilAttachmentRef>,
}

impl RenderPassInfo {
    pub fn create_render_pass(
        &self,
        resource_context: &RenderContext,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<wgpu::RenderPass<'static>, FrameGraphError> {
        let mut color_attachments = vec![];

        for color_attachment in self.color_attachments.iter() {
            color_attachments.push(color_attachment.make(resource_context)?);
        }

        let mut depth_stencil_attachment = None;

        if let Some(depth_stencil_attachment_ref) = &self.depth_stencil_attachment {
            depth_stencil_attachment =
                Some(depth_stencil_attachment_ref.make(resource_context)?);
        }

        let depth_stencil_attachment =
            depth_stencil_attachment
                .as_ref()
                .map(|depth_stencil_attachment| {
                    depth_stencil_attachment.get_render_pass_depth_stencil_attachment()
                });

        let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments
                .iter()
                .map(|color_attachment| Some(color_attachment.get_render_pass_color_attachment()))
                .collect::<Vec<_>>(),
            depth_stencil_attachment,
            ..Default::default()
        });

        let render_pass = render_pass.forget_lifetime();

        Ok(render_pass)
    }
}
