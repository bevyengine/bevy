use super::{
    state_descriptors::{
        BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
        CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, IndexFormat,
        PrimitiveTopology, RasterizationStateDescriptor, StencilStateFaceDescriptor,
    },
    PipelineLayout,
};
use crate::{shader::ShaderStages, texture::TextureFormat};

// TODO: consider removing this in favor of Option<Layout>
#[derive(Clone, Debug)]
pub enum PipelineLayoutType {
    Manual(PipelineLayout),
    Reflected(Option<PipelineLayout>),
}

#[derive(Clone, Debug)]
pub enum DescriptorType<T> {
    Manual(T),
    Reflected(Option<T>),
}

#[derive(Clone, Debug)]
pub struct PipelineDescriptor {
    pub name: Option<String>,
    pub layout: PipelineLayoutType,
    pub shader_stages: ShaderStages,
    pub rasterization_state: Option<RasterizationStateDescriptor>,

    /// The primitive topology used to interpret vertices.
    pub primitive_topology: PrimitiveTopology,

    /// The effect of draw calls on the color aspect of the output target.
    pub color_states: Vec<ColorStateDescriptor>,

    /// The effect of draw calls on the depth and stencil aspects of the output target, if any.
    pub depth_stencil_state: Option<DepthStencilStateDescriptor>,

    /// The format of any index buffers used with this pipeline.
    pub index_format: IndexFormat,

    /// The number of samples calculated per pixel (for MSAA).
    pub sample_count: u32,

    /// Bitmask that restricts the samples of a pixel modified by this pipeline.
    pub sample_mask: u32,

    /// When enabled, produces another sample mask per pixel based on the alpha output value, that
    /// is AND-ed with the sample_mask and the primitive coverage to restrict the set of samples
    /// affected by a primitive.
    /// The implicit mask produced for alpha of zero is guaranteed to be zero, and for alpha of one
    /// is guaranteed to be all 1-s.
    pub alpha_to_coverage_enabled: bool,
}

impl PipelineDescriptor {
    pub fn new(shader_stages: ShaderStages) -> Self {
        PipelineDescriptor {
            name: None,
            layout: PipelineLayoutType::Reflected(None),
            color_states: Vec::new(),
            depth_stencil_state: None,
            shader_stages,
            rasterization_state: None,
            primitive_topology: PrimitiveTopology::TriangleList,
            index_format: IndexFormat::Uint16,
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    pub fn default_config(shader_stages: ShaderStages) -> Self {
        PipelineDescriptor {
            name: None,
            primitive_topology: PrimitiveTopology::TriangleList,
            layout: PipelineLayoutType::Reflected(None),
            index_format: IndexFormat::Uint16,
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
            rasterization_state: Some(RasterizationStateDescriptor {
                front_face: FrontFace::Ccw,
                cull_mode: CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            depth_stencil_state: Some(DepthStencilStateDescriptor {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil_front: StencilStateFaceDescriptor::IGNORE,
                stencil_back: StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
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
            shader_stages,
        }
    }

    pub fn get_layout(&self) -> Option<&PipelineLayout> {
        match self.layout {
            PipelineLayoutType::Reflected(ref layout) => layout.as_ref(),
            PipelineLayoutType::Manual(ref layout) => Some(layout),
        }
    }

    pub fn get_layout_mut(&mut self) -> Option<&mut PipelineLayout> {
        match self.layout {
            PipelineLayoutType::Reflected(ref mut layout) => layout.as_mut(),
            PipelineLayoutType::Manual(ref mut layout) => Some(layout),
        }
    }
}
