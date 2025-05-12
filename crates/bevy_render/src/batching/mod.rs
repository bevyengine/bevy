use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{ResMut, SystemParam, SystemParamItem},
};
use bytemuck::Pod;
use gpu_preprocessing::UntypedPhaseIndirectParametersBuffers;
use nonmax::NonMaxU32;

use crate::{
    render_phase::{
        BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItemExtraIndex,
        SortedPhaseItem, SortedRenderPhase, ViewBinnedRenderPhases,
    },
    render_resource::{CachedRenderPipelineId, GpuArrayBufferable},
    sync_world::MainEntity,
};

pub mod gpu_preprocessing;
pub mod no_gpu_preprocessing;

/// Add this component to mesh entities to disable automatic batching
#[derive(Component, Default)]
pub struct NoAutomaticBatching;

/// Data necessary to be equal for two draw commands to be mergeable
///
/// This is based on the following assumptions:
/// - Only entities with prepared assets (pipelines, materials, meshes) are
///   queued to phases
/// - View bindings are constant across a phase for a given draw function as
///   phases are per-view
/// - `batch_and_prepare_render_phase` is the only system that performs this
///   batching and has sole responsibility for preparing the per-object data.
///   As such the mesh binding and dynamic offsets are assumed to only be
///   variable as a result of the `batch_and_prepare_render_phase` system, e.g.
///   due to having to split data across separate uniform bindings within the
///   same buffer due to the maximum uniform buffer binding size.
#[derive(PartialEq)]
struct BatchMeta<T: PartialEq> {
    /// The pipeline id encompasses all pipeline configuration including vertex
    /// buffers and layouts, shaders and their specializations, bind group
    /// layouts, etc.
    pipeline_id: CachedRenderPipelineId,
    /// The draw function id defines the `RenderCommands` that are called to
    /// set the pipeline and bindings, and make the draw command
    draw_function_id: DrawFunctionId,
    dynamic_offset: Option<NonMaxU32>,
    user_data: T,
}

impl<T: PartialEq> BatchMeta<T> {
    fn new(item: &impl CachedRenderPipelinePhaseItem, user_data: T) -> Self {
        BatchMeta {
            pipeline_id: item.cached_pipeline(),
            draw_function_id: item.draw_function(),
            dynamic_offset: match item.extra_index() {
                PhaseItemExtraIndex::DynamicOffset(dynamic_offset) => {
                    NonMaxU32::new(dynamic_offset)
                }
                PhaseItemExtraIndex::None | PhaseItemExtraIndex::IndirectParametersIndex { .. } => {
                    None
                }
            },
            user_data,
        }
    }
}

/// A trait to support getting data used for batching draw commands via phase
/// items.
///
/// This is a simple version that only allows for sorting, not binning, as well
/// as only CPU processing, not GPU preprocessing. For these fancier features,
/// see [`GetFullBatchData`].
pub trait GetBatchData {
    /// The system parameters [`GetBatchData::get_batch_data`] needs in
    /// order to compute the batch data.
    type Param: SystemParam + 'static;
    /// Data used for comparison between phase items. If the pipeline id, draw
    /// function id, per-instance data buffer dynamic offset and this data
    /// matches, the draws can be batched.
    type CompareData: PartialEq;
    /// The per-instance data to be inserted into the
    /// [`crate::render_resource::GpuArrayBuffer`] containing these data for all
    /// instances.
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;
    /// Get the per-instance data to be inserted into the
    /// [`crate::render_resource::GpuArrayBuffer`]. If the instance can be
    /// batched, also return the data used for comparison when deciding whether
    /// draws can be batched, else return None for the `CompareData`.
    ///
    /// This is only called when building instance data on CPU. In the GPU
    /// instance data building path, we use
    /// [`GetFullBatchData::get_index_and_compare_data`] instead.
    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        query_item: (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)>;
}

/// A trait to support getting data used for batching draw commands via phase
/// items.
///
/// This version allows for binning and GPU preprocessing.
pub trait GetFullBatchData: GetBatchData {
    /// The per-instance data that was inserted into the
    /// [`crate::render_resource::BufferVec`] during extraction.
    type BufferInputData: Pod + Default + Sync + Send;

