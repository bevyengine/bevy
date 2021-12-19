use crate::render_resource::{CachedPipelineId, RenderPipelineCache, RenderPipelineDescriptor};
use bevy_utils::HashMap;
use std::hash::Hash;

pub struct SpecializedPipelines<S: SpecializedPipeline> {
    cache: HashMap<S::Key, CachedPipelineId>,
}

impl<S: SpecializedPipeline> Default for SpecializedPipelines<S> {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}

impl<S: SpecializedPipeline> SpecializedPipelines<S> {
    pub fn specialize(
        &mut self,
        cache: &mut RenderPipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
    ) -> CachedPipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key);
            cache.queue(descriptor)
        })
    }
}

pub trait SpecializedPipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor;
}
