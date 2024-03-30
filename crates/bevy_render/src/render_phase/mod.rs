//! The modular rendering abstraction responsible for queuing, preparing, sorting and drawing
//! entities as part of separate render phases.
//!
//! In Bevy each view (camera, or shadow-casting light, etc.) has one or multiple render phases
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

use bevy_utils::{default, hashbrown::hash_map::Entry, HashMap};
pub use draw::*;
pub use draw_state::*;
use encase::{internal::WriteInto, ShaderSize};
use nonmax::NonMaxU32;
pub use rangefinder::*;

use crate::render_resource::{
    BufferPoolSlice, CachedRenderPipelineId, GpuArrayBufferIndex, PipelineCache,
};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::SRes, SystemParamItem},
};
use smallvec::SmallVec;
use std::{hash::Hash, ops::Range, slice::SliceIndex};

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
///
/// This flavor of render phase is used for phases in which the ordering is less
/// critical: for example, `Opaque3d`. It's generally faster than the
/// alternative [`SortedRenderPhase`].
#[derive(Component)]
pub struct BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// A list of `BinKey`s for batchable items.
    ///
    /// These are accumulated in `queue_material_meshes` and then sorted in
    /// `batch_and_prepare_binned_render_phase`.
    pub batchable_keys: Vec<BPI::BinKey>,

    /// The batchable bins themselves.
    ///
    /// Each bin corresponds to a single batch set. For unbatchable entities,
    /// prefer `unbatchable_values` instead.
    pub(crate) batchable_values: HashMap<BPI::BinKey, Vec<Entity>>,

    /// A list of `BinKey`s for unbatchable items.
    ///
    /// These are accumulated in `queue_material_meshes` and then sorted in
    /// `batch_and_prepare_binned_render_phase`.
    pub unbatchable_keys: Vec<BPI::BinKey>,

    /// The unbatchable bins.
    ///
    /// Each entity here is rendered in a separate drawcall.
    pub(crate) unbatchable_values: HashMap<BPI::BinKey, UnbatchableBinnedEntities>,

    /// Information on each batch set.
    ///
    /// A *batch set* is a set of entities that will be batched together unless
    /// we're on a platform that doesn't support storage buffers (e.g. WebGL 2)
    /// and differing dynamic uniform indices force us to break batches. On
    /// platforms that support storage buffers, a batch set always consists of
    /// at most one batch.
    ///
    /// The unbatchable entities immediately follow the batches in the storage
    /// buffers.
    pub(crate) batch_sets: Vec<SmallVec<[BinnedRenderPhaseBatch; 1]>>,
}

/// Information about a single batch of entities rendered using binned phase
/// items.
#[derive(Debug)]
pub struct BinnedRenderPhaseBatch {
    /// An entity that's *representative* of this batch.
    ///
    /// Bevy uses this to fetch the mesh. It can be any entity in the batch.
    pub representative_entity: Entity,

    /// The range of instance indices in this batch.
    pub instance_range: Range<u32>,

    /// The dynamic offset of the batch.
    ///
    /// Note that dynamic offsets are only used on platforms that don't support
    /// storage buffers.
    pub dynamic_offset: Option<NonMaxU32>,
}

/// Information about the unbatchable entities in a bin.
pub(crate) struct UnbatchableBinnedEntities {
    /// The entities.
    pub(crate) entities: Vec<Entity>,

    /// The GPU array buffer indices of each unbatchable binned entity.
    pub(crate) buffer_indices: UnbatchableBinnedEntityBufferIndex,
}

/// Stores instance indices and dynamic offsets for unbatchable entities in a
/// binned render phase.
///
/// This is conceptually `Vec<UnbatchableBinnedEntityDynamicOffset>`, but it
/// avoids the overhead of storing dynamic offsets on platforms that support
/// them. In other words, this allows a fast path that avoids allocation on
/// platforms that aren't WebGL 2.
#[derive(Default)]

pub(crate) enum UnbatchableBinnedEntityBufferIndex {
    /// There are no unbatchable entities in this bin (yet).
    #[default]
    NoEntities,

    /// The instances for all unbatchable entities in this bin are contiguous,
    /// and there are no dynamic uniforms.
    ///
    /// This is the typical case on platforms other than WebGL 2. We special
    /// case this to avoid allocation on those platforms.
    NoDynamicOffsets {
        /// The range of indices.
        instance_range: Range<u32>,
    },

