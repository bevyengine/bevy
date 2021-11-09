use crate::render_resource::{BindGroupLayout, Shader};
use bevy_asset::Handle;
use bevy_reflect::Uuid;
use std::{borrow::Cow, ops::Deref, sync::Arc};
use wgpu::{
    BufferAddress, ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState,
    VertexAttribute, VertexStepMode,
};

/// A [`RenderPipeline`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderPipelineId(Uuid);

/// A RenderPipeline represents a graphics pipeline and its stages (shaders), bindings and vertex buffers.
///
/// May be converted from and dereferences to a wgpu [`RenderPipeline`](wgpu::RenderPipeline).
/// Can be created via [`RenderDevice::create_render_pipeline`](crate::renderer::RenderDevice::create_render_pipeline).
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    id: RenderPipelineId,
    value: Arc<wgpu::RenderPipeline>,
}

impl RenderPipeline {
    #[inline]
    pub fn id(&self) -> RenderPipelineId {
        self.id
    }
}

impl From<wgpu::RenderPipeline> for RenderPipeline {
    fn from(value: wgpu::RenderPipeline) -> Self {
        RenderPipeline {
            id: RenderPipelineId(Uuid::new_v4()),
            value: Arc::new(value),
        }
    }
}

impl Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// A [`ComputePipeline`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ComputePipelineId(Uuid);

/// A ComputePipeline represents a compute pipeline and its single shader stage.
///
/// May be converted from and dereferences to a wgpu [`ComputePipeline`](wgpu::ComputePipeline).
/// Can be created via [`RenderDevice::create_compute_pipeline`](crate::renderer::RenderDevice::create_compute_pipeline).
#[derive(Clone, Debug)]
pub struct ComputePipeline {
    id: ComputePipelineId,
    value: Arc<wgpu::ComputePipeline>,
}

impl ComputePipeline {
    /// Returns the [`ComputePipelineId`].
    #[inline]
    pub fn id(&self) -> ComputePipelineId {
        self.id
    }
}

impl From<wgpu::ComputePipeline> for ComputePipeline {
    fn from(value: wgpu::ComputePipeline) -> Self {
        ComputePipeline {
            id: ComputePipelineId(Uuid::new_v4()),
            value: Arc::new(value),
        }
    }
}

impl Deref for ComputePipeline {
    type Target = wgpu::ComputePipeline;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Describes a render (graphics) pipeline.
#[derive(Clone, Debug)]
pub struct RenderPipelineDescriptor {
    /// Debug label of the pipeline. This will show up in graphics debuggers for easy identification.
    pub label: Option<Cow<'static, str>>,
    /// The layout of bind groups for this pipeline.
    pub layout: Option<Vec<BindGroupLayout>>,
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

#[derive(Clone, Debug)]
pub struct VertexState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<String>,
    /// The name of the entry point in the compiled shader. There must be a function that returns
    /// void with this name in the shader.
    pub entry_point: Cow<'static, str>,
    /// The format of any vertex buffers used with this pipeline.
    pub buffers: Vec<VertexBufferLayout>,
}

/// Describes how the vertex buffer is interpreted.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexBufferLayout {
    /// The stride, in bytes, between elements of this buffer.
    pub array_stride: BufferAddress,
    /// How often this vertex buffer is "stepped" forward.
    pub step_mode: VertexStepMode,
    /// The list of attributes which comprise a single vertex.
    pub attributes: Vec<VertexAttribute>,
}

/// Describes the fragment process in a render pipeline.
#[derive(Clone, Debug)]
pub struct FragmentState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<String>,
    /// The name of the entry point in the compiled shader. There must be a function that returns
    /// void with this name in the shader.
    pub entry_point: Cow<'static, str>,
    /// The color state of the render targets.
    pub targets: Vec<ColorTargetState>,
}
