use crate::{
    render_phase::{CachedRenderPipelinePhaseItem, PhaseItem, RenderPhase},
    render_resource::{GpuArrayBufferIndex, GpuArrayBufferable},
};

struct BatchState<T: BatchMeta<T>, D: GpuArrayBufferable> {
    meta: Option<T>,
    /// The base index in the object data binding's array
    gpu_array_buffer_index: GpuArrayBufferIndex<D>,
    /// The number of entities in the batch
    count: u32,
    item_index: usize,
}

impl<T: BatchMeta<T>, D: GpuArrayBufferable> Default for BatchState<T, D> {
    fn default() -> Self {
        Self {
            meta: Default::default(),
            gpu_array_buffer_index: Default::default(),
            count: Default::default(),
            item_index: Default::default(),
        }
    }
}

fn update_batch_data<I: PhaseItem, T: BatchMeta<T>, D: GpuArrayBufferable>(
    item: &mut I,
    batch: &BatchState<T, D>,
) {
    let BatchState {
        count,
        gpu_array_buffer_index,
        ..
    } = batch;
    let index = gpu_array_buffer_index.index.map_or(0, |index| index.get());
    *item.batch_range_mut() = index..(index + *count);
    *item.dynamic_offset_mut() = gpu_array_buffer_index.dynamic_offset;
}

pub trait BatchMeta<T: BatchMeta<T>> {
    fn matches(&self, other: &T) -> bool;
}

/// Batch the items in a render phase. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_render_phase<
    I: CachedRenderPipelinePhaseItem,
    T: BatchMeta<T>,       // Batch metadata used for distinguishing batches
    D: GpuArrayBufferable, // Per-instance data
>(
    phase: &mut RenderPhase<I>,
    mut get_batch_meta: impl FnMut(&I) -> Option<(T, GpuArrayBufferIndex<D>)>,
) {
    let mut batch = BatchState::<T, D>::default();
    for i in 0..phase.items.len() {
        let item = &phase.items[i];
        let Some((batch_meta, gpu_array_buffer_index)) = get_batch_meta(item) else {
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
            batch.gpu_array_buffer_index = gpu_array_buffer_index;
            batch.count = 0;
            batch.item_index = i;
        }
        batch.count += 1;
    }
    if !phase.items.is_empty() && batch.count > 0 {
        update_batch_data(&mut phase.items[batch.item_index], &batch);
    }
}
