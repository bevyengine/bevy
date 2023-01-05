mod draw;
mod draw_state;

use bevy_ecs::entity::Entity;
pub use draw::*;
pub use draw_state::*;

use bevy_ecs::prelude::{Component, Query};
use bevy_ecs::world::World;
use std::cell::Cell;
use thread_local::ThreadLocal;

pub struct RenderPhaseScope<'a, I: PhaseItem> {
    phase_queue: &'a mut Vec<I>,
}

impl<'a, I: PhaseItem> RenderPhaseScope<'a, I> {
    /// Adds a [`PhaseItem`] to this render phase.
    #[inline]
    pub fn add(&mut self, item: I) {
        self.phase_queue.push(item);
    }
}

/// A resource to collect and sort draw requests for specific [`PhaseItems`](PhaseItem).
#[derive(Component)]
pub struct RenderPhase<I: PhaseItem> {
    items: ThreadLocal<Cell<Vec<I>>>,
    pub sorted: Vec<I>,
}

impl<I: PhaseItem> Default for RenderPhase<I> {
    fn default() -> Self {
        Self {
            items: ThreadLocal::new(),
            sorted: Vec::new(),
        }
    }
}

impl<I: PhaseItem> RenderPhase<I> {
    pub fn get(&self) -> &Cell<Vec<I>> {
        self.items.get_or_default()
    }

    pub fn phase_scope(&self, f: impl FnOnce(RenderPhaseScope<'_, I>)) {
        let store = self.get();
        let mut phase_queue = store.take();
        f(RenderPhaseScope {
            phase_queue: &mut phase_queue,
        });
        store.set(phase_queue);
    }

    /// Sorts all of its [`PhaseItems`](PhaseItem).
    pub fn sort(&mut self) {
        self.sorted.clear();
        self.sorted.reserve(
            self.items
                .iter_mut()
                .map(|queue| queue.get_mut().len())
                .sum(),
        );
        for queue in self.items.iter_mut() {
            self.sorted.append(queue.get_mut());
        }
        I::sort(&mut self.sorted);
    }

    pub fn render<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) {
        let draw_functions = world.resource::<DrawFunctions<I>>();
        let mut draw_functions = draw_functions.write();
        draw_functions.prepare(world);

        for item in &self.sorted {
            let draw_function = draw_functions.get_mut(item.draw_function()).unwrap();
            draw_function.draw(world, render_pass, view, item);
        }
    }
}

impl<I: BatchedPhaseItem> RenderPhase<I> {
    /// Batches the compatible [`BatchedPhaseItem`]s of this render phase
    pub fn batch(&mut self) {
        // TODO: this could be done in-place
        let mut items = std::mem::take(&mut self.sorted).into_iter();

        self.sorted.reserve(items.len());

        // Start the first batch from the first item
        if let Some(mut current_batch) = items.next() {
            // Batch following items until we find an incompatible item
            for next_item in items {
                if matches!(
                    current_batch.add_to_batch(&next_item),
                    BatchResult::IncompatibleItems
                ) {
                    // Store the completed batch, and start a new one from the incompatible item
                    self.sorted.push(current_batch);
                    current_batch = next_item;
                }
            }
            // Store the last batch
            self.sorted.push(current_batch);
        }
    }
}

/// This system sorts all [`RenderPhases`](RenderPhase) for the [`PhaseItem`] type.
pub fn sort_phase_system<I: PhaseItem>(mut render_phases: Query<&mut RenderPhase<I>>) {
    for mut phase in &mut render_phases {
        phase.sort();
    }
}

/// This system batches the [`PhaseItem`]s of all [`RenderPhase`]s of this type.
pub fn batch_phase_system<I: BatchedPhaseItem>(mut render_phases: Query<&mut RenderPhase<I>>) {
    for mut phase in &mut render_phases {
        phase.batch();
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use bevy_ecs::entity::Entity;

    use super::*;

    #[test]
    fn batching() {
        #[derive(Debug, PartialEq)]
        struct TestPhaseItem {
            entity: Entity,
            batch_range: Option<Range<u32>>,
        }
        impl PhaseItem for TestPhaseItem {
            type SortKey = ();

            fn entity(&self) -> bevy_ecs::entity::Entity {
                self.entity
            }

            fn sort_key(&self) -> Self::SortKey {}

            fn draw_function(&self) -> DrawFunctionId {
                unimplemented!();
            }
        }
        impl BatchedPhaseItem for TestPhaseItem {
            fn batch_range(&self) -> &Option<std::ops::Range<u32>> {
                &self.batch_range
            }

            fn batch_range_mut(&mut self) -> &mut Option<std::ops::Range<u32>> {
                &mut self.batch_range
            }
        }
        let mut render_phase = RenderPhase::<TestPhaseItem>::default();
        let items = [
            TestPhaseItem {
                entity: Entity::from_raw(0),
                batch_range: Some(0..5),
            },
            // This item should be batched
            TestPhaseItem {
                entity: Entity::from_raw(0),
                batch_range: Some(5..10),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(0..5),
            },
            TestPhaseItem {
                entity: Entity::from_raw(0),
                batch_range: Some(10..15),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(5..10),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: None,
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(10..15),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(20..25),
            },
            // This item should be batched
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(25..30),
            },
            // This item should be batched
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(30..35),
            },
        ];
        for item in items {
            render_phase.add(item);
        }
        render_phase.batch();
        let items_batched = [
            TestPhaseItem {
                entity: Entity::from_raw(0),
                batch_range: Some(0..10),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(0..5),
            },
            TestPhaseItem {
                entity: Entity::from_raw(0),
                batch_range: Some(10..15),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(5..10),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: None,
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(10..15),
            },
            TestPhaseItem {
                entity: Entity::from_raw(1),
                batch_range: Some(20..35),
            },
        ];
        assert_eq!(&*render_phase.items, items_batched);
    }
}
