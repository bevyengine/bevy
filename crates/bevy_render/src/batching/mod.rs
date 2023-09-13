use bevy_ecs::component::Component;
use nonmax::NonMaxU32;

use crate::render_phase::{CachedRenderPipelinePhaseItem, RenderPhase};

/// Add this component to mesh entities to disable automatic batching
#[derive(Component)]
pub struct NoAutomaticBatching;

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
        // get current batch meta and update next batch meta
        let Some((batch_meta, index, dynamic_offset)) = std::mem::replace(
            &mut next_batch,
            items.peek().and_then(|item| get_batch_meta(item)),
        ) else {
            // if the current phase item doesn't match the query, we don't modify it
            continue;
        };

        // if we are beginning a new batch record the start item and index 
        if batch_start_item.is_none() {
            batch_start_item = Some(item);
            batch_start_index = index.get();
        }

        if Some(&batch_meta) != next_batch.as_ref().map(|(meta, ..)| meta) {
            // next item doesn't match the current batch (or doesn't exist), 
            // update the phase item to render this batch
            let batch_start_item = batch_start_item.take().unwrap();
            *batch_start_item.batch_range_mut() = batch_start_index..index.get() + 1;
            *batch_start_item.dynamic_offset_mut() = dynamic_offset;
        }
    }
}
