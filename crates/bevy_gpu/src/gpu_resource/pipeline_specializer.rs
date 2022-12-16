use crate::gpu_resource::{
    CachedComputePipelineId, CachedRenderPipelineId, ComputePipelineDescriptor, PipelineCache,
    RenderPipelineDescriptor,
};
use bevy_ecs::system::Resource;
use bevy_utils::{default, HashMap};
use std::hash::Hash;

pub trait SpecializedRenderPipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor;
}

#[derive(Resource)]
pub struct SpecializedRenderPipelines<S: SpecializedRenderPipeline> {
    cache: HashMap<S::Key, CachedRenderPipelineId>,
}

impl<S: SpecializedRenderPipeline> Default for SpecializedRenderPipelines<S> {
    fn default() -> Self {
        Self { cache: default() }
    }
}

impl<S: SpecializedRenderPipeline> SpecializedRenderPipelines<S> {
    pub fn specialize(
        &mut self,
        cache: &mut PipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
    ) -> CachedRenderPipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key);
            cache.queue_render_pipeline(descriptor)
        })
    }
}

pub trait SpecializedComputePipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor;
}

#[derive(Resource)]
pub struct SpecializedComputePipelines<S: SpecializedComputePipeline> {
    cache: HashMap<S::Key, CachedComputePipelineId>,
}

impl<S: SpecializedComputePipeline> Default for SpecializedComputePipelines<S> {
    fn default() -> Self {
        Self { cache: default() }
    }
}

impl<S: SpecializedComputePipeline> SpecializedComputePipelines<S> {
    pub fn specialize(
        &mut self,
        cache: &mut PipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
    ) -> CachedComputePipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key);
            cache.queue_compute_pipeline(descriptor)
        })
    }
}
