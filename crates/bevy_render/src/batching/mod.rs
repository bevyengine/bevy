use bevy_ecs::component::Component;
use nonmax::NonMaxU32;

use crate::render_phase::{CachedRenderPipelinePhaseItem, PhaseItem, RenderPhase};

/// Add this component to mesh entities to disable automatic batching
#[derive(Component)]
pub struct NoAutomaticBatching;

pub trait BatchMeta<T: BatchMeta<T>> {
    fn matches(&self, other: &T) -> bool;
}

struct BatchState<T: BatchMeta<T>> {
    meta: Option<T>,
    /// The base index in the object data binding's array
    index: Option<NonMaxU32>,
    /// The dynamic offset of the data binding
    dynamic_offset: Option<NonMaxU32>,
    /// The number of entities in the batch
    count: u32,
    item_index: usize,
}

impl<T: BatchMeta<T>> Default for BatchState<T> {
    fn default() -> Self {
        Self {
            meta: Default::default(),
            index: Default::default(),
            dynamic_offset: Default::default(),
            count: Default::default(),
            item_index: Default::default(),
        }
    }
}

fn update_batch_data<I: PhaseItem, T: BatchMeta<T>>(item: &mut I, batch: &BatchState<T>) {
    let BatchState {
        count,
        index,
        dynamic_offset,
        ..
    } = batch;
    let index = index.map_or(0, |index| index.get());
    *item.batch_range_mut() = index..(index + *count);
    *item.dynamic_offset_mut() = *dynamic_offset;
}

/// Batch the items in a render phase. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_render_phase<
    I: CachedRenderPipelinePhaseItem,
    T: BatchMeta<T>, // Batch metadata used for distinguishing batches
>(
    phase: &mut RenderPhase<I>,
    mut get_batch_meta: impl FnMut(&I) -> Option<(T, Option<NonMaxU32>, Option<NonMaxU32>)>,
) {
    let mut batch = BatchState::<T>::default();
    for i in 0..phase.items.len() {
        let item = &phase.items[i];
        let Some((batch_meta, index, dynamic_offset)) = get_batch_meta(item) else {
            // It is necessary to start a new batch if an entity not matching the query is
            // encountered. This can be achieved by resetting the batch meta.
            batch.meta = None;
            continue;
        };
        if !batch
            .meta
            .as_ref()
            .map_or(false, |meta| meta.matches(&batch_meta))
        {
            if batch.count > 0 {
                update_batch_data(&mut phase.items[batch.item_index], &batch);
            }

            batch.meta = Some(batch_meta);
            batch.index = index;
            batch.dynamic_offset = dynamic_offset;
            batch.count = 0;
            batch.item_index = i;
        }
        batch.count += 1;
    }
    if !phase.items.is_empty() && batch.count > 0 {
        update_batch_data(&mut phase.items[batch.item_index], &batch);
    }
}
