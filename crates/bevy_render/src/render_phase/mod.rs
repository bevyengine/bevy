//! The modular rendering abstraction responsible for queuing, preparing, sorting and drawing
//! entities as part of separate render phases.
//!
//! In Bevy each view (camera, or shadow-casting light, etc.) has one or multiple render phases
//! (e.g. opaque, transparent, shadow, etc).
//! They are used to queue entities for rendering.
//! Multiple phases might be required due to different sorting/batching behaviors
//! (e.g. opaque: front to back, transparent: back to front) or because one phase depends on
//! the rendered texture of the previous phase (e.g. for screen-space reflections).
//!
//! To draw an entity, a corresponding [`PhaseItem`] has to be added to one or multiple of these
//! render phases for each view that it is visible in.
//! This must be done in the [`RenderSet::Queue`].
//! After that the render phase sorts them in the [`RenderSet::PhaseSort`].
//! Finally the items are rendered using a single [`TrackedRenderPass`], during
//! the [`RenderSet::Render`].
//!
//! Therefore each phase item is assigned a [`Draw`] function.
//! These set up the state of the [`TrackedRenderPass`] (i.e. select the
//! [`RenderPipeline`](crate::render_resource::RenderPipeline), configure the
//! [`BindGroup`](crate::render_resource::BindGroup)s, etc.) and then issue a draw call,
//! for the corresponding item.
//!
//! The [`Draw`] function trait can either be implemented directly or such a function can be
//! created by composing multiple [`RenderCommand`]s.

mod draw;
mod draw_state;
mod rangefinder;

use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_platform_support::collections::{hash_map::Entry, HashMap};
use bevy_utils::default;
pub use draw::*;
pub use draw_state::*;
use encase::{internal::WriteInto, ShaderSize};
use nonmax::NonMaxU32;
pub use rangefinder::*;
use wgpu::Features;

use crate::batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport};
use crate::renderer::RenderDevice;
use crate::sync_world::MainEntity;
use crate::view::RetainedViewEntity;
use crate::{
    batching::{
        self,
        gpu_preprocessing::{self, BatchedInstanceBuffers},
        no_gpu_preprocessing::{self, BatchedInstanceBuffer},
        GetFullBatchData,
    },
    render_resource::{CachedRenderPipelineId, GpuArrayBufferIndex, PipelineCache},
    Render, RenderApp, RenderSet,
};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::SRes, SystemParamItem},
};
use core::{fmt::Debug, hash::Hash, iter, marker::PhantomData, ops::Range, slice::SliceIndex};
use smallvec::SmallVec;

/// Stores the rendering instructions for a single phase that uses bins in all
/// views.
///
/// They're cleared out every frame, but storing them in a resource like this
/// allows us to reuse allocations.
#[derive(Resource, Deref, DerefMut)]
pub struct ViewBinnedRenderPhases<BPI>(pub HashMap<RetainedViewEntity, BinnedRenderPhase<BPI>>)
where
    BPI: BinnedPhaseItem;

/// A collection of all rendering instructions, that will be executed by the GPU, for a
/// single render phase for a single view.
///
/// Each view (camera, or shadow-casting light, etc.) can have one or multiple render phases.
/// They are used to queue entities for rendering.
/// Multiple phases might be required due to different sorting/batching behaviors
/// (e.g. opaque: front to back, transparent: back to front) or because one phase depends on
/// the rendered texture of the previous phase (e.g. for screen-space reflections).
/// All [`PhaseItem`]s are then rendered using a single [`TrackedRenderPass`].
/// The render pass might be reused for multiple phases to reduce GPU overhead.
///
/// This flavor of render phase is used for phases in which the ordering is less
/// critical: for example, `Opaque3d`. It's generally faster than the
/// alternative [`SortedRenderPhase`].
pub struct BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// A list of `BatchSetKey`s for batchable, multidrawable items.
    ///
    /// These are accumulated in `queue_material_meshes` and then sorted in
    /// `batching::sort_binned_render_phase`.
    pub multidrawable_mesh_keys: Vec<BPI::BatchSetKey>,

    /// The multidrawable bins themselves.
    ///
    /// Each batch set key maps to a *batch set*, which in this case is a set of
    /// meshes that can be drawn together in one multidraw call. Each batch set
    /// is subdivided into *bins*, each of which represents a particular mesh.
    /// Each bin contains the entity IDs of instances of that mesh.
    ///
    /// So, for example, if there are two cubes and a sphere present in the
    /// scene, we would generally have one batch set containing two bins,
    /// assuming that the cubes and sphere meshes are allocated together and use
    /// the same pipeline. The first bin, corresponding to the cubes, will have
    /// two entities in it. The second bin, corresponding to the sphere, will
    /// have one entity in it.
    pub multidrawable_mesh_values: HashMap<BPI::BatchSetKey, HashMap<BPI::BinKey, RenderBin>>,

    /// A list of `BinKey`s for batchable items that aren't multidrawable.
    ///
    /// These are accumulated in `queue_material_meshes` and then sorted in
    /// `batch_and_prepare_binned_render_phase`.
    ///
    /// Usually, batchable items aren't multidrawable due to platform or
    /// hardware limitations. However, it's also possible to have batchable
    /// items alongside multidrawable items with custom mesh pipelines. See
    /// `specialized_mesh_pipeline` for an example.
    pub batchable_mesh_keys: Vec<(BPI::BatchSetKey, BPI::BinKey)>,

    /// The bins corresponding to batchable items that aren't multidrawable.
    ///
    /// For multidrawable entities, use `multidrawable_mesh_values`; for
    /// unbatchable entities, use `unbatchable_values`.
    pub batchable_mesh_values: HashMap<(BPI::BatchSetKey, BPI::BinKey), RenderBin>,

    /// A list of `BinKey`s for unbatchable items.
    ///
    /// These are accumulated in `queue_material_meshes` and then sorted in
    /// `batch_and_prepare_binned_render_phase`.
    pub unbatchable_mesh_keys: Vec<(BPI::BatchSetKey, BPI::BinKey)>,

    /// The unbatchable bins.
    ///
    /// Each entity here is rendered in a separate drawcall.
    pub unbatchable_mesh_values:
        HashMap<(BPI::BatchSetKey, BPI::BinKey), UnbatchableBinnedEntities>,

    /// Items in the bin that aren't meshes at all.
    ///
    /// Bevy itself doesn't place anything in this list, but plugins or your app
    /// can in order to execute custom drawing commands. Draw functions for each
    /// entity are simply called in order at rendering time.
    ///
    /// See the `custom_phase_item` example for an example of how to use this.
    pub non_mesh_items: Vec<(BPI::BatchSetKey, BPI::BinKey, (Entity, MainEntity))>,

    /// Information on each batch set.
    ///
    /// A *batch set* is a set of entities that will be batched together unless
    /// we're on a platform that doesn't support storage buffers (e.g. WebGL 2)
    /// and differing dynamic uniform indices force us to break batches. On
    /// platforms that support storage buffers, a batch set always consists of
    /// at most one batch.
    ///
    /// Multidrawable entities come first, then batchable entities, then
    /// unbatchable entities.
    pub(crate) batch_sets: BinnedRenderPhaseBatchSets<BPI::BinKey>,
}

