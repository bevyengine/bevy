use crate::{
    pipeline::state_descriptors::{
        BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
        CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace,
        RasterizationStateDescriptor, StencilStateFaceDescriptor,
    },
    render_graph::RenderGraphBuilder,
    render_resource::resource_name,
    shader::{Shader, ShaderStage},
    texture::TextureFormat,
};
pub trait UiPipelineBuilder {
    fn add_ui_pipeline(&mut self) -> &mut Self;
}

impl<'a, 'b, 'c> UiPipelineBuilder for RenderGraphBuilder<'a, 'b, 'c> {
    fn add_ui_pipeline(&mut self) -> &mut Self {
        self.add_pipeline(resource_name::pipeline::UI, |builder| {
            builder
                .with_vertex_shader(Shader::from_glsl(
                    ShaderStage::Vertex,
                    include_str!("ui.vert"),
                ))
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
                .add_draw_target(resource_name::draw_target::UI);
        })
    }
}
