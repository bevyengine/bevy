use crate::{
    legion::prelude::World,
    render::shader::{Shader, ShaderStages},
};

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc
// Mesh target
// trait DrawTarget {
//     fn draw(device: &wgpu::Device);
// }
type DrawTarget = fn(world: &World, device: &wgpu::Device);

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
    pub draw_targets: Vec<DrawTarget>,
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
    fn new(vertex_shader: Shader) -> Self {
        PipelineDefinition {
            color_states: Vec::new(),
            depth_stencil_state: None,
            draw_targets: Vec::new(),
            shader_stages: ShaderStages::new(vertex_shader),
            vertex_buffer_definitions: Vec::new(),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            index_format: wgpu::IndexFormat::Uint16,
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }
}

impl PipelineDefinition {
    pub fn create_render_pipeline(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        let vertex_shader_module = self.shader_stages.vertex.create_shader_module(device);
        let fragment_shader_module = match self.shader_stages.fragment {
            Some(ref fragment_shader) => Some(fragment_shader.create_shader_module(device)),
            None => None,
        };

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
            vertex_buffers: &self
                .vertex_buffer_definitions
                .iter()
                .map(|v| v.into())
                .collect::<Vec<wgpu::VertexBufferDescriptor>>(),
            sample_count: self.sample_count,
            sample_mask: self.sample_mask,
            alpha_to_coverage_enabled: self.alpha_to_coverage_enabled,
        };

        device.create_render_pipeline(&render_pipeline_descriptor)
    }

    pub fn build(vertex_shader: Shader) -> PipelineBuilder {
        PipelineBuilder::new(vertex_shader)
    }
}

pub struct PipelineBuilder {
    pipeline: PipelineDefinition,
}

impl PipelineBuilder {
    pub fn new(vertex_shader: Shader) -> Self {
        PipelineBuilder {
            pipeline: PipelineDefinition::new(vertex_shader),
        }
    }

    pub fn with_fragment_shader(mut self, fragment_shader: Shader) -> Self {
        self.pipeline.shader_stages.fragment = Some(fragment_shader);
        self
    }

    pub fn with_color_state(mut self, color_state_descriptor: wgpu::ColorStateDescriptor) -> Self {
        self.pipeline.color_states.push(color_state_descriptor);
        self
    }

    pub fn with_depth_stencil_state(mut self, depth_stencil_state: wgpu::DepthStencilStateDescriptor) -> Self {
        if let Some(_) = self.pipeline.depth_stencil_state {
            panic!("Depth stencil state has already been set");
        }
        self.pipeline.depth_stencil_state = Some(depth_stencil_state);
        self
    }

    pub fn with_vertex_buffer_definition(mut self, vertex_buffer_definition: VertexBufferDefinition) -> Self {
        self.pipeline.vertex_buffer_definitions.push(vertex_buffer_definition);
        self
    }

    pub fn with_index_format(mut self, index_format: wgpu::IndexFormat) -> Self {
        self.pipeline.index_format = index_format;
        self
    }

    pub fn with_draw_target(mut self, draw_target: DrawTarget) -> Self {
        self.pipeline.draw_targets.push(draw_target);
        self
    }

    pub fn with_rasterization_state(mut self, rasterization_state: wgpu::RasterizationStateDescriptor) -> Self {
        self.pipeline.rasterization_state = Some(rasterization_state);
        self
    }

    pub fn with_primitive_topology(mut self, primitive_topology: wgpu::PrimitiveTopology) -> Self {
        self.pipeline.primitive_topology = primitive_topology;
        self
    }

    pub fn with_sample_count(mut self, sample_count: u32) -> Self {
        self.pipeline.sample_count = sample_count;
        self
    }

    pub fn with_alpha_to_coverage_enabled(mut self, alpha_to_coverage_enabled: bool) -> Self {
        self.pipeline.alpha_to_coverage_enabled = alpha_to_coverage_enabled;
        self
    }

    pub fn with_sample_mask(mut self, sample_mask: u32) -> Self {
        self.pipeline.sample_mask = sample_mask;
        self
    }
}
