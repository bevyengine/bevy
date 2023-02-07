use crate::{render_resource::*, render_resource_wrapper, renderer::RenderDevice};
use bevy_asset::Handle;
use std::{borrow::Cow, fmt::Debug, hash::Hash, num::NonZeroU32, ops::Deref};

pub(crate) trait PipelineId: Copy + Clone + Hash + Eq + PartialEq + Debug {
    fn new(id: u32) -> Self;
    fn index(&self) -> usize;
}

pub(crate) trait Pipeline<I, D, P> {
    fn process_pipeline(
        id: I,
        descriptor: &D,
        device: &RenderDevice,
        shader_cache: &mut ShaderCache,
        layout_cache: &mut LayoutCache,
    ) -> PipelineState<P>;
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderPipelineId(pub(crate) NonZeroU32);

impl PipelineId for RenderPipelineId {
    fn new(id: u32) -> Self {
        // offset the id by one, for the non-zero optimisation
        Self(NonZeroU32::new(id + 1).unwrap_or_else(|| {
            panic!(
                "The system ran out of unique `{}`s.",
                stringify!(RenderPipelineId)
            );
        }))
    }

    #[inline]
    fn index(&self) -> usize {
        // offset the id by one, for the non-zero optimisation
        self.0.get() as usize - 1
    }
}

render_resource_wrapper!(ErasedRenderPipeline, wgpu::RenderPipeline);

/// A [`RenderPipeline`] represents a graphics pipeline and its stages (shaders), bindings and vertex buffers.
///
/// May be converted from and dereferences to a wgpu [`RenderPipeline`](wgpu::RenderPipeline).
/// Can be created via the [`PipelineCache`](crate::render_resource::PipelineCache).
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    id: RenderPipelineId,
    value: ErasedRenderPipeline,
}

impl RenderPipeline {
    #[inline]
    pub(crate) fn new(id: RenderPipelineId, value: wgpu::RenderPipeline) -> Self {
        Self {
            id,
            value: ErasedRenderPipeline::new(value),
        }
    }

    #[inline]
    pub fn id(&self) -> RenderPipelineId {
        self.id
    }
}

impl Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Pipeline<RenderPipelineId, RenderPipelineDescriptor, RenderPipeline> for RenderPipeline {
    fn process_pipeline(
        id: RenderPipelineId,
        descriptor: &RenderPipelineDescriptor,
        device: &RenderDevice,
        shader_cache: &mut ShaderCache,
        layout_cache: &mut LayoutCache,
    ) -> PipelineState<RenderPipeline> {
        let vertex_module = match shader_cache.get(
            device,
            id.into(),
            &descriptor.vertex.shader,
            &descriptor.vertex.shader_defs,
        ) {
            Ok(module) => module,
            Err(err) => {
                return PipelineState::Err(err);
            }
        };

        let fragment_data = if let Some(fragment) = &descriptor.fragment {
            let fragment_module = match shader_cache.get(
                device,
                id.into(),
                &fragment.shader,
                &fragment.shader_defs,
            ) {
                Ok(module) => module,
                Err(err) => {
                    return PipelineState::Err(err);
                }
            };
            Some((
                fragment_module,
                fragment.entry_point.deref(),
                fragment.targets.as_slice(),
            ))
        } else {
            None
        };

        let vertex_buffer_layouts = descriptor
            .vertex
            .buffers
            .iter()
            .map(|layout| RawVertexBufferLayout {
                array_stride: layout.array_stride,
                attributes: &layout.attributes,
                step_mode: layout.step_mode,
            })
            .collect::<Vec<_>>();

        let layout = if descriptor.layout.is_empty() && descriptor.push_constant_ranges.is_empty() {
            None
        } else {
            Some(layout_cache.get(
                device,
                &descriptor.layout,
                descriptor.push_constant_ranges.to_vec(),
            ))
        };

        let descriptor = RawRenderPipelineDescriptor {
            multiview: None,
            depth_stencil: descriptor.depth_stencil.clone(),
            label: descriptor.label.as_deref(),
            layout,
            multisample: descriptor.multisample,
            primitive: descriptor.primitive,
            vertex: RawVertexState {
                buffers: &vertex_buffer_layouts,
                entry_point: descriptor.vertex.entry_point.deref(),
                module: &vertex_module,
            },
            fragment: fragment_data
                .as_ref()
                .map(|(module, entry_point, targets)| RawFragmentState {
                    entry_point,
                    module,
                    targets,
                }),
        };

        let pipeline = device.create_render_pipeline(id, &descriptor);

        PipelineState::Ok(pipeline)
    }
}

render_resource_wrapper!(ErasedComputePipeline, wgpu::ComputePipeline);

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ComputePipelineId(pub(crate) NonZeroU32);

impl PipelineId for ComputePipelineId {
    fn new(id: u32) -> Self {
        Self(NonZeroU32::new(id + 1).unwrap_or_else(|| {
            // offset the id by one, for the non-zero optimisation
            panic!(
                "The system ran out of unique `{}`s.",
                stringify!(ComputePipelineId)
            );
        }))
    }

    #[inline]
    fn index(&self) -> usize {
        // offset the id by one, for the non-zero optimisation
        self.0.get() as usize - 1
    }
}

