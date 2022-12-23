//! The modular rendering abstractions that queue, prepare, sort and draw entities as part of separate phases.
//!
//! To draw an entity, a corresponding [`PhaseItem`] has to be added to one of the renderers multiple [`RenderPhase`]s (e.g. opaque, transparent, shadow, etc).
//! This must be done in the [`RenderStage::Queue`](crate::RenderStage::Queue).
//! After that the [`RenderPhase`] sorts them in the [`RenderStage::PhaseSort`](crate::RenderStage::PhaseSort).
//! Finally the [`PhaseItem`]s are rendered using a single [`TrackedRenderPass`], during the [`RenderStage::Render`](crate::RenderStage::Render).
//!
//! Therefore each [`PhaseItem`] is assigned a [`Draw`] function.
//! These set up the state of the [`TrackedRenderPass`] (i.e. select the [`RenderPipeline`](crate::render_resource::RenderPipeline), configure the [`BindGroup`](crate::render_resource::BindGroup)s, etc.) and then issue a draw call, for the corresponding [`PhaseItem`].
//!
//! The [`Draw`] function trait can either be implemented directly or such a function can be created by composing multiple [`RenderCommand`]s.

mod draw;
mod draw_state;
mod rangefinder;

use bevy_ecs::entity::Entity;
pub use draw::*;
pub use draw_state::*;
pub use rangefinder::*;

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use std::ops::Range;

/// A render phase sorts and renders all [`PhaseItem`]s (entities) that are assigned to it.
///
/// It corresponds to exactly one [`TrackedRenderPass`] and thus renders all items into the same texture attachments (e.g. color, depth, stencil).
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

    /// Sorts all of its [`PhaseItem`]s.
    pub fn sort(&mut self) {
        I::sort(&mut self.items);
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

        for item in &self.items {
            let draw_function = draw_functions.get_mut(item.draw_function()).unwrap();
            draw_function.draw(world, render_pass, view, item);
        }
    }
}

impl<I: BatchedPhaseItem> RenderPhase<I> {
    /// Batches the compatible [`BatchedPhaseItem`]s of this render phase
    pub fn batch(&mut self) {
        // TODO: this could be done in-place
        let mut items = std::mem::take(&mut self.items).into_iter();

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

/// An item (entity) which will be drawn to the screen.
///
/// A phase item has to be queued up for rendering during the [`RenderStage::Queue`](crate::RenderStage::Queue) stage.
/// Afterwards it will be sorted and rendered automatically in the
/// [`RenderStage::PhaseSort`](crate::RenderStage::PhaseSort) stage and
/// [`RenderStage::Render`](crate::RenderStage::Render) stage, respectively.
pub trait PhaseItem: Sized + Send + Sync + 'static {
    /// The type used for ordering the items. The smallest values are drawn first.
    /// This order can be calculated using the [`ViewRangefinder3d`],
    /// based on the view-space `Z` value of the corresponding view matrix.
    type SortKey: Ord;

    /// The corresponding entity that will be drawn.
    fn entity(&self) -> Entity;

    /// Determines the order in which the items are drawn.
    fn sort_key(&self) -> Self::SortKey;

    /// Specifies the [`Draw`] function used to render the item.
    fn draw_function(&self) -> DrawFunctionId;

    /// Sorts a slice of phase items into render order. Generally if the same type
    /// implements [`BatchedPhaseItem`], this should use a stable sort like [`slice::sort_by_key`].
    /// In almost all other cases, this should not be altered from the default,
    /// which uses a unstable sort, as this provides the best balance of CPU and GPU
    /// performance.
    ///
    /// Implementers can optionally not sort the list at all. This is generally advisable if and
    /// only if the renderer supports a depth prepass, which is by default not supported by
    /// the rest of Bevy's first party rendering crates. Even then, this may have a negative
    /// impact on GPU-side performance due to overdraw.
    ///
    /// It's advised to always profile for performance changes when changing this implementation.
    #[inline]
    fn sort(items: &mut [Self]) {
        items.sort_unstable_by_key(|item| item.sort_key());
    }
}

/// A [`PhaseItem`] that can be batched dynamically.
///
/// Batching is an optimization that regroups multiple items in the same vertex buffer
/// to render them in a single draw call.
///
/// If this is implemented on a type, the implementation of [`PhaseItem::sort`] should
/// be changed to implement a stable sort, or incorrect/suboptimal batching may result.
pub trait BatchedPhaseItem: PhaseItem {
    /// Range in the vertex buffer of this item
    fn batch_range(&self) -> &Option<Range<u32>>;

    /// Range in the vertex buffer of this item
    fn batch_range_mut(&mut self) -> &mut Option<Range<u32>>;

    /// Batches another item within this item if they are compatible.
    /// Items can be batched together if they have the same entity, and consecutive ranges.
    /// If batching is successful, the `other` item should be discarded from the render pass.
    #[inline]
    fn add_to_batch(&mut self, other: &Self) -> BatchResult {
        let self_entity = self.entity();
        if let (Some(self_batch_range), Some(other_batch_range)) = (
            self.batch_range_mut().as_mut(),
            other.batch_range().as_ref(),
        ) {
            // If the items are compatible, join their range into `self`
            if self_entity == other.entity() {
                if self_batch_range.end == other_batch_range.start {
                    self_batch_range.end = other_batch_range.end;
                    return BatchResult::Success;
                } else if self_batch_range.start == other_batch_range.end {
                    self_batch_range.start = other_batch_range.start;
                    return BatchResult::Success;
                }
            }
        }
        BatchResult::IncompatibleItems
    }
}

/// The result of a batching operation.
pub enum BatchResult {
    /// The `other` item was batched into `self`
    Success,
    /// `self` and `other` cannot be batched together
    IncompatibleItems,
}

/// This system sorts the [`PhaseItem`]s of all [`RenderPhase`]s of this type.
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
    use super::*;
    use bevy_ecs::entity::Entity;
    use std::ops::Range;

    #[test]
    fn batching() {
        #[derive(Debug, PartialEq)]
        struct TestPhaseItem {
            entity: Entity,
            batch_range: Option<Range<u32>>,
        }
        impl PhaseItem for TestPhaseItem {
            type SortKey = ();

            fn entity(&self) -> Entity {
                self.entity
            }

            fn sort_key(&self) -> Self::SortKey {}

            fn draw_function(&self) -> DrawFunctionId {
                unimplemented!();
            }
        }
        impl BatchedPhaseItem for TestPhaseItem {
            fn batch_range(&self) -> &Option<Range<u32>> {
                &self.batch_range
            }

            fn batch_range_mut(&mut self) -> &mut Option<Range<u32>> {
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
