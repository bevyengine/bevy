use bevy_ecs::{
    component::Component,
    prelude::Res,
    query::{ReadOnlyWorldQuery, WorldQuery},
    system::{Query, ResMut},
};
use nonmax::NonMaxU32;

use crate::{
    render_phase::{CachedRenderPipelinePhaseItem, DrawFunctionId, RenderPhase},
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
/// - `prepare_and_batch_meshes` is the only system that performs this batching
///   and has sole responsibility for preparing the per-object data. As such
///   the mesh binding and dynamic offsets are assumed to only be variable as a
///   result of the `prepare_and_batch_meshes` system, e.g. due to having to split
///   data across separate uniform bindings within the same buffer due to the
///   maximum uniform buffer binding size.
struct BatchMeta<T: PartialEq> {
    /// The pipeline id encompasses all pipeline configuration including vertex
    /// buffers and layouts, shaders and their specializations, bind group
    /// layouts, etc.
    pub pipeline_id: CachedRenderPipelineId,
    /// The draw function id defines the RenderCommands that are called to
    /// set the pipeline and bindings, and make the draw command
    pub draw_function_id: DrawFunctionId,
    pub dynamic_offset: Option<NonMaxU32>,
    pub user_data: T,
}

impl<T: PartialEq> PartialEq for BatchMeta<T> {
    #[inline]
    fn eq(&self, other: &BatchMeta<T>) -> bool {
        self.pipeline_id == other.pipeline_id
            && self.draw_function_id == other.draw_function_id
            && self.dynamic_offset == other.dynamic_offset
            && self.user_data == other.user_data
    }
}

pub trait GetBatchData {
    type Query: ReadOnlyWorldQuery;
    type CompareData: PartialEq;
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;
    fn get_batch_data(
        batch_data: <Self::Query as WorldQuery>::Item<'_>,
    ) -> (Self::CompareData, Self::BufferData);
}

/// Batch the items in a render phase. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_render_phase<I: CachedRenderPipelinePhaseItem, F: GetBatchData>(
    gpu_array_buffer: ResMut<GpuArrayBuffer<F::BufferData>>,
    mut views: Query<&mut RenderPhase<I>>,
    query: Query<(Option<&NoAutomaticBatching>, F::Query)>,
) {
    let gpu_array_buffer = gpu_array_buffer.into_inner();

    let mut process_item = |item: &mut I| -> Option<BatchMeta<F::CompareData>> {
        let Ok((no_batching, batch_data)) = query.get(item.entity()) else {
            return None;
        };

        let (user_data, buffer_data) = F::get_batch_data(batch_data);

        let buffer_index = gpu_array_buffer.push(buffer_data);
        *item.batch_range_mut() = buffer_index.index.get()..buffer_index.index.get() + 1;
        *item.dynamic_offset_mut() = buffer_index.dynamic_offset;

        if no_batching.is_some() {
            None
        } else {
            Some(BatchMeta {
                pipeline_id: item.cached_pipeline(),
                draw_function_id: item.draw_function(),
                dynamic_offset: buffer_index.dynamic_offset,
                user_data,
            })
        }
    };

    for mut phase in &mut views {
        let mut items = phase.items.iter_mut().peekable();
        let mut batch_start_item = None;
        let mut next_batch = items.peek_mut().and_then(|i| process_item(i));
        while let Some(item) = items.next() {
            // Get the current batch meta and update the next batch meta
            let Some(batch_meta) = std::mem::replace(
                &mut next_batch,
                items.peek_mut().and_then(|i| process_item(i)),
            ) else {
                // If the current phase item doesn't match the query or has NoAutomaticBatching,
                // we don't modify it any further
                continue;
            };

            let batch_end_item = item.batch_range().end;

            // If we are beginning a new batch, record the start item
            if batch_start_item.is_none() {
                batch_start_item = Some(item);
            }

            if Some(&batch_meta) != next_batch.as_ref() {
                // The next item doesn't match the current batch (or doesn't exist).
                // Update the first phase item to render the full batch.
                let batch_start_item = batch_start_item.take().unwrap();
                batch_start_item.batch_range_mut().end = batch_end_item;
            }
        }
    }
}

pub fn flush_buffer<F: GetBatchData>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_array_buffer: ResMut<GpuArrayBuffer<F::BufferData>>,
) {
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    gpu_array_buffer.write_buffer(&render_device, &render_queue);
    gpu_array_buffer.clear();
}