/// All entities that share a mesh and a material and can be batched as part of
/// a [`BinnedRenderPhase`].
#[derive(Default)]
pub struct RenderBin {
    /// A list of the entities in each bin.
    pub entities: Vec<(Entity, MainEntity)>,
}

/// How we store and render the batch sets.
///
/// Each one of these corresponds to a [`GpuPreprocessingMode`].
pub enum BinnedRenderPhaseBatchSets<BK> {
    /// Batches are grouped into batch sets based on dynamic uniforms.
    ///
    /// This corresponds to [`GpuPreprocessingMode::None`].
    DynamicUniforms(Vec<SmallVec<[BinnedRenderPhaseBatch; 1]>>),

    /// Batches are never grouped into batch sets.
    ///
    /// This corresponds to [`GpuPreprocessingMode::PreprocessingOnly`].
    Direct(Vec<BinnedRenderPhaseBatch>),

    /// Batches are grouped together into batch sets based on their ability to
    /// be multi-drawn together.
    ///
    /// This corresponds to [`GpuPreprocessingMode::Culling`].
    MultidrawIndirect(Vec<BinnedRenderPhaseBatchSet<BK>>),
}

pub struct BinnedRenderPhaseBatchSet<BK> {
    pub(crate) batches: Vec<BinnedRenderPhaseBatch>,
    pub(crate) bin_key: BK,
    pub(crate) index: u32,
}

impl<BK> BinnedRenderPhaseBatchSets<BK> {
    fn clear(&mut self) {
        match *self {
            BinnedRenderPhaseBatchSets::DynamicUniforms(ref mut vec) => vec.clear(),
            BinnedRenderPhaseBatchSets::Direct(ref mut vec) => vec.clear(),
            BinnedRenderPhaseBatchSets::MultidrawIndirect(ref mut vec) => vec.clear(),
        }
    }
}

/// Information about a single batch of entities rendered using binned phase
/// items.
#[derive(Debug)]
pub struct BinnedRenderPhaseBatch {
    /// An entity that's *representative* of this batch.
    ///
    /// Bevy uses this to fetch the mesh. It can be any entity in the batch.
    pub representative_entity: (Entity, MainEntity),
    /// The range of instance indices in this batch.
    pub instance_range: Range<u32>,

    /// The dynamic offset of the batch.
    ///
    /// Note that dynamic offsets are only used on platforms that don't support
    /// storage buffers.
    pub extra_index: PhaseItemExtraIndex,
}

/// Information about the unbatchable entities in a bin.
pub struct UnbatchableBinnedEntities {
    /// The entities.
    pub entities: Vec<(Entity, MainEntity)>,

    /// The GPU array buffer indices of each unbatchable binned entity.
    pub(crate) buffer_indices: UnbatchableBinnedEntityIndexSet,
}

/// Stores instance indices and dynamic offsets for unbatchable entities in a
/// binned render phase.
///
/// This is conceptually `Vec<UnbatchableBinnedEntityDynamicOffset>`, but it
/// avoids the overhead of storing dynamic offsets on platforms that support
/// them. In other words, this allows a fast path that avoids allocation on
/// platforms that aren't WebGL 2.
#[derive(Default)]

pub(crate) enum UnbatchableBinnedEntityIndexSet {
    /// There are no unbatchable entities in this bin (yet).
    #[default]
    NoEntities,

    /// The instances for all unbatchable entities in this bin are contiguous,
    /// and there are no dynamic uniforms.
    ///
    /// This is the typical case on platforms other than WebGL 2. We special
    /// case this to avoid allocation on those platforms.
    Sparse {
        /// The range of indices.
        instance_range: Range<u32>,
        /// The index of the first indirect instance parameters.
        ///
        /// The other indices immediately follow these.
        first_indirect_parameters_index: Option<NonMaxU32>,
    },

    /// Dynamic uniforms are present for unbatchable entities in this bin.
    ///
    /// We fall back to this on WebGL 2.
    Dense(Vec<UnbatchableBinnedEntityIndices>),
}

/// The instance index and dynamic offset (if present) for an unbatchable entity.
///
/// This is only useful on platforms that don't support storage buffers.
#[derive(Clone)]
pub(crate) struct UnbatchableBinnedEntityIndices {
    /// The instance index.
    pub(crate) instance_index: u32,
    /// The [`PhaseItemExtraIndex`], if present.
    pub(crate) extra_index: PhaseItemExtraIndex,
}