    /// Get the per-instance data to be inserted into the
    /// [`crate::render_resource::GpuArrayBuffer`].
    ///
    /// This is only called when building uniforms on CPU. In the GPU instance
    /// buffer building path, we use
    /// [`GetFullBatchData::get_index_and_compare_data`] instead.
    fn get_binned_batch_data(
        param: &SystemParamItem<Self::Param>,
        query_item: MainEntity,
    ) -> Option<Self::BufferData>;

    /// Returns the index of the [`GetFullBatchData::BufferInputData`] that the
    /// GPU preprocessing phase will use.
    ///
    /// We already inserted the [`GetFullBatchData::BufferInputData`] during the
    /// extraction phase before we got here, so this function shouldn't need to
    /// look up any render data. If CPU instance buffer building is in use, this
    /// function will never be called.
    fn get_index_and_compare_data(
        param: &SystemParamItem<Self::Param>,
        query_item: MainEntity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)>;

    /// Returns the index of the [`GetFullBatchData::BufferInputData`] that the
    /// GPU preprocessing phase will use.
    ///
    /// We already inserted the [`GetFullBatchData::BufferInputData`] during the
    /// extraction phase before we got here, so this function shouldn't need to
    /// look up any render data.
    ///
    /// This function is currently only called for unbatchable entities when GPU
    /// instance buffer building is in use. For batchable entities, the uniform
    /// index is written during queuing (e.g. in `queue_material_meshes`). In
    /// the case of CPU instance buffer building, the CPU writes the uniforms,
    /// so there's no index to return.
    fn get_binned_index(
        param: &SystemParamItem<Self::Param>,
        query_item: MainEntity,
    ) -> Option<NonMaxU32>;

    /// Writes the [`gpu_preprocessing::IndirectParametersGpuMetadata`]
    /// necessary to draw this batch into the given metadata buffer at the given
    /// index.
    ///
    /// This is only used if GPU culling is enabled (which requires GPU
    /// preprocessing).
    ///
    /// * `indexed` is true if the mesh is indexed or false if it's non-indexed.
    ///
    /// * `base_output_index` is the index of the first mesh instance in this
    ///   batch in the `MeshUniform` output buffer.
    ///
    /// * `batch_set_index` is the index of the batch set in the
    ///   [`gpu_preprocessing::IndirectBatchSet`] buffer, if this batch belongs to
    ///   a batch set.
    ///
    /// * `indirect_parameters_buffers` is the buffer in which to write the
    ///   metadata.
    ///
    /// * `indirect_parameters_offset` is the index in that buffer at which to
    ///   write the metadata.
    fn write_batch_indirect_parameters_metadata(
        indexed: bool,
        base_output_index: u32,
        batch_set_index: Option<NonMaxU32>,
        indirect_parameters_buffers: &mut UntypedPhaseIndirectParametersBuffers,
        indirect_parameters_offset: u32,
    );
}

/// Sorts a render phase that uses bins.
pub fn sort_binned_render_phase<BPI>(mut phases: ResMut<ViewBinnedRenderPhases<BPI>>)
where
    BPI: BinnedPhaseItem,
{
    for phase in phases.values_mut() {
        phase.multidrawable_meshes.sort_unstable_keys();
        phase.batchable_meshes.sort_unstable_keys();
        phase.unbatchable_meshes.sort_unstable_keys();
        phase.non_mesh_items.sort_unstable_keys();
    }
}

/// Batches the items in a sorted render phase.
///
/// This means comparing metadata needed to draw each phase item and trying to
/// combine the draws into a batch.
///
/// This is common code factored out from
/// [`gpu_preprocessing::batch_and_prepare_sorted_render_phase`] and
/// [`no_gpu_preprocessing::batch_and_prepare_sorted_render_phase`].
fn batch_and_prepare_sorted_render_phase<I, GBD>(
    phase: &mut SortedRenderPhase<I>,
    mut process_item: impl FnMut(&mut I) -> Option<GBD::CompareData>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    GBD: GetBatchData,
{
    let items = phase.items.iter_mut().map(|item| {
        let batch_data = match process_item(item) {
            Some(compare_data) if I::AUTOMATIC_BATCHING => Some(BatchMeta::new(item, compare_data)),
            _ => None,
        };
        (item.batch_range_mut(), batch_data)
    });

    items.reduce(|(start_range, prev_batch_meta), (range, batch_meta)| {
        if batch_meta.is_some() && prev_batch_meta == batch_meta {
            start_range.end = range.end;
            (start_range, prev_batch_meta)
        } else {
            (range, batch_meta)
        }
    });
}
