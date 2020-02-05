use crate::render::{
        render_graph_2::{
            resource_name, PassDescriptor,
            RenderGraphBuilder, RenderPassColorAttachmentDescriptor,
        },
    };

pub trait ForwardPassBuilder {
    fn add_forward_pass(self) -> Self;
}

impl ForwardPassBuilder for RenderGraphBuilder {
    fn add_forward_pass(self) -> Self {
        self.add_pass(
            "main",
            PassDescriptor {
                color_attachments: vec![RenderPassColorAttachmentDescriptor {
                    attachment: resource_name::texture::SWAP_CHAIN.to_string(),
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 0.3,
                        g: 0.4,
                        b: 0.5,
                        a: 1.0,
                    },
                }],
                depth_stencil_attachment: None,
                // depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                //     attachment: "forward_depth".to_string(),
                //     depth_load_op: wgpu::LoadOp::Clear,
                //     depth_store_op: wgpu::StoreOp::Store,
                //     stencil_load_op: wgpu::LoadOp::Clear,
                //     stencil_store_op: wgpu::StoreOp::Store,
                //     clear_depth: 1.0,
                //     clear_stencil: 0,
                // }),
                sample_count: 1,
            },
        )
    }
}