    /// Dynamic uniforms are present for unbatchable entities in this bin.
    ///
    /// We fall back to this on WebGL 2.
    DynamicOffsets(Vec<UnbatchableBinnedEntityDynamicOffset>),
}

/// The instance index and dynamic offset (if present) for an unbatchable entity.
///
/// This is only useful on platforms that don't support storage buffers.
#[derive(Clone, Copy)]
pub(crate) struct UnbatchableBinnedEntityDynamicOffset {
    /// The instance index.
    instance_index: u32,
    /// The dynamic offset, if present.
    dynamic_offset: Option<NonMaxU32>,
}

impl<BPI> BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// Bins a new entity.
    ///
    /// `batchable` specifies whether the entity can be batched with other
    /// entities of the same type.
    pub fn add(&mut self, key: BPI::BinKey, entity: Entity, batchable: bool) {
        if batchable {
            match self.batchable_values.entry(key.clone()) {
                Entry::Occupied(mut entry) => entry.get_mut().push(entity),
                Entry::Vacant(entry) => {
                    self.batchable_keys.push(key);
                    entry.insert(vec![entity]);
                }
            }
        } else {
            match self.unbatchable_values.entry(key.clone()) {
                Entry::Occupied(mut entry) => entry.get_mut().entities.push(entity),
                Entry::Vacant(entry) => {
                    self.unbatchable_keys.push(key);
                    entry.insert(UnbatchableBinnedEntities {
                        entities: vec![entity],
                        buffer_indices: default(),
                    });
                }
            }
        }
    }

    /// Encodes the GPU commands needed to render all entities in this phase.
    pub fn render<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) {
        let draw_functions = world.resource::<DrawFunctions<BPI>>();
        let mut draw_functions = draw_functions.write();
        draw_functions.prepare(world);

        // Encode draws for batchables.
        debug_assert_eq!(self.batchable_keys.len(), self.batch_sets.len());
        for (key, batch_set) in self.batchable_keys.iter().zip(self.batch_sets.iter()) {
            for batch in batch_set {
                let binned_phase_item = BPI::new(
                    key.clone(),
                    batch.representative_entity,
                    batch.instance_range.clone(),
                    batch.dynamic_offset,
                );

                // Fetch the draw function.
                let Some(draw_function) = draw_functions.get_mut(binned_phase_item.draw_function())
                else {
                    continue;
                };

                draw_function.draw(world, render_pass, view, &binned_phase_item);
            }
        }

        // Encode draws for unbatchables.

        for key in &self.unbatchable_keys {
            let unbatchable_entities = &self.unbatchable_values[key];
            for (entity_index, &entity) in unbatchable_entities.entities.iter().enumerate() {
                let unbatchable_dynamic_offset = match &unbatchable_entities.buffer_indices {
                    UnbatchableBinnedEntityBufferIndex::NoEntities => {
                        // Shouldn't happen…
                        continue;
                    }
                    UnbatchableBinnedEntityBufferIndex::NoDynamicOffsets { instance_range } => {
                        UnbatchableBinnedEntityDynamicOffset {
                            instance_index: instance_range.start + entity_index as u32,
                            dynamic_offset: None,
                        }
                    }
                    UnbatchableBinnedEntityBufferIndex::DynamicOffsets(ref dynamic_offsets) => {
                        dynamic_offsets[entity_index]
                    }
                };

                let binned_phase_item = BPI::new(
                    key.clone(),
                    entity,
                    unbatchable_dynamic_offset.instance_index
                        ..(unbatchable_dynamic_offset.instance_index + 1),
                    unbatchable_dynamic_offset.dynamic_offset,
                );

                // Fetch the draw function.
                let Some(draw_function) = draw_functions.get_mut(binned_phase_item.draw_function())
                else {
                    continue;
                };

                draw_function.draw(world, render_pass, view, &binned_phase_item);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.batchable_keys.is_empty() && self.unbatchable_keys.is_empty()
    }
}

impl<BPI> Default for BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn default() -> Self {
        Self {
            batchable_keys: vec![],
            batchable_values: HashMap::default(),
            unbatchable_keys: vec![],
            unbatchable_values: HashMap::default(),
            batch_sets: vec![],
        }
    }
}

