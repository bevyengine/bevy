use bevy_render::pass::{
    LoadOp, PassDescriptor, RenderPassColorAttachmentDescriptor,
    RenderPassDepthStencilAttachmentDescriptor, StoreOp, TextureAttachment,
};

use bevy_render::Color;

pub fn build_main_pass() -> PassDescriptor {
    PassDescriptor {
        color_attachments: vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color".to_string()),
            resolve_target: None,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_color: Color::rgb(0.1, 0.1, 0.1),
        }],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
            attachment: TextureAttachment::Input("depth".to_string()),
            depth_load_op: LoadOp::Clear,
            depth_store_op: StoreOp::Store,
            stencil_load_op: LoadOp::Clear,
            stencil_store_op: StoreOp::Store,
            clear_depth: 1.0,
            clear_stencil: 0,
        }),
        sample_count: 1,
    }
}