/// Identifies the list within [`BinnedRenderPhase`] that a phase item is to be
/// placed in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BinnedRenderPhaseType {
    /// The item is a mesh that's eligible for multi-draw indirect rendering and
    /// can be batched with other meshes of the same type.
    MultidrawableMesh,

    /// The item is a mesh that's eligible for single-draw indirect rendering
    /// and can be batched with other meshes of the same type.
    BatchableMesh,

    /// The item is a mesh that's eligible for indirect rendering, but can't be
    /// batched with other meshes of the same type.
    ///
    /// At the moment, this is used for skinned meshes.
    UnbatchableMesh,

    /// The item isn't a mesh at all.
    ///
    /// Bevy will simply invoke the drawing commands for such items one after
    /// another, with no further processing.
    ///
    /// The engine itself doesn't enqueue any items of this type, but it's
    /// available for use in your application and/or plugins.
    NonMesh,
}

impl<T> From<GpuArrayBufferIndex<T>> for UnbatchableBinnedEntityIndices
where
    T: Clone + ShaderSize + WriteInto,
{
    fn from(value: GpuArrayBufferIndex<T>) -> Self {
        UnbatchableBinnedEntityIndices {
            instance_index: value.index,
            extra_index: PhaseItemExtraIndex::maybe_dynamic_offset(value.dynamic_offset),
        }
    }
}

impl<BPI> Default for ViewBinnedRenderPhases<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn default() -> Self {
        Self(default())
    }
}

impl<BPI> ViewBinnedRenderPhases<BPI>
where
    BPI: BinnedPhaseItem,
{
    pub fn insert_or_clear(
        &mut self,
        retained_view_entity: RetainedViewEntity,
        gpu_preprocessing: GpuPreprocessingMode,
    ) {
        match self.entry(retained_view_entity) {
            Entry::Occupied(mut entry) => entry.get_mut().clear(),
            Entry::Vacant(entry) => {
                entry.insert(BinnedRenderPhase::<BPI>::new(gpu_preprocessing));
            }
        }
    }
}

