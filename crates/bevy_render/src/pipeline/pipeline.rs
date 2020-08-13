use super::{
    state_descriptors::{
        BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
        CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, IndexFormat,
        PrimitiveTopology, RasterizationStateDescriptor, StencilStateFaceDescriptor,
    },
    BindType, DynamicBinding, PipelineLayout, VertexBufferDescriptors, StencilStateDescriptor,
};
use crate::{
    shader::{Shader, ShaderStages},
    texture::TextureFormat,
};
use bevy_asset::Assets;

#[derive(Clone, Debug)]
pub struct PipelineDescriptor {
    pub name: Option<String>,
    pub layout: Option<PipelineLayout>,
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
            layout: None,
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
            layout: None,
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
            shader_stages,
        }
    }

    pub fn get_layout(&self) -> Option<&PipelineLayout> {
        self.layout.as_ref()
    }

    pub fn get_layout_mut(&mut self) -> Option<&mut PipelineLayout> {
        self.layout.as_mut()
    }

    /// Reflects the pipeline layout from its shaders.
    ///
    /// If `bevy_conventions` is true, it will be assumed that the shader follows "bevy shader conventions". These allow
    /// richer reflection, such as inferred Vertex Buffer names and inferred instancing.
    ///
    /// If `dynamic_bindings` has values, shader uniforms will be set to "dynamic" if there is a matching binding in the list
    ///
    /// If `vertex_buffer_descriptors` is set, the pipeline's vertex buffers
    /// will inherit their layouts from global descriptors, otherwise the layout will be assumed to be complete / local.
    pub fn reflect_layout(
        &mut self,
        shaders: &Assets<Shader>,
        bevy_conventions: bool,
        vertex_buffer_descriptors: Option<&VertexBufferDescriptors>,
        dynamic_bindings: &[DynamicBinding],
    ) {
        let vertex_spirv = shaders.get(&self.shader_stages.vertex).unwrap();
        let fragment_spirv = self
            .shader_stages
            .fragment
            .as_ref()
            .map(|handle| shaders.get(&handle).unwrap());

        let mut layouts = vec![vertex_spirv.reflect_layout(bevy_conventions).unwrap()];
        if let Some(ref fragment_spirv) = fragment_spirv {
            layouts.push(fragment_spirv.reflect_layout(bevy_conventions).unwrap());
        }

        let mut layout = PipelineLayout::from_shader_layouts(&mut layouts);
        if let Some(vertex_buffer_descriptors) = vertex_buffer_descriptors {
            layout.sync_vertex_buffer_descriptors(vertex_buffer_descriptors);
        }

        if !dynamic_bindings.is_empty() {
            // set binding uniforms to dynamic if render resource bindings use dynamic
            for bind_group in layout.bind_groups.iter_mut() {
                for binding in bind_group.bindings.iter_mut() {
                    let current = DynamicBinding {
                        bind_group: bind_group.index,
                        binding: binding.index,
                    };

                    if dynamic_bindings.contains(&current) {
                        if let BindType::Uniform {
                            ref mut dynamic, ..
                        } = binding.bind_type
                        {
                            *dynamic = true;
                        }
                    }
                }
            }
        }

        self.layout = Some(layout);
    }
}
