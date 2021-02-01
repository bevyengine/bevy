use bevy_asset::{Assets, HandleUntyped};
use bevy_reflect::TypeUuid;
use bevy_render::{
    pipeline::{
        BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite, CompareFunction,
        DepthBiasState, DepthStencilState, PipelineDescriptor, StencilFaceState, StencilState,
    },
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat,
};

pub const FORWARD_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 13148362314012771389);

pub(crate) fn build_forward_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
            clamp_depth: false,
        }),
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::default(),
            color_blend: BlendState {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendState {
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
