use crate::render::{
    Vertex,
    {
        render_graph_2::{
            mesh_draw_target, PassDescriptor, PipelineDescriptor, RenderGraphBuilder,
            RenderPassColorAttachmentDescriptor, RenderPassDepthStencilAttachmentDescriptor,
            resource,
        },
        shader::{Shader, ShaderStage},
    },
};
pub trait ForwardPipelineBuilder {
    fn add_forward_pipeline(self) -> Self;
}

impl ForwardPipelineBuilder for RenderGraphBuilder {
    fn add_forward_pipeline(self) -> Self {
        self.add_pipeline(
            "forward",
            PipelineDescriptor::build(Shader::from_glsl(
                include_str!("forward.vert"),
                ShaderStage::Vertex,
            ))
            .with_fragment_shader(Shader::from_glsl(
                include_str!("forward.frag"),
                ShaderStage::Fragment,
            ))
            .with_rasterization_state(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            })
            .with_depth_stencil_state(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            })
            .with_color_state(wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            })
            .with_vertex_buffer_descriptor(Vertex::get_vertex_buffer_descriptor())
            .with_draw_target(mesh_draw_target)
            .build(),
        )
    }
}

pub trait ForwardPassBuilder {
    fn add_forward_pass(self) -> Self;
}

impl ForwardPassBuilder for RenderGraphBuilder {
    fn add_forward_pass(self) -> Self {
        self.add_pass(
            "main",
            PassDescriptor {
                color_attachments: vec![RenderPassColorAttachmentDescriptor {
                    attachment: resource::texture::SWAP_CHAIN.to_string(),
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
