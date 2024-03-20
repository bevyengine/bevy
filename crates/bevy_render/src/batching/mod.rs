use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Res,
    system::{Query, ResMut, StaticSystemParam, SystemParam, SystemParamItem},
};
use nonmax::NonMaxU32;

use crate::{
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhase, BinnedRenderPhaseBatch, CachedRenderPipelinePhaseItem,
        DrawFunctionId, SortedPhaseItem, SortedRenderPhase,
    },
    render_resource::{CachedRenderPipelineId, GpuArrayBuffer, GpuArrayBufferable},
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

pub trait GetBinnedBatchData {
    type Param: SystemParam + 'static;
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;

    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<Self::BufferData>;
}

/// Batch the items in a sorted render phase. This means comparing metadata
/// needed to draw each phase item and trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase<I, F>(
    gpu_array_buffer: ResMut<GpuArrayBuffer<F::BufferData>>,
    mut views: Query<&mut SortedRenderPhase<I>>,
    param: StaticSystemParam<F::Param>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    F: GetBatchData,
{
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    let system_param_item = param.into_inner();

    let mut process_item = |item: &mut I| {
        let (buffer_data, compare_data) = F::get_batch_data(&system_param_item, item.entity())?;
        let buffer_index = gpu_array_buffer.push(buffer_data);

        let index = buffer_index.index;
        *item.batch_range_mut() = index..index + 1;
        *item.dynamic_offset_mut() = buffer_index.dynamic_offset;

        if I::AUTOMATIC_BATCHING {
            compare_data.map(|compare_data| BatchMeta::new(item, compare_data))
        } else {
            None
        }
    };

    for mut phase in &mut views {
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
    }
}

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase<BPI, GBBD>(
    gpu_array_buffer: ResMut<GpuArrayBuffer<GBBD::BufferData>>,
    mut views: Query<&mut BinnedRenderPhase<BPI>>,
    param: StaticSystemParam<GBBD::Param>,
) where
    BPI: BinnedPhaseItem,
    GBBD: GetBinnedBatchData,
{
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    let system_param_item = param.into_inner();

    for mut phase in &mut views {
        phase.batchable_keys.sort();
        phase.unbatchable_keys.sort();

        let phase = &mut *phase; // Borrow checker.

        // Prepare batchables.

        let mut is_first_instance = true;
        for key in &phase.batchable_keys {
            let mut batch: Option<BinnedRenderPhaseBatch> = None;
            for &entity in &phase.batchable_values[key] {
                let Some(buffer_data) = GBBD::get_batch_data(&system_param_item, entity) else {
                    let instance_index = match phase.batches.last() {
                        Some(batch) => batch.last_instance_index,
                        None => 0,
                    };
                    phase
                        .batches
                        .push(BinnedRenderPhaseBatch::placeholder(instance_index));
                    continue;
                };

                let instance = gpu_array_buffer.push(buffer_data);
                if is_first_instance {
                    phase.first_instance_index = instance.index;
                    is_first_instance = false;
                }

                // If the dynamic offset has changed, flush the batch.
                //
                // This is the only time we ever have more than one batch per
                // bin. Note that dynamic offsets are only used on platforms
                // with no storage buffers.
                if batch
                    .as_ref()
                    .is_some_and(|batch| batch.dynamic_offset != instance.dynamic_offset)
                {
                    phase.batches.push(batch.take().unwrap());
                }

                match batch {
                    None => {
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: entity,
                            last_instance_index: instance.index + 1,
                            dynamic_offset: instance.dynamic_offset,
                        });
                    }
                    Some(ref mut batch) => batch.last_instance_index += 1,
                }
            }

            phase.batches.push(batch.unwrap_or_else(|| {
                let instance_index = match phase.batches.last() {
                    Some(batch) => batch.last_instance_index,
                    None => 0,
                };
                BinnedRenderPhaseBatch::placeholder(instance_index)
            }));
        }

        // Prepare unbatchables.

        for key in &phase.unbatchable_keys {
            let unbatchables = phase.unbatchable_values.get_mut(key).unwrap();
            for (entity_index, &entity) in unbatchables.entities.iter().enumerate() {
                let Some(buffer_data) = GBBD::get_batch_data(&system_param_item, entity) else {
                    continue;
                };

                let instance = gpu_array_buffer.push(buffer_data);
                if is_first_instance {
                    phase.first_instance_index = instance.index;
                    is_first_instance = false;
                }

                if let Some(dynamic_offset) = instance.dynamic_offset {
                    unbatchables.dynamic_offsets.resize(entity_index, None);
                    unbatchables.dynamic_offsets.push(Some(dynamic_offset));
                }
            }
        }
    }
}

pub fn write_batched_instance_buffer<F: GetBatchData>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_array_buffer: ResMut<GpuArrayBuffer<F::BufferData>>,
) {
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    gpu_array_buffer.write_buffer(&render_device, &render_queue);
    gpu_array_buffer.clear();
}
