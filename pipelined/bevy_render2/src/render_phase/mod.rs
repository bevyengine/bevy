mod draw;
mod draw_state;

pub use draw::*;
pub use draw_state::*;

use bevy_ecs::prelude::Query;

pub struct RenderPhase<I: PhaseItem> {
    pub items: Vec<I>,
}

impl<I: PhaseItem> Default for RenderPhase<I> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<I: PhaseItem> RenderPhase<I> {
    #[inline]
    pub fn add(&mut self, item: I) {
        self.items.push(item);
    }

    pub fn sort(&mut self) {
        self.items.sort_by_key(|d| d.sort_key());
    }
}

pub fn sort_phase_system<I: PhaseItem>(mut render_phases: Query<&mut RenderPhase<I>>) {
    for mut phase in render_phases.iter_mut() {
        phase.sort();
    }
}
