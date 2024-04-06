use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Res,
    system::{Query, ResMut, StaticSystemParam, SystemParam, SystemParamItem},
};
use nonmax::NonMaxU32;
use smallvec::{smallvec, SmallVec};

use crate::{
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhase, BinnedRenderPhaseBatch, CachedRenderPipelinePhaseItem,
        DrawFunctionId, SortedPhaseItem, SortedRenderPhase,
    },
    render_resource::{CachedRenderPipelineId, GpuArrayBufferPool, GpuArrayBufferable},
    renderer::{RenderDevice, RenderQueue},
};

/// Add this component to mesh entities to disable automatic batching
#[derive(Component)]
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
    /// The draw function id defines the RenderCommands that are called to
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
            dynamic_offset: item.dynamic_offset(),
            user_data,
        }
    }
}

/// A trait to support getting data used for batching draw commands via phase
/// items.
pub trait GetBatchData {
    /// The system parameters [`GetBatchData::get_batch_data`] needs in
    /// order to compute the batch data.
    type Param: SystemParam + 'static;
    /// Data used for comparison between phase items. If the pipeline id, draw
    /// function id, per-instance data buffer dynamic offset and this data
    /// matches, the draws can be batched.
    type CompareData: PartialEq;
    /// The per-instance data to be inserted into the [`GpuArrayBuffer`]
    /// containing these data for all instances.
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;
    /// Get the per-instance data to be inserted into the [`GpuArrayBuffer`].
    /// If the instance can be batched, also return the data used for
    /// comparison when deciding whether draws can be batched, else return None
    /// for the `CompareData`.
    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)>;
}

pub fn clear_batch_buffer<F: GetBatchData>(
    mut gpu_array_buffer: ResMut<GpuArrayBufferPool<F::BufferData>>,
) {
    gpu_array_buffer.clear();
}

pub fn reserve_binned_batch_buffer<I: BinnedPhaseItem, F: GetBatchData>(
    mut gpu_array_buffer: ResMut<GpuArrayBufferPool<F::BufferData>>,
    mut views: Query<&mut BinnedRenderPhase<I>>,
) {
    for mut phase in &mut views {
        phase.reserved_range =
            wgpu::BufferSize::new(phase.len() as u64).map(|size| gpu_array_buffer.reserve(size));
    }
}

pub fn reserve_sorted_batch_buffer<I: SortedPhaseItem, F: GetBatchData>(
    mut gpu_array_buffer: ResMut<GpuArrayBufferPool<F::BufferData>>,
    mut views: Query<&mut SortedRenderPhase<I>>,
) {
    for mut phase in &mut views {
        phase.reserved_range = wgpu::BufferSize::new(phase.items.len() as u64)
            .map(|size| gpu_array_buffer.reserve(size));
    }
}

pub fn allocate_batch_buffer<F: GetBatchData>(
    mut gpu_array_buffer: ResMut<GpuArrayBufferPool<F::BufferData>>,
    device: Res<RenderDevice>,
) {
    gpu_array_buffer.allocate(&device);
}

/// When implemented on a pipeline, this trait allows the batching logic to
/// compute the per-batch data that will be uploaded to the GPU.
///
/// This includes things like the mesh transforms.
pub trait GetBinnedBatchData {
    /// The system parameters [`GetBinnedBatchData::get_batch_data`] needs
    /// in order to compute the batch data.
    type Param: SystemParam + 'static;
    /// The per-instance data to be inserted into the [`GpuArrayBuffer`]
    /// containing these data for all instances.
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;

    /// Get the per-instance data to be inserted into the [`GpuArrayBuffer`].
    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<Self::BufferData>;
}

