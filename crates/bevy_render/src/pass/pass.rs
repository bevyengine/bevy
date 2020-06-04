use super::{LoadOp, StoreOp};
use crate::{render_resource::RenderResource, Color};

#[derive(Debug, Clone)]
pub enum TextureAttachment {
    RenderResource(RenderResource),
    Name(String),
    Input(String),
}

impl TextureAttachment {
    pub fn get_resource(&self) -> Option<RenderResource> {
        if let TextureAttachment::RenderResource(render_resource) = self {
            Some(*render_resource)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderPassColorAttachmentDescriptor {
    /// The actual color attachment.
    pub attachment: TextureAttachment,

    /// The resolve target for this color attachment, if any.
    pub resolve_target: Option<TextureAttachment>,

    /// The beginning-of-pass load operation for this color attachment.
    pub load_op: LoadOp,

    /// The end-of-pass store operation for this color attachment.
    pub store_op: StoreOp,

    /// The color that will be assigned to every pixel of this attachment when cleared.
    pub clear_color: Color,
}

#[derive(Debug, Clone)]
pub struct RenderPassDepthStencilAttachmentDescriptor {
    pub attachment: TextureAttachment,
    pub depth_load_op: LoadOp,
    pub depth_store_op: StoreOp,
    pub clear_depth: f32,
    pub stencil_load_op: LoadOp,
    pub stencil_store_op: StoreOp,
    pub depth_read_only: bool,
    pub stencil_read_only: bool,
    pub clear_stencil: u32,
}

// A set of pipeline bindings and draw calls with color and depth outputs
#[derive(Debug, Clone)]
pub struct PassDescriptor {
    pub color_attachments: Vec<RenderPassColorAttachmentDescriptor>,
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachmentDescriptor>,
    pub sample_count: u32,
}