impl<BPI> BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// Bins a new entity.
    ///
    /// The `phase_type` parameter specifies whether the entity is a
    /// preprocessable mesh and whether it can be binned with meshes of the same
    /// type.
    pub fn add(
        &mut self,
        batch_set_key: BPI::BatchSetKey,
        bin_key: BPI::BinKey,
        (entity, main_entity): (Entity, MainEntity),
        phase_type: BinnedRenderPhaseType,
    ) {
        match phase_type {
            BinnedRenderPhaseType::MultidrawableMesh => {
                match self.multidrawable_mesh_values.entry(batch_set_key.clone()) {
                    Entry::Occupied(mut entry) => {
                        entry
                            .get_mut()
                            .entry(bin_key)
                            .or_default()
                            .entities
                            .push((entity, main_entity));
                    }
                    Entry::Vacant(entry) => {
                        self.multidrawable_mesh_keys.push(batch_set_key);
                        let mut new_batch_set = HashMap::default();
                        new_batch_set.insert(
                            bin_key,
                            RenderBin {
                                entities: vec![(entity, main_entity)],
                            },
                        );
                        entry.insert(new_batch_set);
                    }
                }
            }

            BinnedRenderPhaseType::BatchableMesh => {
                match self
                    .batchable_mesh_values
                    .entry((batch_set_key.clone(), bin_key.clone()).clone())
                {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().entities.push((entity, main_entity));
                    }
                    Entry::Vacant(entry) => {
                        self.batchable_mesh_keys.push((batch_set_key, bin_key));
                        entry.insert(RenderBin {
                            entities: vec![(entity, main_entity)],
                        });
                    }
                }
            }

            BinnedRenderPhaseType::UnbatchableMesh => {
                match self
                    .unbatchable_mesh_values
                    .entry((batch_set_key.clone(), bin_key.clone()))
                {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().entities.push((entity, main_entity));
                    }
                    Entry::Vacant(entry) => {
                        self.unbatchable_mesh_keys.push((batch_set_key, bin_key));
                        entry.insert(UnbatchableBinnedEntities {
                            entities: vec![(entity, main_entity)],
                            buffer_indices: default(),
                        });
                    }
                }
            }

            BinnedRenderPhaseType::NonMesh => {
                // We don't process these items further.
                self.non_mesh_items
                    .push((batch_set_key, bin_key, (entity, main_entity)));
            }
        }
    }

    /// Encodes the GPU commands needed to render all entities in this phase.
    pub fn render<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        {
            let draw_functions = world.resource::<DrawFunctions<BPI>>();
            let mut draw_functions = draw_functions.write();
            draw_functions.prepare(world);
            // Make sure to drop the reader-writer lock here to avoid recursive
            // locks.
        }

        self.render_batchable_meshes(render_pass, world, view)?;
        self.render_unbatchable_meshes(render_pass, world, view)?;
        self.render_non_meshes(render_pass, world, view)?;

        Ok(())
    }

    /// Renders all batchable meshes queued in this phase.
    fn render_batchable_meshes<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        let draw_functions = world.resource::<DrawFunctions<BPI>>();
        let mut draw_functions = draw_functions.write();

        let render_device = world.resource::<RenderDevice>();
        let multi_draw_indirect_count_supported = render_device
            .features()
            .contains(Features::MULTI_DRAW_INDIRECT_COUNT);

        match self.batch_sets {
            BinnedRenderPhaseBatchSets::DynamicUniforms(ref batch_sets) => {
                debug_assert_eq!(self.batchable_mesh_keys.len(), batch_sets.len());

                for ((batch_set_key, bin_key), batch_set) in
                    self.batchable_mesh_keys.iter().zip(batch_sets.iter())
                {
                    for batch in batch_set {
                        let binned_phase_item = BPI::new(
                            batch_set_key.clone(),
                            bin_key.clone(),
                            batch.representative_entity,
                            batch.instance_range.clone(),
                            batch.extra_index.clone(),
                        );

                        // Fetch the draw function.
                        let Some(draw_function) =
                            draw_functions.get_mut(binned_phase_item.draw_function())
                        else {
                            continue;
                        };

                        draw_function.draw(world, render_pass, view, &binned_phase_item)?;
                    }
                }
            }

            BinnedRenderPhaseBatchSets::Direct(ref batch_set) => {
                for (batch, (batch_set_key, bin_key)) in
                    batch_set.iter().zip(self.batchable_mesh_keys.iter())
                {
                    let binned_phase_item = BPI::new(
                        batch_set_key.clone(),
                        bin_key.clone(),
                        batch.representative_entity,
                        batch.instance_range.clone(),
                        batch.extra_index.clone(),
                    );

                    // Fetch the draw function.
                    let Some(draw_function) =
                        draw_functions.get_mut(binned_phase_item.draw_function())
                    else {
                        continue;
                    };

                    draw_function.draw(world, render_pass, view, &binned_phase_item)?;
                }
            }

            BinnedRenderPhaseBatchSets::MultidrawIndirect(ref batch_sets) => {
                for (batch_set_key, batch_set) in self
                    .multidrawable_mesh_keys
                    .iter()
                    .chain(
                        self.batchable_mesh_keys
                            .iter()
                            .map(|(batch_set_key, _)| batch_set_key),
                    )
                    .zip(batch_sets.iter())
                {
                    let Some(batch) = batch_set.batches.first() else {
                        continue;
                    };

                    let batch_set_index = if multi_draw_indirect_count_supported {
                        NonMaxU32::new(batch_set.index)
                    } else {
                        None
                    };

                    let binned_phase_item = BPI::new(
                        batch_set_key.clone(),
                        batch_set.bin_key.clone(),
                        batch.representative_entity,
                        batch.instance_range.clone(),
                        match batch.extra_index {
                            PhaseItemExtraIndex::None => PhaseItemExtraIndex::None,
                            PhaseItemExtraIndex::DynamicOffset(ref dynamic_offset) => {
                                PhaseItemExtraIndex::DynamicOffset(*dynamic_offset)
                            }
                            PhaseItemExtraIndex::IndirectParametersIndex { ref range, .. } => {
                                PhaseItemExtraIndex::IndirectParametersIndex {
                                    range: range.start
                                        ..(range.start + batch_set.batches.len() as u32),
                                    batch_set_index,
                                }
                            }
                        },
                    );

                    // Fetch the draw function.
                    let Some(draw_function) =
                        draw_functions.get_mut(binned_phase_item.draw_function())
                    else {
                        continue;
                    };

                    draw_function.draw(world, render_pass, view, &binned_phase_item)?;
                }
            }
        }

        Ok(())
    }

    /// Renders all unbatchable meshes queued in this phase.
    fn render_unbatchable_meshes<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        let draw_functions = world.resource::<DrawFunctions<BPI>>();
        let mut draw_functions = draw_functions.write();

        for (batch_set_key, bin_key) in &self.unbatchable_mesh_keys {
            let unbatchable_entities =
                &self.unbatchable_mesh_values[&(batch_set_key.clone(), bin_key.clone())];
            for (entity_index, &entity) in unbatchable_entities.entities.iter().enumerate() {
                let unbatchable_dynamic_offset = match &unbatchable_entities.buffer_indices {
                    UnbatchableBinnedEntityIndexSet::NoEntities => {
                        // Shouldn't happen…
                        continue;
                    }
                    UnbatchableBinnedEntityIndexSet::Sparse {
                        instance_range,
                        first_indirect_parameters_index,
                    } => UnbatchableBinnedEntityIndices {
                        instance_index: instance_range.start + entity_index as u32,
                        extra_index: match first_indirect_parameters_index {
                            None => PhaseItemExtraIndex::None,
                            Some(first_indirect_parameters_index) => {
                                let first_indirect_parameters_index_for_entity =
                                    u32::from(*first_indirect_parameters_index)
                                        + entity_index as u32;
                                PhaseItemExtraIndex::IndirectParametersIndex {
                                    range: first_indirect_parameters_index_for_entity
                                        ..(first_indirect_parameters_index_for_entity + 1),
                                    batch_set_index: None,
                                }
                            }
                        },
                    },
                    UnbatchableBinnedEntityIndexSet::Dense(ref dynamic_offsets) => {
                        dynamic_offsets[entity_index].clone()
                    }
                };

                let binned_phase_item = BPI::new(
                    batch_set_key.clone(),
                    bin_key.clone(),
                    entity,
                    unbatchable_dynamic_offset.instance_index
                        ..(unbatchable_dynamic_offset.instance_index + 1),
                    unbatchable_dynamic_offset.extra_index,
                );

                // Fetch the draw function.
                let Some(draw_function) = draw_functions.get_mut(binned_phase_item.draw_function())
                else {
                    continue;
                };

                draw_function.draw(world, render_pass, view, &binned_phase_item)?;
            }
        }
        Ok(())
    }

    /// Renders all objects of type [`BinnedRenderPhaseType::NonMesh`].
    ///
    /// These will have been added by plugins or the application.
    fn render_non_meshes<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        let draw_functions = world.resource::<DrawFunctions<BPI>>();
        let mut draw_functions = draw_functions.write();

        for &(ref batch_set_key, ref bin_key, entity) in &self.non_mesh_items {
            // Come up with a fake batch range and extra index. The draw
            // function is expected to manage any sort of batching logic itself.
            let binned_phase_item = BPI::new(
                batch_set_key.clone(),
                bin_key.clone(),
                entity,
                0..1,
                PhaseItemExtraIndex::None,
            );

            let Some(draw_function) = draw_functions.get_mut(binned_phase_item.draw_function())
            else {
                continue;
            };

            draw_function.draw(world, render_pass, view, &binned_phase_item)?;
        }

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.multidrawable_mesh_keys.is_empty()
            && self.batchable_mesh_keys.is_empty()
            && self.unbatchable_mesh_keys.is_empty()
            && self.non_mesh_items.is_empty()
    }

    pub fn clear(&mut self) {
        self.multidrawable_mesh_keys.clear();
        self.multidrawable_mesh_values.clear();
        self.batchable_mesh_keys.clear();
        self.batchable_mesh_values.clear();
        self.unbatchable_mesh_keys.clear();
        self.unbatchable_mesh_values.clear();
        self.non_mesh_items.clear();
        self.batch_sets.clear();
    }
}

