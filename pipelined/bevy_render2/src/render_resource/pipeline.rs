use std::sync::atomic::{AtomicUsize, Ordering};
use std::{ops::Deref, sync::Arc};

static MAX_RENDER_PIPELINE_ID: AtomicUsize = AtomicUsize::new(0);
static MAX_COMPUTE_PIPELINE_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderPipelineId(usize);

impl RenderPipelineId {
    /// Creates a new id by incrementing the atomic id counter.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(MAX_RENDER_PIPELINE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ComputePipelineId(usize);

impl ComputePipelineId {
    /// Creates a new id by incrementing the atomic id counter.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(MAX_COMPUTE_PIPELINE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

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
            id: RenderPipelineId::new(),
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

#[derive(Clone, Debug)]
pub struct ComputePipeline {
    id: ComputePipelineId,
    value: Arc<wgpu::ComputePipeline>,
}

impl ComputePipeline {
    #[inline]
    pub fn id(&self) -> ComputePipelineId {
        self.id
    }
}

impl From<wgpu::ComputePipeline> for ComputePipeline {
    fn from(value: wgpu::ComputePipeline) -> Self {
        ComputePipeline {
            id: ComputePipelineId::new(),
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
