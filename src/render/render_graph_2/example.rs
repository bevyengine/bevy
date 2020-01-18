use crate::render::{render_graph_2::*, shader::{Shader, ShaderStage}, Vertex};

fn build_example_graph() -> RenderGraph {
    // TODO: read this from swap chain
    let swap_chain_color_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    RenderGraph::build()
        .add_pass(
            "main",
            PassDescriptor {
                color_attachments: Vec::new(),
                depth_stencil_attachment: None,
                sample_count: 1,
            },
        )
        .add_pipeline(
            "forward",
            PipelineDescriptor::build(Shader::from_glsl(
                include_str!("../passes/forward/forward.vert"),
                ShaderStage::Vertex,
            ))
            .with_fragment_shader(Shader::from_glsl(
                include_str!("../passes/forward/forward.vert"),
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
                format: swap_chain_color_format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            })
            .with_vertex_buffer_descriptor(Vertex::get_vertex_buffer_descriptor())
            .with_draw_target(mesh_draw_target)
            .build()
        )
        .build()
}