impl<BPI> BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn new(gpu_preprocessing: GpuPreprocessingMode) -> Self {
        Self {
            multidrawable_mesh_keys: vec![],
            multidrawable_mesh_values: HashMap::default(),
            batchable_mesh_keys: vec![],
            batchable_mesh_values: HashMap::default(),
            unbatchable_mesh_keys: vec![],
            unbatchable_mesh_values: HashMap::default(),
            non_mesh_items: vec![],
            batch_sets: match gpu_preprocessing {
                GpuPreprocessingMode::Culling => {
                    BinnedRenderPhaseBatchSets::MultidrawIndirect(vec![])
                }
                GpuPreprocessingMode::PreprocessingOnly => {
                    BinnedRenderPhaseBatchSets::Direct(vec![])
                }
                GpuPreprocessingMode::None => BinnedRenderPhaseBatchSets::DynamicUniforms(vec![]),
            },
        }
    }
}

impl UnbatchableBinnedEntityIndexSet {
    /// Returns the [`UnbatchableBinnedEntityIndices`] for the given entity.
    fn indices_for_entity_index(
        &self,
        entity_index: u32,
    ) -> Option<UnbatchableBinnedEntityIndices> {
        match self {
            UnbatchableBinnedEntityIndexSet::NoEntities => None,
            UnbatchableBinnedEntityIndexSet::Sparse { instance_range, .. }
                if entity_index >= instance_range.len() as u32 =>
            {
                None
            }
            UnbatchableBinnedEntityIndexSet::Sparse {
                instance_range,
                first_indirect_parameters_index: None,
            } => Some(UnbatchableBinnedEntityIndices {
                instance_index: instance_range.start + entity_index,
                extra_index: PhaseItemExtraIndex::None,
            }),
            UnbatchableBinnedEntityIndexSet::Sparse {
                instance_range,
                first_indirect_parameters_index: Some(first_indirect_parameters_index),
            } => {
                let first_indirect_parameters_index_for_this_batch =
                    u32::from(*first_indirect_parameters_index) + entity_index;
                Some(UnbatchableBinnedEntityIndices {
                    instance_index: instance_range.start + entity_index,
                    extra_index: PhaseItemExtraIndex::IndirectParametersIndex {
                        range: first_indirect_parameters_index_for_this_batch
                            ..(first_indirect_parameters_index_for_this_batch + 1),
                        batch_set_index: None,
                    },
                })
            }
            UnbatchableBinnedEntityIndexSet::Dense(ref indices) => {
                indices.get(entity_index as usize).cloned()
            }
        }
    }
}

/// A convenient abstraction for adding all the systems necessary for a binned
/// render phase to the render app.
///
/// This is the version used when the pipeline supports GPU preprocessing: e.g.
/// 3D PBR meshes.
pub struct BinnedRenderPhasePlugin<BPI, GFBD>(PhantomData<(BPI, GFBD)>)
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData;

impl<BPI, GFBD> Default for BinnedRenderPhasePlugin<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<BPI, GFBD> Plugin for BinnedRenderPhasePlugin<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData + Sync + Send + 'static,
{
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ViewBinnedRenderPhases<BPI>>()
            .add_systems(
                Render,
                (
                    batching::sort_binned_render_phase::<BPI>.in_set(RenderSet::PhaseSort),
                    (
                        no_gpu_preprocessing::batch_and_prepare_binned_render_phase::<BPI, GFBD>
                            .run_if(resource_exists::<BatchedInstanceBuffer<GFBD::BufferData>>),
                        gpu_preprocessing::batch_and_prepare_binned_render_phase::<BPI, GFBD>
                            .run_if(
                                resource_exists::<
                                    BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
                                >,
                            ),
                    )
                        .in_set(RenderSet::PrepareResources),
                ),
            );
    }
}

/// Stores the rendering instructions for a single phase that sorts items in all
/// views.
///
/// They're cleared out every frame, but storing them in a resource like this
/// allows us to reuse allocations.
#[derive(Resource, Deref, DerefMut)]
pub struct ViewSortedRenderPhases<SPI>(pub HashMap<RetainedViewEntity, SortedRenderPhase<SPI>>)
where
    SPI: SortedPhaseItem;

impl<SPI> Default for ViewSortedRenderPhases<SPI>
where
    SPI: SortedPhaseItem,
{
    fn default() -> Self {
        Self(default())
    }
}

impl<SPI> ViewSortedRenderPhases<SPI>
where
    SPI: SortedPhaseItem,
{
    pub fn insert_or_clear(&mut self, retained_view_entity: RetainedViewEntity) {
        match self.entry(retained_view_entity) {
            Entry::Occupied(mut entry) => entry.get_mut().clear(),
            Entry::Vacant(entry) => {
                entry.insert(default());
            }
        }
    }
}

/// A convenient abstraction for adding all the systems necessary for a sorted
/// render phase to the render app.
///
/// This is the version used when the pipeline supports GPU preprocessing: e.g.
/// 3D PBR meshes.
pub struct SortedRenderPhasePlugin<SPI, GFBD>(PhantomData<(SPI, GFBD)>)
where
    SPI: SortedPhaseItem,
    GFBD: GetFullBatchData;

