use crate::{asset::{AssetStorage, Handle}, render::{
    render_graph_2::{BindGroup, DrawTarget, PipelineLayout},
    shader::{Shader, ShaderStages},
}};

#[derive(Clone, Debug)]
pub struct VertexBufferDescriptor {
    pub stride: wgpu::BufferAddress,
    pub step_mode: wgpu::InputStepMode,
    pub attributes: Vec<wgpu::VertexAttributeDescriptor>,
}

impl<'a> Into<wgpu::VertexBufferDescriptor<'a>> for &'a VertexBufferDescriptor {
    fn into(self) -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            step_mode: self.step_mode,
            stride: self.stride,
            attributes: &self.attributes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PipelineDescriptor {
    pub draw_targets: Vec<String>,
    pub pipeline_layout: PipelineLayout,
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
    pub vertex_buffer_descriptors: Vec<VertexBufferDescriptor>,

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

impl PipelineDescriptor {
    fn new(vertex_shader: Handle<Shader>) -> Self {
        PipelineDescriptor {
            pipeline_layout: PipelineLayout::new(),
            color_states: Vec::new(),
            depth_stencil_state: None,
            draw_targets: Vec::new(),
            shader_stages: ShaderStages::new(vertex_shader),
            vertex_buffer_descriptors: Vec::new(),
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

impl PipelineDescriptor {
    pub fn build(shader_storage: &mut AssetStorage<Shader>, vertex_shader: Shader) -> PipelineBuilder {
        PipelineBuilder::new(shader_storage, vertex_shader)
    }
}

pub struct PipelineBuilder<'a> {
    pipeline: PipelineDescriptor,
    shader_storage: &'a mut AssetStorage<Shader>,
}

impl<'a> PipelineBuilder<'a> {
    pub fn new(shader_storage: &'a mut AssetStorage<Shader>, vertex_shader: Shader) -> Self {
        let vertex_shader_handle = shader_storage.add(vertex_shader);
        PipelineBuilder {
            pipeline: PipelineDescriptor::new(vertex_shader_handle),
            shader_storage,
        }
    }

    pub fn build(self) -> PipelineDescriptor {
        self.pipeline
    }

    pub fn with_fragment_shader(mut self, fragment_shader: Shader) -> Self {
        let fragment_shader_handle = self.shader_storage.add(fragment_shader);
        self.pipeline.shader_stages.fragment = Some(fragment_shader_handle);
        self
    }

    pub fn add_color_state(mut self, color_state_descriptor: wgpu::ColorStateDescriptor) -> Self {
        self.pipeline.color_states.push(color_state_descriptor);
        self
    }

    pub fn with_depth_stencil_state(
        mut self,
        depth_stencil_state: wgpu::DepthStencilStateDescriptor,
    ) -> Self {
        if let Some(_) = self.pipeline.depth_stencil_state {
            panic!("Depth stencil state has already been set");
        }
        self.pipeline.depth_stencil_state = Some(depth_stencil_state);
        self
    }

    pub fn add_bind_group(mut self, bind_group: BindGroup) -> Self {
        self.pipeline.pipeline_layout.bind_groups.push(bind_group);
        self
    }

    pub fn add_vertex_buffer_descriptor(
        mut self,
        vertex_buffer_descriptor: VertexBufferDescriptor,
    ) -> Self {
        self.pipeline
            .vertex_buffer_descriptors
            .push(vertex_buffer_descriptor);
        self
    }

    pub fn with_index_format(mut self, index_format: wgpu::IndexFormat) -> Self {
        self.pipeline.index_format = index_format;
        self
    }

    pub fn add_draw_target(mut self, name: &str) -> Self {
        self.pipeline.draw_targets.push(name.to_string());
        self
    }

    pub fn with_rasterization_state(
        mut self,
        rasterization_state: wgpu::RasterizationStateDescriptor,
    ) -> Self {
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
