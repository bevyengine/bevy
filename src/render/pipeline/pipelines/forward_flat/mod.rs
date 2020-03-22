use crate::render::{
    pipeline::state_descriptors::{
        BlendDescriptor, ColorStateDescriptor, ColorWrite, CompareFunction, CullMode,
        DepthStencilStateDescriptor, FrontFace, RasterizationStateDescriptor,
        StencilStateFaceDescriptor,
    },
    render_graph::RenderGraphBuilder,
    render_resource::resource_name,
    shader::{Shader, ShaderStage},
    texture::TextureFormat,
};

pub trait ForwardFlatPipelineBuilder {
    fn add_forward_flat_pipeline(&mut self) -> &mut Self;
}

impl<'a> ForwardFlatPipelineBuilder for RenderGraphBuilder<'a> {
    fn add_forward_flat_pipeline(&mut self) -> &mut Self {
        self.add_pipeline(resource_name::pipeline::FORWARD_FLAT, |builder| {
            builder
                .with_vertex_shader(Shader::from_glsl(
                    ShaderStage::Vertex,
                    include_str!("forward_flat.vert"),
                ))
                .with_fragment_shader(Shader::from_glsl(
                    ShaderStage::Fragment,
                    include_str!("forward_flat.frag"),
                ))
                .with_rasterization_state(RasterizationStateDescriptor {
                    front_face: FrontFace::Ccw,
                    cull_mode: CullMode::Back,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                })
                .with_depth_stencil_state(DepthStencilStateDescriptor {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Less,
                    stencil_front: StencilStateFaceDescriptor::IGNORE,
                    stencil_back: StencilStateFaceDescriptor::IGNORE,
                    stencil_read_mask: 0,
                    stencil_write_mask: 0,
                })
                .add_color_state(ColorStateDescriptor {
                    format: TextureFormat::Bgra8UnormSrgb,
                    color_blend: BlendDescriptor::REPLACE,
                    alpha_blend: BlendDescriptor::REPLACE,
                    write_mask: ColorWrite::ALL,
                })
                .add_draw_target(resource_name::draw_target::MESHES);
        })
    }
}