impl<SPI, GFBD> Default for SortedRenderPhasePlugin<SPI, GFBD>
where
    SPI: SortedPhaseItem,
    GFBD: GetFullBatchData,
{
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<SPI, GFBD> Plugin for SortedRenderPhasePlugin<SPI, GFBD>
where
    SPI: SortedPhaseItem + CachedRenderPipelinePhaseItem,
    GFBD: GetFullBatchData + Sync + Send + 'static,
{
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ViewSortedRenderPhases<SPI>>()
            .add_systems(
                Render,
                (
                    no_gpu_preprocessing::batch_and_prepare_sorted_render_phase::<SPI, GFBD>
                        .run_if(resource_exists::<BatchedInstanceBuffer<GFBD::BufferData>>),
                    gpu_preprocessing::batch_and_prepare_sorted_render_phase::<SPI, GFBD>.run_if(
                        resource_exists::<
                            BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
                        >,
                    ),
                )
                    .in_set(RenderSet::PrepareResources),
            );
    }
}

impl UnbatchableBinnedEntityIndexSet {
    /// Adds a new entity to the list of unbatchable binned entities.
    pub fn add(&mut self, indices: UnbatchableBinnedEntityIndices) {
        match self {
            UnbatchableBinnedEntityIndexSet::NoEntities => {
                match indices.extra_index {
                    PhaseItemExtraIndex::DynamicOffset(_) => {
                        // This is the first entity we've seen, and we don't have
                        // compute shaders. Initialize an array.
                        *self = UnbatchableBinnedEntityIndexSet::Dense(vec![indices]);
                    }
                    PhaseItemExtraIndex::None => {
                        // This is the first entity we've seen, and we have compute
                        // shaders. Initialize the fast path.
                        *self = UnbatchableBinnedEntityIndexSet::Sparse {
                            instance_range: indices.instance_index..indices.instance_index + 1,
                            first_indirect_parameters_index: None,
                        }
                    }
                    PhaseItemExtraIndex::IndirectParametersIndex {
                        range: ref indirect_parameters_index,
                        ..
                    } => {
                        // This is the first entity we've seen, and we have compute
                        // shaders. Initialize the fast path.
                        *self = UnbatchableBinnedEntityIndexSet::Sparse {
                            instance_range: indices.instance_index..indices.instance_index + 1,
                            first_indirect_parameters_index: NonMaxU32::new(
                                indirect_parameters_index.start,
                            ),
                        }
                    }
                }
            }

            UnbatchableBinnedEntityIndexSet::Sparse {
                ref mut instance_range,
                first_indirect_parameters_index,
            } if instance_range.end == indices.instance_index
                && ((first_indirect_parameters_index.is_none()
                    && indices.extra_index == PhaseItemExtraIndex::None)
                    || first_indirect_parameters_index.is_some_and(
                        |first_indirect_parameters_index| match indices.extra_index {
                            PhaseItemExtraIndex::IndirectParametersIndex {
                                range: ref this_range,
                                ..
                            } => {
                                u32::from(first_indirect_parameters_index) + instance_range.end
                                    - instance_range.start
                                    == this_range.start
                            }
                            PhaseItemExtraIndex::DynamicOffset(_) | PhaseItemExtraIndex::None => {
                                false
                            }
                        },
                    )) =>
            {
                // This is the normal case on non-WebGL 2.
                instance_range.end += 1;
            }

            UnbatchableBinnedEntityIndexSet::Sparse { instance_range, .. } => {
                // We thought we were in non-WebGL 2 mode, but we got a dynamic
                // offset or non-contiguous index anyway. This shouldn't happen,
                // but let's go ahead and do the sensible thing anyhow: demote
                // the compressed `NoDynamicOffsets` field to the full
                // `DynamicOffsets` array.
                let new_dynamic_offsets = (0..instance_range.len() as u32)
                    .flat_map(|entity_index| self.indices_for_entity_index(entity_index))
                    .chain(iter::once(indices))
                    .collect();
                *self = UnbatchableBinnedEntityIndexSet::Dense(new_dynamic_offsets);
            }

            UnbatchableBinnedEntityIndexSet::Dense(ref mut dense_indices) => {
                dense_indices.push(indices);
            }
        }
    }
}

/// A collection of all items to be rendered that will be encoded to GPU
/// commands for a single render phase for a single view.
///
/// Each view (camera, or shadow-casting light, etc.) can have one or multiple render phases.
/// They are used to queue entities for rendering.
/// Multiple phases might be required due to different sorting/batching behaviors
/// (e.g. opaque: front to back, transparent: back to front) or because one phase depends on
/// the rendered texture of the previous phase (e.g. for screen-space reflections).
/// All [`PhaseItem`]s are then rendered using a single [`TrackedRenderPass`].
/// The render pass might be reused for multiple phases to reduce GPU overhead.
///
/// This flavor of render phase is used only for meshes that need to be sorted
/// back-to-front, such as transparent meshes. For items that don't need strict
/// sorting, [`BinnedRenderPhase`] is preferred, for performance.
pub struct SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
    /// The items within this [`SortedRenderPhase`].
    pub items: Vec<I>,
}

impl<I> Default for SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<I> SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
    /// Adds a [`PhaseItem`] to this render phase.
    #[inline]
    pub fn add(&mut self, item: I) {
        self.items.push(item);
    }

    /// Removes all [`PhaseItem`]s from this render phase.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Sorts all of its [`PhaseItem`]s.
    pub fn sort(&mut self) {
        I::sort(&mut self.items);
    }

    /// An [`Iterator`] through the associated [`Entity`] for each [`PhaseItem`] in order.
    #[inline]
    pub fn iter_entities(&'_ self) -> impl Iterator<Item = Entity> + '_ {
        self.items.iter().map(PhaseItem::entity)
    }

    /// Renders all of its [`PhaseItem`]s using their corresponding draw functions.
    pub fn render<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        self.render_range(render_pass, world, view, ..)
    }

    /// Renders all [`PhaseItem`]s in the provided `range` (based on their index in `self.items`) using their corresponding draw functions.
    pub fn render_range<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
        range: impl SliceIndex<[I], Output = [I]>,
    ) -> Result<(), DrawError> {
        let items = self
            .items
            .get(range)
            .expect("`Range` provided to `render_range()` is out of bounds");

        let draw_functions = world.resource::<DrawFunctions<I>>();
        let mut draw_functions = draw_functions.write();
        draw_functions.prepare(world);

        let mut index = 0;
        while index < items.len() {
            let item = &items[index];
            let batch_range = item.batch_range();
            if batch_range.is_empty() {
                index += 1;
            } else {
                let draw_function = draw_functions.get_mut(item.draw_function()).unwrap();
                draw_function.draw(world, render_pass, view, item)?;
                index += batch_range.len();
            }
        }
        Ok(())
    }
}

