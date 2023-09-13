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
    // we iterate in reverse so that we can write to the last item of the current batch, and still skip the 
    // right number of phase items when iterating forwards in the Render stage
    let mut items = phase.items.iter_mut().rev().peekable();
    let mut batch_start_index = None;
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

        // record the start index if we are beginning a new batch
        if batch_start_index.is_none() {
            batch_start_index = Some(index);
        }

        if Some(&batch_meta) != next_batch.as_ref().map(|(meta, ..)| meta) {
            // next item doesn't match, update the phase item to render this batch
            *item.batch_range_mut() = batch_start_index.take().unwrap().get()..index.get() + 1;
            *item.dynamic_offset_mut() = dynamic_offset;
            batch_start_index = None;
        }
    }
}
