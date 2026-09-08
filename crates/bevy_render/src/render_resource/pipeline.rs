use crate::renderer::{impl_eq_ord_hash_wrapper, wgpu_wrapper};
use core::ops::Deref;

wgpu_wrapper! {
    #[derive(Clone, Debug)]
    struct WgpuRenderPipeline(wgpu::RenderPipeline);
}

impl_eq_ord_hash_wrapper!(WgpuRenderPipeline);

/// An opaque identifier for a [`RenderPipeline`], backed by the wrapped wgpu [`RenderPipeline`](wgpu::RenderPipeline).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderPipelineId(WgpuRenderPipeline);

/// A [`RenderPipeline`] represents a graphics pipeline and its stages (shaders), bindings and vertex buffers.
///
/// May be converted from and dereferences to a wgpu [`RenderPipeline`](wgpu::RenderPipeline).
/// Can be created via [`RenderDevice::create_render_pipeline`](crate::renderer::RenderDevice::create_render_pipeline).
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    value: WgpuRenderPipeline,
}

impl RenderPipeline {
    #[inline]
    pub fn id(&self) -> RenderPipelineId {
        RenderPipelineId(self.value.clone())
    }
}

impl From<wgpu::RenderPipeline> for RenderPipeline {
    fn from(value: wgpu::RenderPipeline) -> Self {
        RenderPipeline {
            value: WgpuRenderPipeline::new(value),
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

wgpu_wrapper! {
    #[derive(Clone, Debug)]
    struct WgpuComputePipeline(wgpu::ComputePipeline);
}

impl_eq_ord_hash_wrapper!(WgpuComputePipeline);

/// An opaque identifier for a [`ComputePipeline`], backed by the wrapped wgpu [`ComputePipeline`](wgpu::ComputePipeline).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComputePipelineId(WgpuComputePipeline);

/// A [`ComputePipeline`] represents a compute pipeline and its single shader stage.
///
/// May be converted from and dereferences to a wgpu [`ComputePipeline`](wgpu::ComputePipeline).
/// Can be created via [`RenderDevice::create_compute_pipeline`](crate::renderer::RenderDevice::create_compute_pipeline).
#[derive(Clone, Debug)]
pub struct ComputePipeline {
    value: WgpuComputePipeline,
}

impl ComputePipeline {
    /// Returns the [`ComputePipelineId`].
    #[inline]
    pub fn id(&self) -> ComputePipelineId {
        ComputePipelineId(self.value.clone())
    }
}

impl From<wgpu::ComputePipeline> for ComputePipeline {
    fn from(value: wgpu::ComputePipeline) -> Self {
        ComputePipeline {
            value: WgpuComputePipeline::new(value),
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