/// An item (entity of the render world) which will be drawn to a texture or the screen,
/// as part of a render phase.
///
/// The data required for rendering an entity is extracted from the main world in the
/// [`ExtractSchedule`](crate::ExtractSchedule).
/// Then it has to be queued up for rendering during the [`RenderSet::Queue`],
/// by adding a corresponding phase item to a render phase.
/// Afterwards it will be possibly sorted and rendered automatically in the
/// [`RenderSet::PhaseSort`] and [`RenderSet::Render`], respectively.
///
/// `PhaseItem`s come in two flavors: [`BinnedPhaseItem`]s and
/// [`SortedPhaseItem`]s.
///
/// * Binned phase items have a `BinKey` which specifies what bin they're to be
///     placed in. All items in the same bin are eligible to be batched together.
///     The `BinKey`s are sorted, but the individual bin items aren't. Binned phase
///     items are good for opaque meshes, in which the order of rendering isn't
///     important. Generally, binned phase items are faster than sorted phase items.
///
/// * Sorted phase items, on the other hand, are placed into one large buffer
///     and then sorted all at once. This is needed for transparent meshes, which
///     have to be sorted back-to-front to render with the painter's algorithm.
///     These types of phase items are generally slower than binned phase items.
pub trait PhaseItem: Sized + Send + Sync + 'static {
    /// Whether or not this `PhaseItem` should be subjected to automatic batching. (Default: `true`)
    const AUTOMATIC_BATCHING: bool = true;

    /// The corresponding entity that will be drawn.
    ///
    /// This is used to fetch the render data of the entity, required by the draw function,
    /// from the render world .
    fn entity(&self) -> Entity;

    /// The main world entity represented by this `PhaseItem`.
    fn main_entity(&self) -> MainEntity;

    /// Specifies the [`Draw`] function used to render the item.
    fn draw_function(&self) -> DrawFunctionId;

    /// The range of instances that the batch covers. After doing a batched draw, batch range
    /// length phase items will be skipped. This design is to avoid having to restructure the
    /// render phase unnecessarily.
    fn batch_range(&self) -> &Range<u32>;
    fn batch_range_mut(&mut self) -> &mut Range<u32>;

    /// Returns the [`PhaseItemExtraIndex`].
    ///
    /// If present, this is either a dynamic offset or an indirect parameters
    /// index.
    fn extra_index(&self) -> PhaseItemExtraIndex;

    /// Returns a pair of mutable references to both the batch range and extra
    /// index.
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex);
}

/// The "extra index" associated with some [`PhaseItem`]s, alongside the
/// indirect instance index.
///
/// Sometimes phase items require another index in addition to the range of
/// instances they already have. These can be:
///
/// * The *dynamic offset*: a `wgpu` dynamic offset into the uniform buffer of
///     instance data. This is used on platforms that don't support storage
///     buffers, to work around uniform buffer size limitations.
///
/// * The *indirect parameters index*: an index into the buffer that specifies
///     the indirect parameters for this [`PhaseItem`]'s drawcall. This is used when
///     indirect mode is on (as used for GPU culling).
///
/// Note that our indirect draw functionality requires storage buffers, so it's
/// impossible to have both a dynamic offset and an indirect parameters index.
/// This convenient fact allows us to pack both indices into a single `u32`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum PhaseItemExtraIndex {
    /// No extra index is present.
    None,
    /// A `wgpu` dynamic offset into the uniform buffer of instance data. This
    /// is used on platforms that don't support storage buffers, to work around
    /// uniform buffer size limitations.
    DynamicOffset(u32),
    /// An index into the buffer that specifies the indirect parameters for this
    /// [`PhaseItem`]'s drawcall. This is used when indirect mode is on (as used
    /// for GPU culling).
    IndirectParametersIndex {
        /// The range of indirect parameters within the indirect parameters array.
        ///
        /// If we're using `multi_draw_indirect_count`, this specifies the
        /// maximum range of indirect parameters within that array. If batches
        /// are ultimately culled out on the GPU, the actual number of draw
        /// commands might be lower than the length of this range.
        range: Range<u32>,
        /// If `multi_draw_indirect_count` is in use, and this phase item is
        /// part of a batch set, specifies the index of the batch set that this
        /// phase item is a part of.
        ///
        /// If `multi_draw_indirect_count` isn't in use, or this phase item
        /// isn't part of a batch set, this is `None`.
        batch_set_index: Option<NonMaxU32>,
    },
}

impl PhaseItemExtraIndex {
    /// Returns either an indirect parameters index or
    /// [`PhaseItemExtraIndex::None`], as appropriate.
    pub fn maybe_indirect_parameters_index(
        indirect_parameters_index: Option<NonMaxU32>,
    ) -> PhaseItemExtraIndex {
        match indirect_parameters_index {
            Some(indirect_parameters_index) => PhaseItemExtraIndex::IndirectParametersIndex {
                range: u32::from(indirect_parameters_index)
                    ..(u32::from(indirect_parameters_index) + 1),
                batch_set_index: None,
            },
            None => PhaseItemExtraIndex::None,
        }
    }

    /// Returns either a dynamic offset index or [`PhaseItemExtraIndex::None`],
    /// as appropriate.
    pub fn maybe_dynamic_offset(dynamic_offset: Option<NonMaxU32>) -> PhaseItemExtraIndex {
        match dynamic_offset {
            Some(dynamic_offset) => PhaseItemExtraIndex::DynamicOffset(dynamic_offset.into()),
            None => PhaseItemExtraIndex::None,
        }
    }
}

