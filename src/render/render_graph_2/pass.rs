pub struct RenderPassColorAttachmentDescriptor {
    /// The actual color attachment.
    pub attachment: String,

    /// The resolve target for this color attachment, if any.
    pub resolve_target: Option<String>,

    /// The beginning-of-pass load operation for this color attachment.
    pub load_op: wgpu::LoadOp,

    /// The end-of-pass store operation for this color attachment.
    pub store_op: wgpu::StoreOp,

    /// The color that will be assigned to every pixel of this attachment when cleared.
    pub clear_color: wgpu::Color,
}

pub struct RenderPassDepthStencilAttachmentDescriptor {
    pub attachment: String,
    pub depth_load_op: wgpu::LoadOp,
    pub depth_store_op: wgpu::StoreOp,
    pub clear_depth: f32,
    pub stencil_load_op: wgpu::LoadOp,
    pub stencil_store_op: wgpu::StoreOp,
    pub clear_stencil: u32,
}

// A set of pipeline bindings and draw calls with color and depth outputs
pub struct PassDescriptor {
    pub color_attachments: Vec<RenderPassColorAttachmentDescriptor>,
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachmentDescriptor>,
    pub sample_count: u32,
}