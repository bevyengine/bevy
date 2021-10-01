use crate::render_resource::{next_id, Counter, Id};
use std::{ops::Deref, sync::Arc};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderPipelineId(Id);

impl RenderPipelineId {
    /// Creates a new, unique [`RenderPipelineId`].
    /// Returns [`None`] if the supply of unique ids has been exhausted.
    fn new() -> Option<Self> {
        static COUNTER: Counter = Counter::new(0);
        next_id(&COUNTER).map(Self)
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
            id: RenderPipelineId::new().expect("The system ran out of unique `RenderPipelineId`s."),
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

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ComputePipelineId(Id);

impl ComputePipelineId {
    /// Creates a new, unique [`ComputePipelineId`].
    /// Returns [`None`] if the supply of unique ids has been exhausted.
    #[allow(clippy::new_without_default)]
    fn new() -> Option<Self> {
        static COUNTER: Counter = Counter::new(0);
        next_id(&COUNTER).map(Self)
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
            id: ComputePipelineId::new()
                .expect("The system ran out of unique `ComputePipelineId`s."),
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