/// Represents phase items that are placed into bins. The `BinKey` specifies
/// which bin they're to be placed in. Bin keys are sorted, and items within the
/// same bin are eligible to be batched together. The elements within the bins
/// aren't themselves sorted.
///
/// An example of a binned phase item is `Opaque3d`, for which the rendering
/// order isn't critical.
pub trait BinnedPhaseItem: PhaseItem {
    /// The key used for binning [`PhaseItem`]s into bins. Order the members of
    /// [`BinnedPhaseItem::BinKey`] by the order of binding for best
    /// performance. For example, pipeline id, draw function id, mesh asset id,
    /// lowest variable bind group id such as the material bind group id, and
    /// its dynamic offsets if any, next bind group and offsets, etc. This
    /// reduces the need for rebinding between bins and improves performance.
    type BinKey: Clone + Send + Sync + PartialEq + Eq + Ord + Hash;

    /// The key used to combine batches into batch sets.
    ///
    /// A *batch set* is a set of meshes that can potentially be multi-drawn
    /// together.
    type BatchSetKey: PhaseItemBatchSetKey;

    /// Creates a new binned phase item from the key and per-entity data.
    ///
    /// Unlike [`SortedPhaseItem`]s, this is generally called "just in time"
    /// before rendering. The resulting phase item isn't stored in any data
    /// structures, resulting in significant memory savings.
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self;
}

/// A key used to combine batches into batch sets.
///
/// A *batch set* is a set of meshes that can potentially be multi-drawn
/// together.
pub trait PhaseItemBatchSetKey: Clone + Send + Sync + PartialEq + Eq + Ord + Hash {
    /// Returns true if this batch set key describes indexed meshes or false if
    /// it describes non-indexed meshes.
    ///
    /// Bevy uses this in order to determine which kind of indirect draw
    /// parameters to use, if indirect drawing is enabled.
    fn indexed(&self) -> bool;
}

/// Represents phase items that must be sorted. The `SortKey` specifies the
/// order that these items are drawn in. These are placed into a single array,
/// and the array as a whole is then sorted.
///
/// An example of a sorted phase item is `Transparent3d`, which must be sorted
/// back to front in order to correctly render with the painter's algorithm.
pub trait SortedPhaseItem: PhaseItem {
    /// The type used for ordering the items. The smallest values are drawn first.
    /// This order can be calculated using the [`ViewRangefinder3d`],
    /// based on the view-space `Z` value of the corresponding view matrix.
    type SortKey: Ord;

    /// Determines the order in which the items are drawn.
    fn sort_key(&self) -> Self::SortKey;

    /// Sorts a slice of phase items into render order. Generally if the same type
    /// is batched this should use a stable sort like [`slice::sort_by_key`].
    /// In almost all other cases, this should not be altered from the default,
    /// which uses an unstable sort, as this provides the best balance of CPU and GPU
    /// performance.
    ///
    /// Implementers can optionally not sort the list at all. This is generally advisable if and
    /// only if the renderer supports a depth prepass, which is by default not supported by
    /// the rest of Bevy's first party rendering crates. Even then, this may have a negative
    /// impact on GPU-side performance due to overdraw.
    ///
    /// It's advised to always profile for performance changes when changing this implementation.
    #[inline]
    fn sort(items: &mut [Self]) {
        items.sort_unstable_by_key(Self::sort_key);
    }

    /// Whether this phase item targets indexed meshes (those with both vertex
    /// and index buffers as opposed to just vertex buffers).
    ///
    /// Bevy needs this information in order to properly group phase items
    /// together for multi-draw indirect, because the GPU layout of indirect
    /// commands differs between indexed and non-indexed meshes.
    ///
    /// If you're implementing a custom phase item that doesn't describe a mesh,
    /// you can safely return false here.
    fn indexed(&self) -> bool;
}

/// A [`PhaseItem`] item, that automatically sets the appropriate render pipeline,
/// cached in the [`PipelineCache`].
///
/// You can use the [`SetItemPipeline`] render command to set the pipeline for this item.
pub trait CachedRenderPipelinePhaseItem: PhaseItem {
    /// The id of the render pipeline, cached in the [`PipelineCache`], that will be used to draw
    /// this phase item.
    fn cached_pipeline(&self) -> CachedRenderPipelineId;
}

/// A [`RenderCommand`] that sets the pipeline for the [`CachedRenderPipelinePhaseItem`].
pub struct SetItemPipeline;

impl<P: CachedRenderPipelinePhaseItem> RenderCommand<P> for SetItemPipeline {
    type Param = SRes<PipelineCache>;
    type ViewQuery = ();
    type ItemQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _entity: Option<()>,
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(pipeline) = pipeline_cache
            .into_inner()
            .get_render_pipeline(item.cached_pipeline())
        {
            pass.set_render_pipeline(pipeline);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Skip
        }
    }
}

/// This system sorts the [`PhaseItem`]s of all [`SortedRenderPhase`]s of this
/// type.
pub fn sort_phase_system<I>(mut render_phases: ResMut<ViewSortedRenderPhases<I>>)
where
    I: SortedPhaseItem,
{
    for phase in render_phases.values_mut() {
        phase.sort();
    }
}

impl BinnedRenderPhaseType {
    pub fn mesh(
        batchable: bool,
        gpu_preprocessing_support: &GpuPreprocessingSupport,
    ) -> BinnedRenderPhaseType {
        match (batchable, gpu_preprocessing_support.max_supported_mode) {
            (true, GpuPreprocessingMode::Culling) => BinnedRenderPhaseType::MultidrawableMesh,
            (true, _) => BinnedRenderPhaseType::BatchableMesh,
            (false, _) => BinnedRenderPhaseType::UnbatchableMesh,
        }
    }
}