impl UnbatchableBinnedEntityBufferIndex {
    /// Adds a new entity to the list of unbatchable binned entities.
    pub fn add<T>(&mut self, gpu_array_buffer_index: GpuArrayBufferIndex<T>)
    where
        T: ShaderSize + WriteInto + Clone,
    {
        match (&mut *self, gpu_array_buffer_index.dynamic_offset) {
            (UnbatchableBinnedEntityBufferIndex::NoEntities, None) => {
                // This is the first entity we've seen, and we're not on WebGL
                // 2. Initialize the fast path.
                *self = UnbatchableBinnedEntityBufferIndex::NoDynamicOffsets {
                    instance_range: gpu_array_buffer_index.index
                        ..(gpu_array_buffer_index.index + 1),
                }
            }

            (UnbatchableBinnedEntityBufferIndex::NoEntities, Some(dynamic_offset)) => {
                // This is the first entity we've seen, and we're on WebGL 2.
                // Initialize an array.
                *self = UnbatchableBinnedEntityBufferIndex::DynamicOffsets(vec![
                    UnbatchableBinnedEntityDynamicOffset {
                        instance_index: gpu_array_buffer_index.index,
                        dynamic_offset: Some(dynamic_offset),
                    },
                ]);
            }

            (
                UnbatchableBinnedEntityBufferIndex::NoDynamicOffsets {
                    ref mut instance_range,
                },
                None,
            ) if instance_range.end == gpu_array_buffer_index.index => {
                // This is the normal case on non-WebGL 2.
                instance_range.end += 1;
            }

            (
                UnbatchableBinnedEntityBufferIndex::DynamicOffsets(ref mut offsets),
                dynamic_offset,
            ) => {
                // This is the normal case on WebGL 2.
                offsets.push(UnbatchableBinnedEntityDynamicOffset {
                    instance_index: gpu_array_buffer_index.index,
                    dynamic_offset,
                });
            }

            (
                UnbatchableBinnedEntityBufferIndex::NoDynamicOffsets { instance_range },
                dynamic_offset,
            ) => {
                // We thought we were in non-WebGL 2 mode, but we got a dynamic
                // offset or non-contiguous index anyway. This shouldn't happen,
                // but let's go ahead and do the sensible thing anyhow: demote
                // the compressed `NoDynamicOffsets` field to the full
                // `DynamicOffsets` array.
                let mut new_dynamic_offsets: Vec<_> = instance_range
                    .map(|instance_index| UnbatchableBinnedEntityDynamicOffset {
                        instance_index,
                        dynamic_offset: None,
                    })
                    .collect();
                new_dynamic_offsets.push(UnbatchableBinnedEntityDynamicOffset {
                    instance_index: gpu_array_buffer_index.index,
                    dynamic_offset,
                });
                *self = UnbatchableBinnedEntityBufferIndex::DynamicOffsets(new_dynamic_offsets);
            }
        }
    }
}

/// A collection of all items to be rendered that will be encoded to GPU
/// commands for a single render phase for a single view.
///
/// Each view (camera, or shadow-casting light, etc.) can have one or multiple render phases.
/// They are used to queue entities for rendering.
/// Multiple phases might be required due to different sorting/batching behaviors
/// (e.g. opaque: front to back, transparent: back to front) or because one phase depends on
/// the rendered texture of the previous phase (e.g. for screen-space reflections).
/// All [`PhaseItem`]s are then rendered using a single [`TrackedRenderPass`].
/// The render pass might be reused for multiple phases to reduce GPU overhead.
///
/// This flavor of render phase is used only for meshes that need to be sorted
/// back-to-front, such as transparent meshes. For items that don't need strict
/// sorting, [`BinnedRenderPhase`] is preferred, for performance.
#[derive(Component)]
pub struct SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
    pub items: Vec<I>,
    pub reserved_range: Option<BufferPoolSlice>,
}

impl<I> Default for SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
    fn default() -> Self {
        Self {
            items: Vec::new(),
            reserved_range: None,
        }
    }
}

