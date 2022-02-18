use crate::render_resource::{
    CachedPipelineId, ComputePipelineDescriptor, PipelineCache, RenderPipelineDescriptor,
};
use bevy_utils::HashMap;
use std::hash::Hash;

pub struct SpecializedRenderPipelines<S: SpecializedRenderPipeline> {
    cache: HashMap<S::Key, CachedPipelineId>,
}

impl<S: SpecializedRenderPipeline> Default for SpecializedRenderPipelines<S> {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}

impl<S: SpecializedRenderPipeline> SpecializedRenderPipelines<S> {
    pub fn specialize(
        &mut self,
        cache: &mut PipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
    ) -> CachedPipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key);
            cache.queue_render_pipeline(descriptor)
        })
    }
}

pub trait SpecializedRenderPipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor;
}

pub struct SpecializedComputePipelines<S: SpecializedComputePipeline> {
    cache: HashMap<S::Key, CachedPipelineId>,
}

impl<S: SpecializedComputePipeline> Default for SpecializedComputePipelines<S> {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}

impl<S: SpecializedComputePipeline> SpecializedComputePipelines<S> {
    pub fn specialize(
        &mut self,
        cache: &mut PipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
    ) -> CachedPipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key);
            cache.queue_compute_pipeline(descriptor)
        })
    }
}

pub trait SpecializedComputePipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor;
}
