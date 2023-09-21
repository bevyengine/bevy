//! The modular rendering abstraction responsible for queuing, preparing, sorting and drawing
//! entities as part of separate render phases.
//!
//! In Bevy each view (camera, or shadow-casting light, etc.) has one or multiple [`RenderPhase`]s
//! (e.g. opaque, transparent, shadow, etc).
//! They are used to queue entities for rendering.
//! Multiple phases might be required due to different sorting/batching behaviors
//! (e.g. opaque: front to back, transparent: back to front) or because one phase depends on
//! the rendered texture of the previous phase (e.g. for screen-space reflections).
//!
//! To draw an entity, a corresponding [`PhaseItem`] has to be added to one or multiple of these
//! render phases for each view that it is visible in.
//! This must be done in the [`RenderSet::Queue`](crate::RenderSet::Queue).
//! After that the render phase sorts them in the
//! [`RenderSet::PhaseSort`](crate::RenderSet::PhaseSort).
//! Finally the items are rendered using a single [`TrackedRenderPass`], during the
//! [`RenderSet::Render`](crate::RenderSet::Render).
//!
//! Therefore each phase item is assigned a [`Draw`] function.
//! These set up the state of the [`TrackedRenderPass`] (i.e. select the
//! [`RenderPipeline`](crate::render_resource::RenderPipeline), configure the
//! [`BindGroup`](crate::render_resource::BindGroup)s, etc.) and then issue a draw call,
//! for the corresponding item.
//!
//! The [`Draw`] function trait can either be implemented directly or such a function can be
//! created by composing multiple [`RenderCommand`]s.

mod draw;
mod draw_state;
mod rangefinder;

use bevy_utils::nonmax::NonMaxU32;
pub use draw::*;
pub use draw_state::*;
pub use rangefinder::*;

use crate::render_resource::{CachedRenderPipelineId, PipelineCache};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::SRes, SystemParamItem},
};
use std::{ops::Range, slice::SliceIndex};

/// A collection of all rendering instructions, that will be executed by the GPU, for a
/// single render phase for a single view.
///
/// Each view (camera, or shadow-casting light, etc.) can have one or multiple render phases.
/// They are used to queue entities for rendering.
/// Multiple phases might be required due to different sorting/batching behaviors
/// (e.g. opaque: front to back, transparent: back to front) or because one phase depends on
/// the rendered texture of the previous phase (e.g. for screen-space reflections).
/// All [`PhaseItem`]s are then rendered using a single [`TrackedRenderPass`].
/// The render pass might be reused for multiple phases to reduce GPU overhead.
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

    /// An [`Iterator`] through the associated [`Entity`] for each [`PhaseItem`] in order.
    #[inline]
    pub fn iter_entities(&'_ self) -> impl Iterator<Item = Entity> + '_ {
        self.items.iter().map(|item| item.entity())
    }

    /// Renders all of its [`PhaseItem`]s using their corresponding draw functions.
    pub fn render<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) {
        self.render_range(render_pass, world, view, ..);
    }

    /// Renders all [`PhaseItem`]s in the provided `range` (based on their index in `self.items`) using their corresponding draw functions.
    pub fn render_range<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
        range: impl SliceIndex<[I], Output = [I]>,
    ) {
        let items = self
            .items
            .get(range)
            .expect("`Range` provided to `render_range()` is out of bounds");

        let draw_functions = world.resource::<DrawFunctions<I>>();
        let mut draw_functions = draw_functions.write();
        draw_functions.prepare(world);

        let mut index = 0;
        while index < items.len() {
            let item = &items[index];
            let batch_range = item.batch_range();
            if batch_range.is_empty() {
                index += 1;
            } else {
                let draw_function = draw_functions.get_mut(item.draw_function()).unwrap();
                draw_function.draw(world, render_pass, view, item);
                index += batch_range.len();
            }
        }
    }
}

/// An item (entity of the render world) which will be drawn to a texture or the screen,
/// as part of a [`RenderPhase`].
///
/// The data required for rendering an entity is extracted from the main world in the
/// [`ExtractSchedule`](crate::ExtractSchedule).
/// Then it has to be queued up for rendering during the
/// [`RenderSet::Queue`](crate::RenderSet::Queue), by adding a corresponding phase item to
/// a render phase.
/// Afterwards it will be sorted and rendered automatically in the
/// [`RenderSet::PhaseSort`](crate::RenderSet::PhaseSort) and
/// [`RenderSet::Render`](crate::RenderSet::Render), respectively.
pub trait PhaseItem: Sized + Send + Sync + 'static {
    /// The type used for ordering the items. The smallest values are drawn first.
    /// This order can be calculated using the [`ViewRangefinder3d`],
    /// based on the view-space `Z` value of the corresponding view matrix.
    type SortKey: Ord;

    /// The corresponding entity that will be drawn.
    ///
    /// This is used to fetch the render data of the entity, required by the draw function,
    /// from the render world .
    fn entity(&self) -> Entity;

    /// Determines the order in which the items are drawn.
    fn sort_key(&self) -> Self::SortKey;

    /// Specifies the [`Draw`] function used to render the item.
    fn draw_function(&self) -> DrawFunctionId;

    /// Sorts a slice of phase items into render order. Generally if the same type
    /// is batched this should use a stable sort like [`slice::sort_by_key`].
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

    /// The range of instances that the batch covers. After doing a batched draw, batch range
    /// length phase items will be skipped. This design is to avoid having to restructure the
    /// render phase unnecessarily.
    fn batch_range(&self) -> &Range<u32>;
    fn batch_range_mut(&mut self) -> &mut Range<u32>;

    fn dynamic_offset(&self) -> Option<NonMaxU32>;
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32>;
}

/// A [`PhaseItem`] item, that automatically sets the appropriate render pipeline,
/// cached in the [`PipelineCache`].
///
/// You can use the [`SetItemPipeline`] render command to set the pipeline for this item.
pub trait CachedRenderPipelinePhaseItem: PhaseItem {
    /// The id of the render pipeline, cached in the [`PipelineCache`], that will be used to draw
    /// this phase item.
    fn cached_pipeline(&self) -> CachedRenderPipelineId;
}

/// A [`RenderCommand`] that sets the pipeline for the [`CachedRenderPipelinePhaseItem`].
pub struct SetItemPipeline;

impl<P: CachedRenderPipelinePhaseItem> RenderCommand<P> for SetItemPipeline {
    type Param = SRes<PipelineCache>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _entity: (),
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(pipeline) = pipeline_cache
            .into_inner()
            .get_render_pipeline(item.cached_pipeline())
        {
            pass.set_render_pipeline(pipeline);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

/// This system sorts the [`PhaseItem`]s of all [`RenderPhase`]s of this type.
pub fn sort_phase_system<I: PhaseItem>(mut render_phases: Query<&mut RenderPhase<I>>) {
    for mut phase in &mut render_phases {
        phase.sort();
    }
}