impl<I> SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
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
/// as part of a render phase.
///
/// The data required for rendering an entity is extracted from the main world in the
/// [`ExtractSchedule`](crate::ExtractSchedule).
/// Then it has to be queued up for rendering during the
/// [`RenderSet::Queue`](crate::RenderSet::Queue), by adding a corresponding phase item to
/// a render phase.
/// Afterwards it will be possibly sorted and rendered automatically in the
/// [`RenderSet::PhaseSort`](crate::RenderSet::PhaseSort) and
/// [`RenderSet::Render`](crate::RenderSet::Render), respectively.
///
/// `PhaseItem`s come in two flavors: [`BinnedPhaseItem`]s and
/// [`SortedPhaseItem`]s.
///
/// * Binned phase items have a `BinKey` which specifies what bin they're to be
/// placed in. All items in the same bin are eligible to be batched together.
/// The `BinKey`s are sorted, but the individual bin items aren't. Binned phase
/// items are good for opaque meshes, in which the order of rendering isn't
/// important. Generally, binned phase items are faster than sorted phase items.
///
/// * Sorted phase items, on the other hand, are placed into one large buffer
/// and then sorted all at once. This is needed for transparent meshes, which
/// have to be sorted back-to-front to render with the painter's algorithm.
/// These types of phase items are generally slower than binned phase items.
pub trait PhaseItem: Sized + Send + Sync + 'static {
    /// Whether or not this `PhaseItem` should be subjected to automatic batching. (Default: `true`)
    const AUTOMATIC_BATCHING: bool = true;

    /// The corresponding entity that will be drawn.
    ///
    /// This is used to fetch the render data of the entity, required by the draw function,
    /// from the render world .
    fn entity(&self) -> Entity;

    /// Specifies the [`Draw`] function used to render the item.
    fn draw_function(&self) -> DrawFunctionId;

    /// The range of instances that the batch covers. After doing a batched draw, batch range
    /// length phase items will be skipped. This design is to avoid having to restructure the
    /// render phase unnecessarily.
    fn batch_range(&self) -> &Range<u32>;
    fn batch_range_mut(&mut self) -> &mut Range<u32>;

    fn dynamic_offset(&self) -> Option<NonMaxU32>;
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32>;
}

/// Represents phase items that are placed into bins. The `BinKey` specifies
/// which bin they're to be placed in. Bin keys are sorted, and items within the
/// same bin are eligible to be batched together. The elements within the bins
/// aren't themselves sorted.
///
/// An example of a binned phase item is `Opaque3d`, for which the rendering
/// order isn't critical.
pub trait BinnedPhaseItem: PhaseItem {
    /// The key used for binning [`PhaseItem`]s into bins. Order the members of
    /// [`BinnedPhaseItem::BinKey`] by the order of binding for best
    /// performance. For example, pipeline id, draw function id, mesh asset id,
    /// lowest variable bind group id such as the material bind group id, and
    /// its dynamic offsets if any, next bind group and offsets, etc. This
    /// reduces the need for rebinding between bins and improves performance.
    type BinKey: Clone + Send + Sync + Eq + Ord + Hash;

    /// Creates a new binned phase item from the key and per-entity data.
    ///
    /// Unlike [`SortedPhaseItem`]s, this is generally called "just in time"
    /// before rendering. The resulting phase item isn't stored in any data
    /// structures, resulting in significant memory savings.
    fn new(
        key: Self::BinKey,
        representative_entity: Entity,
        batch_range: Range<u32>,
        dynamic_offset: Option<NonMaxU32>,
    ) -> Self;
}

/// Represents phase items that must be sorted. The `SortKey` specifies the
/// order that these items are drawn in. These are placed into a single array,
/// and the array as a whole is then sorted.
///
/// An example of a sorted phase item is `Transparent3d`, which must be sorted
/// back to front in order to correctly render with the painter's algorithm.
pub trait SortedPhaseItem: PhaseItem {
    /// The type used for ordering the items. The smallest values are drawn first.
    /// This order can be calculated using the [`ViewRangefinder3d`],
    /// based on the view-space `Z` value of the corresponding view matrix.
    type SortKey: Ord;

    /// Determines the order in which the items are drawn.
    fn sort_key(&self) -> Self::SortKey;

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
    type ViewQuery = ();
    type ItemQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _entity: Option<()>,
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

/// This system sorts the [`PhaseItem`]s of all [`SortedRenderPhase`]s of this
/// type.
pub fn sort_phase_system<I>(mut render_phases: Query<&mut SortedRenderPhase<I>>)
where
    I: SortedPhaseItem,
{
    for mut phase in &mut render_phases {
        phase.sort();
    }
}
