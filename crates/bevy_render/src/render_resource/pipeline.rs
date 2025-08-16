use super::empty_bind_group_layout;
use crate::WgpuWrapper;
use crate::{define_atomic_id, render_resource::BindGroupLayout};
use alloc::borrow::Cow;
use bevy_asset::Handle;
use bevy_mesh::VertexBufferLayout;
use bevy_shader::{Shader, ShaderDefVal};
use core::iter;
use core::ops::Deref;
use thiserror::Error;
use wgpu::{
    ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState, PushConstantRange,
};

define_atomic_id!(RenderPipelineId);

/// A [`RenderPipeline`] represents a graphics pipeline and its stages (shaders), bindings and vertex buffers.
///
/// May be converted from and dereferences to a wgpu [`RenderPipeline`](wgpu::RenderPipeline).
/// Can be created via [`RenderDevice::create_render_pipeline`](crate::renderer::RenderDevice::create_render_pipeline).
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    id: RenderPipelineId,
    value: WgpuWrapper<wgpu::RenderPipeline>,
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
            id: RenderPipelineId::new(),
            value: WgpuWrapper::new(value),
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

define_atomic_id!(ComputePipelineId);

/// A [`ComputePipeline`] represents a compute pipeline and its single shader stage.
///
/// May be converted from and dereferences to a wgpu [`ComputePipeline`](wgpu::ComputePipeline).
/// Can be created via [`RenderDevice::create_compute_pipeline`](crate::renderer::RenderDevice::create_compute_pipeline).
#[derive(Clone, Debug)]
pub struct ComputePipeline {
    id: ComputePipelineId,
    value: WgpuWrapper<wgpu::ComputePipeline>,
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
            id: ComputePipelineId::new(),
            value: WgpuWrapper::new(value),
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
#[derive(Clone, Debug, PartialEq, Default)]
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
    /// Whether to zero-initialize workgroup memory by default. If you're not sure, set this to true.
    /// If this is false, reading from workgroup variables before writing to them will result in garbage values.
    pub zero_initialize_workgroup_memory: bool,
}

#[derive(Copy, Clone, Debug, Error)]
#[error("RenderPipelineDescriptor has no FragmentState configured")]
pub struct NoFragmentStateError;

impl RenderPipelineDescriptor {
    pub fn fragment_mut(&mut self) -> Result<&mut FragmentState, NoFragmentStateError> {
        self.fragment.as_mut().ok_or(NoFragmentStateError)
    }

    pub fn set_layout(&mut self, index: usize, layout: BindGroupLayout) {
        filling_set_at(&mut self.layout, index, empty_bind_group_layout(), layout);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct VertexState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader, or `None` if the default entry point
    /// is used.
    pub entry_point: Option<Cow<'static, str>>,
    /// The format of any vertex buffers used with this pipeline.
    pub buffers: Vec<VertexBufferLayout>,
}

/// Describes the fragment process in a render pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FragmentState {
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader, or `None` if the default entry point
    /// is used.
    pub entry_point: Option<Cow<'static, str>>,
    /// The color state of the render targets.
    pub targets: Vec<Option<ColorTargetState>>,
}

impl FragmentState {
    pub fn set_target(&mut self, index: usize, target: ColorTargetState) {
        filling_set_at(&mut self.targets, index, None, Some(target));
    }
}

/// Describes a compute pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ComputePipelineDescriptor {
    pub label: Option<Cow<'static, str>>,
    pub layout: Vec<BindGroupLayout>,
    pub push_constant_ranges: Vec<PushConstantRange>,
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader, or `None` if the default entry point
    /// is used.
    pub entry_point: Option<Cow<'static, str>>,
    /// Whether to zero-initialize workgroup memory by default. If you're not sure, set this to true.
    /// If this is false, reading from workgroup variables before writing to them will result in garbage values.
    pub zero_initialize_workgroup_memory: bool,
}

// utility function to set a value at the specified index, extending with
// a filler value if the index is out of bounds.
fn filling_set_at<T: Clone>(vec: &mut Vec<T>, index: usize, filler: T, value: T) {
    let num_to_fill = (index + 1).saturating_sub(vec.len());
    vec.extend(iter::repeat_n(filler, num_to_fill));
    vec[index] = value;
}
