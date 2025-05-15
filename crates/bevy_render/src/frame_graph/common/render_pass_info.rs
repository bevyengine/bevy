use std::borrow::Cow;

use crate::frame_graph::RenderContext;

use super::{
    ColorAttachment, ColorAttachmentOwner, DepthStencilAttachment, DepthStencilAttachmentOwner,
    ResourceBinding,
};

#[derive(Default)]
pub struct RenderPassDrawing {
    pub label: Option<Cow<'static, str>>,
    pub color_attachments: Vec<Option<ColorAttachment>>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
    pub raw_color_attachments: Vec<Option<ColorAttachmentOwner>>,
}

pub struct RenderPassInfo {
    pub label: Option<Cow<'static, str>>,
    pub color_attachments: Vec<Option<ColorAttachmentOwner>>,
    pub depth_stencil_attachment: Option<DepthStencilAttachmentOwner>,
}

impl ResourceBinding for RenderPassDrawing {
    type Resource = RenderPassInfo;

    fn make_resource<'a>(&self, render_context: &RenderContext<'a>) -> Self::Resource {
        let mut color_attachments = self.raw_color_attachments.clone();

        for color_attachment in self.color_attachments.iter() {
            if color_attachment.is_none() {
                color_attachments.push(None);
            } else {
                color_attachments.push(Some(
                    color_attachment
                        .as_ref()
                        .unwrap()
                        .make_resource(render_context),
                ));
            }
        }

        let mut depth_stencil_attachment = None;

        if let Some(depth_stencil_attachment_blue_print) = &self.depth_stencil_attachment {
            depth_stencil_attachment =
                Some(depth_stencil_attachment_blue_print.make_resource(render_context));
        }

        RenderPassInfo {
            label: self.label.clone(),
            color_attachments,
            depth_stencil_attachment,
        }
    }
}

impl RenderPassInfo {
    pub fn create_render_pass(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'static> {
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
                .map(|color_attachment| {
                    color_attachment.as_ref().and_then(|color_attachment| {
                        Some(color_attachment.get_render_pass_color_attachment())
                    })
                })
                .collect::<Vec<_>>(),
            depth_stencil_attachment,
            ..Default::default()
        });

        let render_pass = render_pass.forget_lifetime();

        render_pass
    }
}
