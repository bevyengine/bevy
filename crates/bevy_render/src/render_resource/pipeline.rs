use crate::renderer::WgpuWrapper;
use bevy_utils::define_atomic_id;
use core::ops::Deref;

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
