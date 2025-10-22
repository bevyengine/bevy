use crate::render_resource::{
    CachedComputePipelineId, CachedRenderPipelineId, ComputePipelineDescriptor, PipelineCache,
    RenderPipelineDescriptor,
};
use bevy_ecs::resource::Resource;
use bevy_mesh::{MeshVertexBufferLayoutRef, MissingVertexAttributeError, VertexBufferLayout};
use bevy_platform::{
    collections::{
        hash_map::{Entry, RawEntryMut, VacantEntry},
        HashMap,
    },
    hash::FixedHasher,
};
use bevy_utils::default;
use core::{fmt::Debug, hash::Hash};
use thiserror::Error;
use tracing::error;

/// A trait that allows constructing different variants of a render pipeline from a key.
///
/// Note: This is intended for modifying your pipeline descriptor on the basis of a key. If your key
/// contains no data then you don't need to specialize. For example, if you are using the
/// [`AsBindGroup`](crate::render_resource::AsBindGroup) without the `#[bind_group_data]` attribute,
/// you don't need to specialize. Instead, create the pipeline directly from [`PipelineCache`] and
/// store its ID.
///
/// See [`SpecializedRenderPipelines`] for more info.
pub trait SpecializedRenderPipeline {
    /// The key that defines each "variant" of the render pipeline.
    type Key: Clone + Hash + PartialEq + Eq;

    /// Construct a new render pipeline based on the provided key.
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor;
}

/// A convenience cache for creating different variants of a render pipeline based on some key.
///
/// Some render pipelines may need to be configured differently depending on the exact situation.
/// This cache allows constructing different render pipelines for each situation based on a key,
/// making it easy to A) construct the necessary pipelines, and B) reuse already constructed
/// pipelines.
///
/// Note: This is intended for modifying your pipeline descriptor on the basis of a key. If your key
/// contains no data then you don't need to specialize. For example, if you are using the
/// [`AsBindGroup`](crate::render_resource::AsBindGroup) without the `#[bind_group_data]` attribute,
/// you don't need to specialize. Instead, create the pipeline directly from [`PipelineCache`] and
/// store its ID.
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
    /// Get or create a specialized instance of the pipeline corresponding to `key`.
    pub fn specialize(
        &mut self,
        cache: &PipelineCache,
        pipeline_specializer: &S,
        key: S::Key,
    ) -> CachedRenderPipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = pipeline_specializer.specialize(key);
            cache.queue_render_pipeline(descriptor)
        })
    }
}

/// A trait that allows constructing different variants of a compute pipeline from a key.
///
/// Note: This is intended for modifying your pipeline descriptor on the basis of a key. If your key
/// contains no data then you don't need to specialize. For example, if you are using the
/// [`AsBindGroup`](crate::render_resource::AsBindGroup) without the `#[bind_group_data]` attribute,
/// you don't need to specialize. Instead, create the pipeline directly from [`PipelineCache`] and
/// store its ID.
///
/// See [`SpecializedComputePipelines`] for more info.
pub trait SpecializedComputePipeline {
    /// The key that defines each "variant" of the compute pipeline.
    type Key: Clone + Hash + PartialEq + Eq;

    /// Construct a new compute pipeline based on the provided key.
    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor;
}

/// A convenience cache for creating different variants of a compute pipeline based on some key.
///
/// Some compute pipelines may need to be configured differently depending on the exact situation.
/// This cache allows constructing different compute pipelines for each situation based on a key,
/// making it easy to A) construct the necessary pipelines, and B) reuse already constructed
/// pipelines.
///
/// Note: This is intended for modifying your pipeline descriptor on the basis of a key. If your key
/// contains no data then you don't need to specialize. For example, if you are using the
/// [`AsBindGroup`](crate::render_resource::AsBindGroup) without the `#[bind_group_data]` attribute,
/// you don't need to specialize. Instead, create the pipeline directly from [`PipelineCache`] and
/// store its ID.
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
    /// Get or create a specialized instance of the pipeline corresponding to `key`.
    pub fn specialize(
        &mut self,
        cache: &PipelineCache,
        specialize_pipeline: &S,
        key: S::Key,
    ) -> CachedComputePipelineId {
        *self.cache.entry(key.clone()).or_insert_with(|| {
            let descriptor = specialize_pipeline.specialize(key);
            cache.queue_compute_pipeline(descriptor)
        })
    }
}

