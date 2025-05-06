use std::borrow::Cow;

use crate::frame_graph::{FrameGraphError, RenderContext};

use super::{
    ColorAttachment, ColorAttachmentDrawing, DepthStencilAttachment, DepthStencilAttachmentDrawing,
    ResourceDrawing,
};

#[derive(Default)]
pub struct RenderPassBlutPrint {
    pub label: Option<Cow<'static, str>>,
    pub color_attachments: Vec<ColorAttachmentDrawing>,
    pub depth_stencil_attachment: Option<DepthStencilAttachmentDrawing>,
    pub raw_color_attachments: Vec<ColorAttachment>,
}

pub struct RenderPassInfo {
    pub label: Option<Cow<'static, str>>,
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
}

impl ResourceDrawing for RenderPassBlutPrint {
    type Resource = RenderPassInfo;

    fn make_resource<'a>(
        &self,
        render_context: &RenderContext<'a>,
    ) -> Result<Self::Resource, FrameGraphError> {
        let mut color_attachments = self.raw_color_attachments.clone();

        for color_attachment in self.color_attachments.iter() {
            color_attachments.push(color_attachment.make_resource(render_context)?);
        }

        let mut depth_stencil_attachment = None;

        if let Some(depth_stencil_attachment_blue_print) = &self.depth_stencil_attachment {
            depth_stencil_attachment =
                Some(depth_stencil_attachment_blue_print.make_resource(render_context)?);
        }

        Ok(RenderPassInfo {
            label: self.label.clone(),
            color_attachments,
            depth_stencil_attachment,
        })
    }
}

impl RenderPassInfo {
    pub fn create_render_pass(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<wgpu::RenderPass<'static>, FrameGraphError> {
        let depth_stencil_attachment =
            self.depth_stencil_attachment
                .as_ref()
                .map(|depth_stencil_attachment| {
                    depth_stencil_attachment.get_render_pass_depth_stencil_attachment()
                });

        let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label.as_deref(),
            color_attachments: &self
                .color_attachments
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