/// A [`ComputePipeline`] represents a compute pipeline and its single shader stage.
///
/// May be converted from and dereferences to a wgpu [`ComputePipeline`](wgpu::ComputePipeline).
/// Can be created via the [`PipelineCache`](crate::render_resource::PipelineCache).
#[derive(Clone, Debug)]
pub struct ComputePipeline {
    id: ComputePipelineId,
    value: ErasedComputePipeline,
}

impl ComputePipeline {
    #[inline]
    pub(crate) fn new(id: ComputePipelineId, value: wgpu::ComputePipeline) -> Self {
        Self {
            id,
            value: ErasedComputePipeline::new(value),
        }
    }

    /// Returns the [`ComputePipelineId`].
    #[inline]
    pub fn id(&self) -> ComputePipelineId {
        self.id
    }
}

impl Deref for ComputePipeline {
    type Target = wgpu::ComputePipeline;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Pipeline<ComputePipelineId, ComputePipelineDescriptor, ComputePipeline> for ComputePipeline {
    fn process_pipeline(
        id: ComputePipelineId,
        descriptor: &ComputePipelineDescriptor,
        device: &RenderDevice,
        shader_cache: &mut ShaderCache,
        layout_cache: &mut LayoutCache,
    ) -> PipelineState<ComputePipeline> {
        let compute_module = match shader_cache.get(
            device,
            id.into(),
            &descriptor.shader,
            &descriptor.shader_defs,
        ) {
            Ok(module) => module,
            Err(err) => {
                return PipelineState::Err(err);
            }
        };

        let layout = if descriptor.layout.is_empty() && descriptor.push_constant_ranges.is_empty() {
            None
        } else {
            Some(layout_cache.get(
                device,
                &descriptor.layout,
                descriptor.push_constant_ranges.to_vec(),
            ))
        };

        let descriptor = RawComputePipelineDescriptor {
            label: descriptor.label.as_deref(),
            layout,
            module: &compute_module,
            entry_point: descriptor.entry_point.as_ref(),
        };

        let pipeline = device.create_compute_pipeline(id, &descriptor);

        PipelineState::Ok(pipeline)
    }
}

/// Describes a render (graphics) pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderPipelineDescriptor {
    /// Debug label of the pipeline. This will show up in graphics debuggers for easy identification.
    pub label: Option<Cow<'static, str>>,
    /// The layout of bind groups for this pipeline.
    pub layout: Vec<BindGroupLayout>,
    /// The push constant ranges for this pipeline.
    /// Supply an empty vector if the pipeline doesn't use push constants.
    pub push_constant_ranges: Vec<PushConstantRange>,
    /// The compiled vertex stage, its entry point, and the input buffers layout.
    pub vertex: VertexState,
    /// The properties of the pipeline at the primitive assembly and rasterization level.
    pub primitive: PrimitiveState,
    /// The effect of draw calls on the depth and stencil aspects of the output target, if any.
    pub depth_stencil: Option<DepthStencilState>,
    /// The multi-sampling properties of the pipeline.
    pub multisample: MultisampleState,
    /// The compiled fragment stage, its entry point, and the color targets.
    pub fragment: Option<FragmentState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader. There must be a
    /// function with this name in the shader.
    pub entry_point: Cow<'static, str>,
    /// The format of any vertex buffers used with this pipeline.
    pub buffers: Vec<VertexBufferLayout>,
}

/// Describes how the vertex buffer is interpreted.
#[derive(Default, Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexBufferLayout {
    /// The stride, in bytes, between elements of this buffer.
    pub array_stride: BufferAddress,
    /// How often this vertex buffer is "stepped" forward.
    pub step_mode: VertexStepMode,
    /// The list of attributes which comprise a single vertex.
    pub attributes: Vec<VertexAttribute>,
}

impl VertexBufferLayout {
    /// Creates a new densely packed [`VertexBufferLayout`] from an iterator of vertex formats.
    /// Iteration order determines the `shader_location` and `offset` of the [`VertexAttributes`](VertexAttribute).
    /// The first iterated item will have a `shader_location` and `offset` of zero.
    /// The `array_stride` is the sum of the size of the iterated [`VertexFormats`](VertexFormat) (in bytes).
    pub fn from_vertex_formats<T: IntoIterator<Item = VertexFormat>>(
        step_mode: VertexStepMode,
        vertex_formats: T,
    ) -> Self {
        let mut offset = 0;
        let mut attributes = Vec::new();
        for (shader_location, format) in vertex_formats.into_iter().enumerate() {
            attributes.push(VertexAttribute {
                format,
                offset,
                shader_location: shader_location as u32,
            });
            offset += format.size();
        }

        VertexBufferLayout {
            array_stride: offset,
            step_mode,
            attributes,
        }
    }
}

/// Describes the fragment process in a render pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader. There must be a
    /// function with this name in the shader.
    pub entry_point: Cow<'static, str>,
    /// The color state of the render targets.
    pub targets: Vec<Option<ColorTargetState>>,
}

/// Describes a compute pipeline.
#[derive(Clone, Debug)]
pub struct ComputePipelineDescriptor {
    pub label: Option<Cow<'static, str>>,
    pub layout: Vec<BindGroupLayout>,
    pub push_constant_ranges: Vec<PushConstantRange>,
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader. There must be a
    /// function with this name in the shader.
    pub entry_point: Cow<'static, str>,
}
