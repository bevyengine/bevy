use bevy_asset::{Assets, Handle};
use bevy_render::{
    pipeline::{
        BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
        CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, PipelineDescriptor,
        RasterizationStateDescriptor, StencilStateFaceDescriptor, StencilStateDescriptor,
    },
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat,
};

pub const FORWARD_PIPELINE_HANDLE: Handle<PipelineDescriptor> =
    Handle::from_u128(131483623140127713893804825450360211204);

pub(crate) fn build_forward_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilStateDescriptor {
                front: StencilStateFaceDescriptor::IGNORE,
                back: StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        color_states: vec![ColorStateDescriptor {
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
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("forward.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("forward.frag"),
            ))),
        })
    }
}
