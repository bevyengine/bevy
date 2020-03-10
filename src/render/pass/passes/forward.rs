use crate::render::{
    pass::{
        LoadOp, PassDescriptor, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor, StoreOp,
    },
    render_graph::RenderGraphBuilder,
    render_resource::{resource_name, resource_providers::FrameTextureResourceProvider},
    texture::{TextureDescriptor, TextureDimension},
    Color,
};

pub trait ForwardPassBuilder {
    fn add_forward_pass(self) -> Self;
}

impl ForwardPassBuilder for RenderGraphBuilder {
    fn add_forward_pass(self) -> Self {
        self.add_resource_provider(FrameTextureResourceProvider::new(
            resource_name::texture::DEPTH,
            TextureDescriptor {
                size: wgpu::Extent3d {
                    depth: 1,
                    width: 1,
                    height: 1,
                },
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            },
        ))
        .add_pass(
            resource_name::pass::MAIN,
            PassDescriptor {
                color_attachments: vec![RenderPassColorAttachmentDescriptor {
                    attachment: resource_name::texture::SWAP_CHAIN.to_string(),
                    resolve_target: None,
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
                    clear_color: Color::rgb(0.3, 0.4, 0.5),
                }],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                    attachment: resource_name::texture::DEPTH.to_string(),
                    depth_load_op: LoadOp::Clear,
                    depth_store_op: StoreOp::Store,
                    stencil_load_op: LoadOp::Clear,
                    stencil_store_op: StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
                sample_count: 1,
            },
        )
    }
}
