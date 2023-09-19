use bevy_ecs::{
    component::Component,
    prelude::Res,
    query::{Has, QueryItem, ReadOnlyWorldQuery},
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

pub trait GetBatchData {
    type Query: ReadOnlyWorldQuery;
    type CompareData: PartialEq;
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;
    fn get_batch_data(batch_data: QueryItem<Self::Query>) -> (Self::CompareData, Self::BufferData);
}

/// Batch the items in a render phase. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_and_prepare_render_phase<I: CachedRenderPipelinePhaseItem, F: GetBatchData>(
    gpu_array_buffer: ResMut<GpuArrayBuffer<F::BufferData>>,
    mut views: Query<&mut RenderPhase<I>>,
    query: Query<(Has<NoAutomaticBatching>, F::Query)>,
) {
    let gpu_array_buffer = gpu_array_buffer.into_inner();

    let mut process_item = |item: &mut I| {
        let (no_auto_batching, batch_query_item) = query.get(item.entity()).ok()?;
        let (user_data, buffer_data) = F::get_batch_data(batch_query_item);

        let buffer_index = gpu_array_buffer.push(buffer_data);
        let index = buffer_index.index.get();
        *item.batch_range_mut() = index..index + 1;
        *item.dynamic_offset_mut() = buffer_index.dynamic_offset;

        (!no_auto_batching).then(|| BatchMeta::new(item, user_data))
    };

    for mut phase in &mut views {
        let items = phase.items.iter_mut().map(|i| {
            let batch_data = process_item(i);
            (i.batch_range_mut(), batch_data)
        });
        items.reduce(|(mut start_range, old_batch_meta), (range, batch_meta)| {
            if old_batch_meta == batch_meta && batch_meta.is_some() {
                start_range.end = range.end;
            } else {
                start_range = range;
            }
            (start_range, batch_meta)
        });
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