/// Batch the items in a sorted render phase. This means comparing metadata
/// needed to draw each phase item and trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase<I, F>(
    gpu_array_buffer: Res<GpuArrayBufferPool<F::BufferData>>,
    mut views: Query<&mut SortedRenderPhase<I>>,
    render_queue: Res<RenderQueue>,
    param: StaticSystemParam<F::Param>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    F: GetBatchData,
    for<'w, 's> <<F as GetBatchData>::Param as SystemParam>::Item<'w, 's>: Sync,
{
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    let system_param_item = param.into_inner();

    views.par_iter_mut().for_each(|mut phase| {
        let Some(slice) = phase.reserved_range else {
            return;
        };
        let mut writer = gpu_array_buffer
            .get_writer(slice, &render_queue)
            .expect("GPU Array Buffer was not allocated.");

        let mut process_item = |item: &mut I| {
            let (buffer_data, compare_data) = F::get_batch_data(&system_param_item, item.entity())?;
            let buffer_index = writer.write(buffer_data);

            let index = buffer_index.index;
            *item.batch_range_mut() = index..index + 1;
            *item.dynamic_offset_mut() = buffer_index.dynamic_offset;

            if I::AUTOMATIC_BATCHING {
                compare_data.map(|compare_data| BatchMeta::new(item, compare_data))
            } else {
                None
            }
        };

        let items = phase.items.iter_mut().map(|item| {
            let batch_data = process_item(item);
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
    });
}

/// Sorts a render phase that uses bins.
pub fn sort_binned_render_phase<BPI>(mut views: Query<&mut BinnedRenderPhase<BPI>>)
where
    BPI: BinnedPhaseItem,
{
    for mut phase in &mut views {
        phase.batchable_keys.sort_unstable();
        phase.unbatchable_keys.sort_unstable();
    }
}

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase<BPI, GBBD>(
    gpu_array_buffer: ResMut<GpuArrayBufferPool<GBBD::BufferData>>,
    mut views: Query<&mut BinnedRenderPhase<BPI>>,
    render_queue: Res<RenderQueue>,
    param: StaticSystemParam<GBBD::Param>,
) where
    BPI: BinnedPhaseItem,
    GBBD: GetBinnedBatchData,
    for<'w, 's> <<GBBD as GetBinnedBatchData>::Param as SystemParam>::Item<'w, 's>: Sync,
{
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    let system_param_item = param.into_inner();

    views.par_iter_mut().for_each(|mut phase| {
        let phase = &mut *phase; // Borrow checker.
        let Some(slice) = phase.reserved_range else {
            return;
        };
        let mut writer = gpu_array_buffer
            .get_writer(slice, &render_queue)
            .expect("GPU Array Buffer was not allocated.");

        // Prepare batchables.

        for key in &phase.batchable_keys {
            let mut batch_set: SmallVec<[BinnedRenderPhaseBatch; 1]> = smallvec![];
            for &entity in &phase.batchable_values[key] {
                let Some(buffer_data) = GBBD::get_batch_data(&system_param_item, entity) else {
                    continue;
                };

                let instance = writer.write(buffer_data);

                // If the dynamic offset has changed, flush the batch.
                //
                // This is the only time we ever have more than one batch per
                // bin. Note that dynamic offsets are only used on platforms
                // with no storage buffers.
                if !batch_set.last().is_some_and(|batch| {
                    batch.instance_range.end == instance.index
                        && batch.dynamic_offset == instance.dynamic_offset
                }) {
                    batch_set.push(BinnedRenderPhaseBatch {
                        representative_entity: entity,
                        instance_range: instance.index..instance.index,
                        dynamic_offset: instance.dynamic_offset,
                    });
                }

                if let Some(batch) = batch_set.last_mut() {
                    batch.instance_range.end = instance.index + 1;
                }
            }

            phase.batch_sets.push(batch_set);
        }

        // Prepare unbatchables.
        for key in &phase.unbatchable_keys {
            let unbatchables = phase.unbatchable_values.get_mut(key).unwrap();
            for &entity in &unbatchables.entities {
                if let Some(buffer_data) = GBBD::get_batch_data(&system_param_item, entity) {
                    let instance = writer.write(buffer_data);
                    unbatchables.buffer_indices.add(instance);
                }
            }
        }
    });
}
