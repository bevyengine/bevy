use crate::{
    asset::AssetStorage,
    render::{
        pipeline::{
            state_descriptors::{
                BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
                CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace,
                RasterizationStateDescriptor, StencilStateFaceDescriptor,
            },
            InputStepMode, PipelineDescriptor, VertexAttributeDescriptor, VertexBufferDescriptor,
            VertexFormat,
        },
        render_graph::RenderGraphBuilder,
        render_resource::{resource_name, resource_providers::RectData},
        shader::{Shader, ShaderStage},
        texture::TextureFormat,
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
                resource_name::pipeline::UI,
                shader_storage,
                Shader::from_glsl(ShaderStage::Vertex, include_str!("ui.vert")),
            )
            .with_fragment_shader(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("ui.frag"),
            ))
            .with_rasterization_state(RasterizationStateDescriptor {
                front_face: FrontFace::Ccw,
                cull_mode: CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            })
            .with_depth_stencil_state(DepthStencilStateDescriptor {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil_front: StencilStateFaceDescriptor::IGNORE,
                stencil_back: StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            })
            .add_color_state(ColorStateDescriptor {
                format: TextureFormat::Bgra8UnormSrgb,
                color_blend: BlendDescriptor {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha_blend: BlendDescriptor {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                write_mask: ColorWrite::ALL,
            })
            .add_vertex_buffer_descriptor(Vertex::get_vertex_buffer_descriptor())
            .add_vertex_buffer_descriptor(VertexBufferDescriptor {
                stride: std::mem::size_of::<RectData>() as u64,
                step_mode: InputStepMode::Instance,
                attributes: vec![
                    VertexAttributeDescriptor {
                        format: VertexFormat::Float2,
                        offset: 0,
                        shader_location: 3,
                    },
                    VertexAttributeDescriptor {
                        format: VertexFormat::Float2,
                        offset: 2 * 4,
                        shader_location: 4,
                    },
                    VertexAttributeDescriptor {
                        format: VertexFormat::Float4,
                        offset: 4 * 4,
                        shader_location: 5,
                    },
                    VertexAttributeDescriptor {
                        format: VertexFormat::Float,
                        offset: 8 * 4,
                        shader_location: 6,
                    },
                ],
            })
            .add_draw_target(resource_name::draw_target::UI)
            .finish(),
        )
    }
}
