use crate::{
    asset::AssetStorage,
    render::{
        pipeline::{PipelineDescriptor, VertexBufferDescriptor},
        render_graph::RenderGraphBuilder,
        render_resource::{resource_name, resource_providers::RectData},
        shader::{Shader, ShaderStage},
        Vertex,
    },
};
pub trait UiPipelineBuilder {
    fn add_ui_pipeline(
        self,
        pipeline_descriptor_storage: &mut AssetStorage<PipelineDescriptor>,
        shader_storage: &mut AssetStorage<Shader>,
    ) -> Self;
}

impl UiPipelineBuilder for RenderGraphBuilder {
    fn add_ui_pipeline(
        self,
        pipeline_descriptor_storage: &mut AssetStorage<PipelineDescriptor>,
        shader_storage: &mut AssetStorage<Shader>,
    ) -> Self {
        self.add_pipeline(
            pipeline_descriptor_storage,
            PipelineDescriptor::build(
                shader_storage,
                Shader::from_glsl(ShaderStage::Vertex, include_str!("ui.vert")),
            )
            .with_fragment_shader(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("ui.frag"),
            ))
            .with_rasterization_state(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            })
            .with_depth_stencil_state(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            })
            .add_color_state(wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                write_mask: wgpu::ColorWrite::ALL,
            })
            .add_vertex_buffer_descriptor(Vertex::get_vertex_buffer_descriptor())
            .add_vertex_buffer_descriptor(VertexBufferDescriptor {
                stride: std::mem::size_of::<RectData>() as u64,
                step_mode: wgpu::InputStepMode::Instance,
                attributes: vec![
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 3,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 2 * 4,
                        shader_location: 4,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float4,
                        offset: 4 * 4,
                        shader_location: 5,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float,
                        offset: 8 * 4,
                        shader_location: 6,
                    },
                ],
            })
            .add_draw_target(resource_name::draw_target::UI)
            .build(),
        )
    }
}
