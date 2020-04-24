use super::{
    state_descriptors::{
        BlendDescriptor, ColorStateDescriptor, ColorWrite, CompareFunction, CullMode,
        DepthStencilStateDescriptor, FrontFace, IndexFormat, PrimitiveTopology,
        RasterizationStateDescriptor, StencilStateFaceDescriptor,
    },
    BindGroupDescriptor, PipelineLayout, VertexBufferDescriptor,
};
use crate::{
    render_resource::resource_name,
    shader::{Shader, ShaderStages},
    texture::TextureFormat,
};

use bevy_asset::{AssetStorage, Handle};

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
    pub draw_targets: Vec<String>,
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
    pub fn new_new(shader_stages: ShaderStages) -> Self {
        PipelineDescriptor {
            name: None,
            layout: PipelineLayoutType::Reflected(None),
            color_states: Vec::new(),
            depth_stencil_state: None,
            draw_targets: Vec::new(),
            shader_stages,
            rasterization_state: None,
            primitive_topology: PrimitiveTopology::TriangleList,
            index_format: IndexFormat::Uint16,
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    fn new(name: Option<&str>, vertex_shader: Handle<Shader>) -> Self {
        PipelineDescriptor {
            name: name.map(|name| name.to_string()),
            layout: PipelineLayoutType::Reflected(None),
            color_states: Vec::new(),
            depth_stencil_state: None,
            draw_targets: Vec::new(),
            shader_stages: ShaderStages::new(vertex_shader),
            rasterization_state: Some(RasterizationStateDescriptor {
                front_face: FrontFace::Ccw,
                cull_mode: CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: PrimitiveTopology::TriangleList,
            index_format: IndexFormat::Uint16,
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
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

impl PipelineDescriptor {
    pub fn build<'a>(
        name: &'a str,
        shader_storage: &'a mut AssetStorage<Shader>,
    ) -> PipelineBuilder<'a> {
        PipelineBuilder::new(name, shader_storage)
    }
}

pub struct PipelineBuilder<'a> {
    pipeline: Option<PipelineDescriptor>,
    shader_storage: &'a mut AssetStorage<Shader>,
    name: &'a str,
}

impl<'a> PipelineBuilder<'a> {
    pub fn new(name: &'a str, shader_storage: &'a mut AssetStorage<Shader>) -> Self {
        PipelineBuilder {
            pipeline: None,
            shader_storage,
            name,
        }
    }

    pub fn finish(&mut self) -> PipelineDescriptor {
        self.pipeline.take().unwrap()
    }

    pub fn with_vertex_shader(&mut self, vertex_shader: Shader) -> &mut Self {
        let vertex_shader_handle = self.shader_storage.add(vertex_shader);
        self.pipeline = Some(PipelineDescriptor::new(
            Some(&self.name),
            vertex_shader_handle,
        ));
        self
    }

    pub fn with_fragment_shader(&mut self, fragment_shader: Shader) -> &mut Self {
        let fragment_shader_handle = self.shader_storage.add(fragment_shader);
        self.pipeline.as_mut().unwrap().shader_stages.fragment = Some(fragment_shader_handle);
        self
    }

    pub fn add_color_state(&mut self, color_state_descriptor: ColorStateDescriptor) -> &mut Self {
        self.pipeline
            .as_mut()
            .unwrap()
            .color_states
            .push(color_state_descriptor);
        self
    }

    pub fn with_depth_stencil_state(
        &mut self,
        depth_stencil_state: DepthStencilStateDescriptor,
    ) -> &mut Self {
        if let Some(_) = self.pipeline.as_ref().unwrap().depth_stencil_state {
            panic!("Depth stencil state has already been set");
        }
        self.pipeline.as_mut().unwrap().depth_stencil_state = Some(depth_stencil_state);
        self
    }

    pub fn add_bind_group(&mut self, bind_group: BindGroupDescriptor) -> &mut Self {
        let pipeline = self.pipeline.as_mut().unwrap();
        if let PipelineLayoutType::Reflected(_) = pipeline.layout {
            pipeline.layout = PipelineLayoutType::Manual(PipelineLayout::default());
        }

        if let PipelineLayoutType::Manual(ref mut layout) = pipeline.layout {
            layout.bind_groups.push(bind_group);
        }

        self
    }

    pub fn add_vertex_buffer_descriptor(
        &mut self,
        vertex_buffer_descriptor: VertexBufferDescriptor,
    ) -> &mut Self {
        let pipeline = self.pipeline.as_mut().unwrap();
        if let PipelineLayoutType::Reflected(_) = pipeline.layout {
            pipeline.layout = PipelineLayoutType::Manual(PipelineLayout::default());
        }

        if let PipelineLayoutType::Manual(ref mut layout) = pipeline.layout {
            layout
                .vertex_buffer_descriptors
                .push(vertex_buffer_descriptor);
        }

        self
    }

    pub fn with_index_format(&mut self, index_format: IndexFormat) -> &mut Self {
        self.pipeline.as_mut().unwrap().index_format = index_format;
        self
    }

    pub fn add_draw_target(&mut self, name: &str) -> &mut Self {
        self.pipeline
            .as_mut()
            .unwrap()
            .draw_targets
            .push(name.to_string());
        self
    }

    pub fn with_rasterization_state(
        &mut self,
        rasterization_state: RasterizationStateDescriptor,
    ) -> &mut Self {
        self.pipeline.as_mut().unwrap().rasterization_state = Some(rasterization_state);
        self
    }

    pub fn with_primitive_topology(&mut self, primitive_topology: PrimitiveTopology) -> &mut Self {
        self.pipeline.as_mut().unwrap().primitive_topology = primitive_topology;
        self
    }

    pub fn with_sample_count(&mut self, sample_count: u32) -> &mut Self {
        self.pipeline.as_mut().unwrap().sample_count = sample_count;
        self
    }

    pub fn with_alpha_to_coverage_enabled(&mut self, alpha_to_coverage_enabled: bool) -> &mut Self {
        self.pipeline.as_mut().unwrap().alpha_to_coverage_enabled = alpha_to_coverage_enabled;
        self
    }

    pub fn with_sample_mask(&mut self, sample_mask: u32) -> &mut Self {
        self.pipeline.as_mut().unwrap().sample_mask = sample_mask;
        self
    }

    pub fn with_default_config(&mut self) -> &mut Self {
        self.with_depth_stencil_state(DepthStencilStateDescriptor {
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
        .add_draw_target(resource_name::draw_target::ASSIGNED_MESHES)
    }
}
