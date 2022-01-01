mod draw;
mod draw_state;

pub use draw::*;
pub use draw_state::*;

use bevy_ecs::prelude::{Component, Query};

/// A resource to collect and sort draw requests for specific [`PhaseItems`](PhaseItem).
#[derive(Component)]
pub struct RenderPhase<I: PhaseItem> {
    pub items: Vec<I>,
}

impl<I: PhaseItem> Default for RenderPhase<I> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<I: PhaseItem> RenderPhase<I> {
    /// Adds a [`PhaseItem`] to this render phase.
    #[inline]
    pub fn add(&mut self, item: I) {
        self.items.push(item);
    }

    /// Sorts all of its [`PhaseItems`](PhaseItem).
    pub fn sort(&mut self) {
        self.items.sort_by_key(|d| d.sort_key());
    }
}

impl<I: BatchedPhaseItem> RenderPhase<I> {
    /// Batches the compatible [`BatchedPhaseItem`]s of this render phase
    pub fn batch(&mut self) {
        // TODO: this could be done in-place
        let mut items = std::mem::take(&mut self.items);
        let mut items = items.drain(..);

        self.items.reserve(items.len());

        // Start the first batch from the first item
        if let Some(mut current_batch) = items.next() {
            // Batch following items until we find an incompatible item
            for next_item in items {
                if matches!(
                    current_batch.add_to_batch(&next_item),
                    BatchResult::IncompatibleItems
                ) {
                    // Store the completed batch, and start a new one from the incompatible item
                    self.items.push(current_batch);
                    current_batch = next_item;
                }
            }
            // Store the last batch
            self.items.push(current_batch);
        }
    }
}

/// This system sorts all [`RenderPhases`](RenderPhase) for the [`PhaseItem`] type.
pub fn sort_phase_system<I: PhaseItem>(mut render_phases: Query<&mut RenderPhase<I>>) {
    for mut phase in render_phases.iter_mut() {
        phase.sort();
    }
}

/// This batches the [`PhaseItem`]s of a [`RenderPhases`](RenderPhase).
pub fn batch_phase_system<I: BatchedPhaseItem>(mut render_phases: Query<&mut RenderPhase<I>>) {
    for mut phase in render_phases.iter_mut() {
        phase.batch();
    }
}
