use crate::render::shader::ShaderStages;

pub struct VertexBufferDefinition {
    pub stride: wgpu::BufferAddress,
    pub step_mode: wgpu::InputStepMode,
    pub attributes: Vec<wgpu::VertexAttributeDescriptor>,
}

impl<'a> Into<wgpu::VertexBufferDescriptor<'a>> for &'a VertexBufferDefinition {
    fn into(self) -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            step_mode: self.step_mode,
            stride: self.stride,
            attributes: &self.attributes,
        }
    }
}

pub struct PipelineDefinition {
    pub shader_stages: ShaderStages,
    pub rasterization_state: Option<wgpu::RasterizationStateDescriptor>,

    /// The primitive topology used to interpret vertices.
    pub primitive_topology: wgpu::PrimitiveTopology,

    /// The effect of draw calls on the color aspect of the output target.
    pub color_states: Vec<wgpu::ColorStateDescriptor>,

    /// The effect of draw calls on the depth and stencil aspects of the output target, if any.
    pub depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>,

    /// The format of any index buffers used with this pipeline.
    pub index_format: wgpu::IndexFormat,

    /// The format of any vertex buffers used with this pipeline.
    pub vertex_buffer_definitions: Vec<VertexBufferDefinition>,

    /// The number of samples calculated per pixel (for MSAA).
    pub sample_count: u32,

    /// Bitmask that restricts the samples of a pixel modified by this pipeline.
    pub sample_mask: u32,

    /// When enabled, produces another sample mask per pixel based on the alpha output value, that
    /// is ANDed with the sample_mask and the primitive coverage to restrict the set of samples
    /// affected by a primitive.
    /// The implicit mask produced for alpha of zero is guaranteed to be zero, and for alpha of one
    /// is guaranteed to be all 1-s.
    pub alpha_to_coverage_enabled: bool,
}

impl PipelineDefinition {
    pub fn create_render_pipeline(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        let vertex_shader_module = self.shader_stages.vertex.create_shader_module(device);
        let fragment_shader_module = match self.shader_stages.fragment {
            Some(ref fragment_shader) => Some(fragment_shader.create_shader_module(device)),
            None => None,
        };

        let pipeline_layout =
                device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[],
                });
        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vertex_shader_module,
                entry_point: &self.shader_stages.vertex.entry_point,
            },
            fragment_stage: match self.shader_stages.fragment {
                Some(ref fragment_shader) => Some(wgpu::ProgrammableStageDescriptor {
                    entry_point: &fragment_shader.entry_point,
                    module: fragment_shader_module.as_ref().unwrap(),
                }),
                None => None,
            },
            rasterization_state: self.rasterization_state.clone(),
            primitive_topology: self.primitive_topology,
            color_states: &self.color_states,
            depth_stencil_state: self.depth_stencil_state.clone(),
            index_format: self.index_format,
            vertex_buffers: &self.vertex_buffer_definitions.iter().map(|v| v.into()).collect::<Vec<wgpu::VertexBufferDescriptor>>(),
            sample_count: self.sample_count,
            sample_mask: self.sample_mask,
            alpha_to_coverage_enabled: self.alpha_to_coverage_enabled,
        };

        device.create_render_pipeline(&render_pipeline_descriptor)
    }
}
