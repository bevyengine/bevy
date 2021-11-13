use crate::{
    mesh::{InnerMeshVertexBufferLayout, MeshVertexBufferLayout},
    render_resource::{
        CachedPipelineId, RenderPipelineCache, RenderPipelineDescriptor, VertexBufferLayout,
    },
};
use bevy_utils::{
    hashbrown::hash_map::RawEntryMut, Entry, HashMap, Hashed, PreHashMap, PreHashMapExt,
};
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
    ) -> CachedPipelineId {
        let map = self
            .mesh_layout_cache
            .get_or_insert_with(layout, Default::default);
        *map.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key.clone(), layout);
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
            match layout_map.entry(key) {
                Entry::Occupied(entry) => *entry.into_mut(),
                Entry::Vacant(entry) => *entry.insert(cache.queue(descriptor)),
            }
        })
    }
}

pub trait SpecializedMeshPipeline {
    type Key: Clone + Hash + PartialEq + Eq;
    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> RenderPipelineDescriptor;
}