/// A trait that allows constructing different variants of a render pipeline from a key and the
/// particular mesh's vertex buffer layout.
///
/// See [`SpecializedMeshPipelines`] for more info.
pub trait SpecializedMeshPipeline {
    /// The key that defines each "variant" of the render pipeline.
    type Key: Clone + Hash + PartialEq + Eq;

    /// Construct a new render pipeline based on the provided key and vertex layout.
    ///
    /// The returned pipeline descriptor should have a single vertex buffer, which is derived from
    /// `layout`.
    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError>;
}

/// A cache of different variants of a render pipeline based on a key and the particular mesh's
/// vertex buffer layout.
#[derive(Resource)]
pub struct SpecializedMeshPipelines<S: SpecializedMeshPipeline> {
    mesh_layout_cache: HashMap<(MeshVertexBufferLayoutRef, S::Key), CachedRenderPipelineId>,
    vertex_layout_cache: VertexLayoutCache<S>,
}

type VertexLayoutCache<S> = HashMap<
    VertexBufferLayout,
    HashMap<<S as SpecializedMeshPipeline>::Key, CachedRenderPipelineId>,
>;

impl<S: SpecializedMeshPipeline> Default for SpecializedMeshPipelines<S> {
    fn default() -> Self {
        Self {
            mesh_layout_cache: Default::default(),
            vertex_layout_cache: Default::default(),
        }
    }
}

impl<S: SpecializedMeshPipeline> SpecializedMeshPipelines<S> {
    /// Construct a new render pipeline based on the provided key and the mesh's vertex buffer
    /// layout.
    #[inline]
    pub fn specialize(
        &mut self,
        cache: &PipelineCache,
        pipeline_specializer: &S,
        key: S::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<CachedRenderPipelineId, SpecializedMeshPipelineError> {
        return match self.mesh_layout_cache.entry((layout.clone(), key.clone())) {
            Entry::Occupied(entry) => Ok(*entry.into_mut()),
            Entry::Vacant(entry) => specialize_slow(
                &mut self.vertex_layout_cache,
                cache,
                pipeline_specializer,
                key,
                layout,
                entry,
            ),
        };

        #[cold]
        fn specialize_slow<S>(
            vertex_layout_cache: &mut VertexLayoutCache<S>,
            cache: &PipelineCache,
            specialize_pipeline: &S,
            key: S::Key,
            layout: &MeshVertexBufferLayoutRef,
            entry: VacantEntry<
                (MeshVertexBufferLayoutRef, S::Key),
                CachedRenderPipelineId,
                FixedHasher,
            >,
        ) -> Result<CachedRenderPipelineId, SpecializedMeshPipelineError>
        where
            S: SpecializedMeshPipeline,
        {
            let descriptor = specialize_pipeline
                .specialize(key.clone(), layout)
                .map_err(|mut err| {
                    {
                        let SpecializedMeshPipelineError::MissingVertexAttribute(err) = &mut err;
                        err.pipeline_type = Some(core::any::type_name::<S>());
                    }
                    err
                })?;
            // Different MeshVertexBufferLayouts can produce the same final VertexBufferLayout
            // We want compatible vertex buffer layouts to use the same pipelines, so we must "deduplicate" them
            let layout_map = match vertex_layout_cache
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
                        let stored_descriptor = cache.get_render_pipeline_descriptor(*entry.get());
                        if stored_descriptor != &descriptor {
                            error!(
                                "The cached pipeline descriptor for {} is not \
                                    equal to the generated descriptor for the given key. \
                                    This means the SpecializePipeline implementation uses \
                                    unused' MeshVertexBufferLayout information to specialize \
                                    the pipeline. This is not allowed because it would invalidate \
                                    the pipeline cache.",
                                core::any::type_name::<S>()
                            );
                        }
                    }
                    *entry.into_mut()
                }
                Entry::Vacant(entry) => *entry.insert(cache.queue_render_pipeline(descriptor)),
            }))
        }
    }
}

#[derive(Error, Debug)]
pub enum SpecializedMeshPipelineError {
    #[error(transparent)]
    MissingVertexAttribute(#[from] MissingVertexAttributeError),
}
