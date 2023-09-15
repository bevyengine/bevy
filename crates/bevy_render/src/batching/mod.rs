use bevy_asset::AssetId;
use bevy_ecs::component::Component;
use nonmax::NonMaxU32;

use crate::{
    mesh::Mesh,
    render_phase::{CachedRenderPipelinePhaseItem, DrawFunctionId, RenderPhase},
    render_resource::CachedRenderPipelineId,
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
pub struct BatchMeta<T: PartialEq> {
    /// The pipeline id encompasses all pipeline configuration including vertex
    /// buffers and layouts, shaders and their specializations, bind group
    /// layouts, etc.
    pub pipeline_id: CachedRenderPipelineId,
    /// The draw function id defines the RenderCommands that are called to
    /// set the pipeline and bindings, and make the draw command
    pub draw_function_id: DrawFunctionId,
    pub material_bind_group_id: Option<T>,
    pub mesh_asset_id: AssetId<Mesh>,
    pub dynamic_offset: Option<NonMaxU32>,
}

impl<T: PartialEq> PartialEq for BatchMeta<T> {
    #[inline]
    fn eq(&self, other: &BatchMeta<T>) -> bool {
        self.pipeline_id == other.pipeline_id
            && self.draw_function_id == other.draw_function_id
            && self.mesh_asset_id == other.mesh_asset_id
            && self.dynamic_offset == other.dynamic_offset
            && self.material_bind_group_id == other.material_bind_group_id
    }
}

/// Batch the items in a render phase. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_render_phase<
    I: CachedRenderPipelinePhaseItem,
    T: PartialEq, // Batch metadata used for distinguishing batches
>(
    phase: &mut RenderPhase<I>,
    mut get_batch_meta: impl FnMut(&I) -> Option<(T, NonMaxU32, Option<NonMaxU32>)>,
) {
    let mut items = phase.items.iter_mut().peekable();
    let mut batch_start_item = None;
    let mut batch_start_index = 0;
    let mut next_batch = items.peek().and_then(|item| get_batch_meta(item));
    while let Some(item) = items.next() {
        // Get the current batch meta and update the next batch meta
        let Some((batch_meta, index, dynamic_offset)) = std::mem::replace(
            &mut next_batch,
            items.peek().and_then(|item| get_batch_meta(item)),
        ) else {
            // If the current phase item doesn't match the query, we don't modify it
            continue;
        };

        // If we are beginning a new batch, record the start item and index
        if batch_start_item.is_none() {
            batch_start_item = Some(item);
            batch_start_index = index.get();
        }

        if Some(&batch_meta) != next_batch.as_ref().map(|(meta, ..)| meta) {
            // The next item doesn't match the current batch (or doesn't exist).
            // Update the phase item to render this batch.
            let batch_start_item = batch_start_item.take().unwrap();
            *batch_start_item.batch_range_mut() = batch_start_index..index.get() + 1;
            *batch_start_item.dynamic_offset_mut() = dynamic_offset;
        }
    }
}
