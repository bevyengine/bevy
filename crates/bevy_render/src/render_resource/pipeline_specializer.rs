use crate::{
    mesh::{InnerMeshVertexBufferLayout, MeshVertexBufferLayout, MissingVertexAttributeError},
    render_resource::{
        CachedPipelineId, RenderPipelineCache, RenderPipelineDescriptor, VertexBufferLayout,
    },
};
use bevy_utils::{
    hashbrown::hash_map::RawEntryMut, tracing::error, Entry, HashMap, PreHashMap, PreHashMapExt,
};
use std::fmt::Debug;
use std::hash::Hash;
use thiserror::Error;

pub trait SpecializedPipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor;
}

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

#[derive(Error, Debug)]
pub enum SpecializedMeshPipelineError {
    #[error(transparent)]
    MissingVertexAttribute(#[from] MissingVertexAttributeError),
}

pub trait SpecializedMeshPipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError>;
}

pub struct SpecializedMeshPipelines<S: SpecializedMeshPipeline> {
    mesh_layout_cache: PreHashMap<InnerMeshVertexBufferLayout, HashMap<S::Key, CachedPipelineId>>,
    vertex_layout_cache: HashMap<VertexBufferLayout, HashMap<S::Key, CachedPipelineId>>,
}

impl<S: SpecializedMeshPipeline> Default for SpecializedMeshPipelines<S> {
    fn default() -> Self {
        Self {
            mesh_layout_cache: Default::default(),
            vertex_layout_cache: Default::default(),
        }
    }
}

impl<S: SpecializedMeshPipeline> SpecializedMeshPipelines<S> {
    #[inline]
    pub fn specialize(
        &mut self,
        cache: &mut RenderPipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<CachedPipelineId, SpecializedMeshPipelineError> {
        let map = self
            .mesh_layout_cache
            .get_or_insert_with(layout, Default::default);
        match map.entry(key.clone()) {
            Entry::Occupied(entry) => Ok(*entry.into_mut()),
            Entry::Vacant(entry) => {
                let descriptor = specialize_pipeline
                    .specialize(key.clone(), layout)
                    .map_err(|mut err| {
                        {
                            let SpecializedMeshPipelineError::MissingVertexAttribute(err) =
                                &mut err;
                            err.pipeline_type = Some(std::any::type_name::<S>());
                        }
                        err
                    })?;
                // Different MeshVertexBufferLayouts can produce the same final VertexBufferLayout
                // We want compatible vertex buffer layouts to use the same pipelines, so we must "deduplicate" them
                let layout_map = match self
                    .vertex_layout_cache
                    .raw_entry_mut()
                    .from_key(&descriptor.vertex.buffers[0])
                {
                    RawEntryMut::Occupied(entry) => entry.into_mut(),
                    RawEntryMut::Vacant(entry) => {
                        entry
                            .insert(descriptor.vertex.buffers[0].clone(), Default::default())
                            .1
                    }
                };
                Ok(*entry.insert(match layout_map.entry(key) {
                    Entry::Occupied(entry) => {
                        if cfg!(debug_assertions) {
                            let stored_descriptor = cache.get_descriptor(*entry.get());
                            if stored_descriptor != &descriptor {
                                error!("The cached pipeline descriptor for {} is not equal to the generated descriptor for the given key. This means the SpecializePipeline implementation uses 'unused' MeshVertexBufferLayout information to specialize the pipeline. This is not allowed because it would invalidate the pipeline cache.", std::any::type_name::<S>());
                            }
                        }
                        *entry.into_mut()
                    }
                    Entry::Vacant(entry) => {
                        *entry.insert(cache.queue(descriptor))
                    }
                }))
            }
        }
    }
}
