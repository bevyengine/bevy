use crate::frame_graph::{ExtraResource, FrameGraphError, RenderContext};

use super::TextureViewRef;

pub struct ColorAttachmentRef {
    pub view_ref: TextureViewRef,
    pub resolve_target: Option<TextureViewRef>,
    pub ops: wgpu::Operations<wgpu::Color>,
}

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
            ops: self.ops.clone(),
        }
    }
}

impl ExtraResource for ColorAttachmentRef {
    type Resource = ColorAttachment;

    fn extra_resource(
        &self,
        resource_context: &RenderContext,
    ) -> Result<Self::Resource, FrameGraphError> {
        let view = self.view_ref.extra_resource(resource_context)?;

        if let Some(resolve_target) = &self.resolve_target {
            let resolve_target = resolve_target.extra_resource(resource_context)?;

            Ok(ColorAttachment {
                view,
                resolve_target: Some(resolve_target),
                ops: self.ops.clone(),
            })
        } else {
            Ok(ColorAttachment {
                view,
                resolve_target: None,
                ops: self.ops.clone(),
            })
        }
    }
}
