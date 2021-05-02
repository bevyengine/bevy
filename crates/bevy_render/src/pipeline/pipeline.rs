use super::{
    state_descriptors::{
        BlendFactor, BlendOperation, ColorWrite, CompareFunction, Face, FrontFace,
        PrimitiveTopology,
    },
    PipelineLayout,
};
use crate::{
    pipeline::{
        BlendComponent, BlendState, ColorTargetState, DepthBiasState, DepthStencilState,
        MultisampleState, PolygonMode, PrimitiveState, StencilFaceState, StencilState,
    },
    shader::ShaderStages,
    texture::TextureFormat,
};
use bevy_reflect::TypeUuid;

#[derive(Clone, Debug, TypeUuid)]
#[uuid = "ebfc1d11-a2a4-44cb-8f12-c49cc631146c"]
pub struct PipelineDescriptor {
    pub name: Option<String>,
    pub layout: Option<PipelineLayout>,
    pub shader_stages: ShaderStages,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,

    /// The effect of draw calls on the color aspect of the output target.
    pub color_target_states: Vec<ColorTargetState>,
}

impl PipelineDescriptor {
    pub fn new(shader_stages: ShaderStages) -> Self {
        PipelineDescriptor {
            name: None,
            layout: None,
            color_target_states: Vec::new(),
            depth_stencil: None,
            shader_stages,
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }

    pub fn default_config(shader_stages: ShaderStages) -> Self {
        PipelineDescriptor {
            name: None,
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            layout: None,
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
            }),
            color_target_states: vec![ColorTargetState {
                format: TextureFormat::default(),
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                }),
                write_mask: ColorWrite::ALL,
            }],
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            shader_stages,
        }
    }

    pub fn get_layout(&self) -> Option<&PipelineLayout> {
        self.layout.as_ref()
    }

    pub fn get_layout_mut(&mut self) -> Option<&mut PipelineLayout> {
        self.layout.as_mut()
    }
}
