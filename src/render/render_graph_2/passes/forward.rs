use crate::render::render_graph_2::{
    resource_name, PassDescriptor, RenderGraphBuilder, RenderPassColorAttachmentDescriptor,
    RenderPassDepthStencilAttachmentDescriptor, TextureDescriptor, TextureDimension,
};

pub trait ForwardPassBuilder {
    fn add_forward_pass(self) -> Self;
}

impl ForwardPassBuilder for RenderGraphBuilder {
    fn add_forward_pass(self) -> Self {
        self.add_texture(
            resource_name::texture::DEPTH,
            TextureDescriptor {
                size: wgpu::Extent3d {
                    depth: 1,
                    width: 2560,
                    height: 1440,
                },
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            },
        )
        .add_pass(
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
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                    attachment: resource_name::texture::DEPTH.to_string(),
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
                sample_count: 1,
            },
        )
    }
}
