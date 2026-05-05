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
//! This must be done in the [`RenderSystems::Queue`].
//! After that the render phase sorts them in the [`RenderSystems::PhaseSort`].
//! Finally the items are rendered using a single [`TrackedRenderPass`], during
//! the [`RenderSystems::Render`].
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

use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHash;
use bevy_platform::collections::{hash_map::Entry, HashMap};
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};
pub use draw::*;
pub use draw_state::*;
use encase::ShaderType;
use encase::{internal::WriteInto, ShaderSize};
use indexmap::IndexMap;
use nonmax::NonMaxU32;
pub use rangefinder::*;
use wgpu::{BufferUsages, Features};

use crate::batching::gpu_preprocessing::{
    GpuPreprocessingMode, GpuPreprocessingSupport, PhaseBatchedInstanceBuffers,
    PhaseIndirectParametersBuffers,
};
use crate::render_resource::RawBufferVec;
use crate::renderer::RenderDevice;
use crate::sync_world::{MainEntity, MainEntityHashMap};
use crate::view::{ExtractedView, RetainedViewEntity};
use crate::RenderDebugFlags;
use bevy_material::descriptor::CachedRenderPipelineId;

use crate::{
    batching::{
        self,
        gpu_preprocessing::{self, BatchedInstanceBuffers},
        no_gpu_preprocessing::{self, BatchedInstanceBuffer},
        GetFullBatchData,
    },
    render_resource::{GpuArrayBufferIndex, PipelineCache},
    GpuResourceAppExt, Render, RenderApp, RenderSystems,
};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::SRes, SystemParamItem},
};
use bevy_log::warn;
pub use bevy_material::labels::DrawFunctionId;
pub use bevy_material_macros::DrawFunctionLabel;
pub use bevy_material_macros::ShaderLabel;
use bevy_render::renderer::RenderAdapterInfo;
use core::{
    fmt::Debug,
    hash::Hash,
    iter,
    marker::PhantomData,
    mem,
    ops::{Range, RangeBounds},
};
use smallvec::SmallVec;

/// Stores the rendering instructions for a single phase that uses bins in all
/// views.
///
/// They're cleared out every frame, but storing them in a resource like this
/// allows us to reuse allocations.
#[derive(Resource, Deref, DerefMut)]
pub struct ViewBinnedRenderPhases<BPI>(pub HashMap<RetainedViewEntity, BinnedRenderPhase<BPI>>)
where
    BPI: BinnedPhaseItem;

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
pub struct BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// The multidrawable bins.
    ///
    /// Each batch set key maps to a *batch set*, which in this case is a set of
    /// meshes that can be drawn together in one multidraw call. Each batch set
    /// is subdivided into *bins*, each of which represents a particular mesh.
    /// Each bin contains the entity IDs of instances of that mesh.
    ///
    /// So, for example, if there are two cubes and a sphere present in the
    /// scene, we would generally have one batch set containing two bins,
    /// assuming that the cubes and sphere meshes are allocated together and use
    /// the same pipeline. The first bin, corresponding to the cubes, will have
    /// two entities in it. The second bin, corresponding to the sphere, will
    /// have one entity in it.
    pub multidrawable_meshes: IndexMap<BPI::BatchSetKey, RenderMultidrawableBatchSet<BPI>>,

    /// The bins corresponding to batchable items that aren't multidrawable.
    ///
    /// For multidrawable entities, use `multidrawable_meshes`; for
    /// unbatchable entities, use `unbatchable_values`.
    pub batchable_meshes: IndexMap<(BPI::BatchSetKey, BPI::BinKey), RenderBin>,

    /// The unbatchable bins.
    ///
    /// Each entity here is rendered in a separate drawcall.
    pub unbatchable_meshes: IndexMap<(BPI::BatchSetKey, BPI::BinKey), UnbatchableBinnedEntities>,

    /// Items in the bin that aren't meshes at all.
    ///
    /// Bevy itself doesn't place anything in this list, but plugins or your app
    /// can in order to execute custom drawing commands. Draw functions for each
    /// entity are simply called in order at rendering time.
    ///
    /// See the `custom_phase_item` example for an example of how to use this.
    pub non_mesh_items: IndexMap<(BPI::BatchSetKey, BPI::BinKey), NonMeshEntities>,

    /// Information on each batch set.
    ///
    /// A *batch set* is a set of entities that will be batched together unless
    /// we're on a platform that doesn't support storage buffers (e.g. WebGL 2)
    /// and differing dynamic uniform indices force us to break batches. On
    /// platforms that support storage buffers, a batch set always consists of
    /// at most one batch.
    ///
    /// Multidrawable entities come first, then batchable entities, then
    /// unbatchable entities.
    pub(crate) batch_sets: BinnedRenderPhaseBatchSets<BPI::BinKey>,

    /// The batch and bin key for each entity.
    ///
    /// We retain these so that, when the entity changes, the methods that
    /// remove items from bins can quickly find the bin each entity was located
    /// in in order to remove it.
    cached_entity_bin_keys: MainEntityHashMap<CachedBinnedEntity<BPI>>,

    /// The gpu preprocessing mode configured for the view this phase is associated
    /// with.
    gpu_preprocessing_mode: GpuPreprocessingMode,
}

/// All entities that share a mesh and a material and can be batched as part of
/// a [`BinnedRenderPhase`].
#[derive(Default)]
pub struct RenderBin {
    /// A list of the entities in each bin, along with their cached
    /// [`InputUniformIndex`].
    entities: IndexMap<MainEntity, InputUniformIndex, EntityHash>,
}

/// Information about each bin that the [`RenderMultidrawableBatchSet`]
/// maintains on the CPU.
#[derive(Default)]
pub struct RenderMultidrawableBin {
    /// The [`RenderBinnedMeshInstanceIndex`] of each entity in this bin.
    ///
    /// Note that [`RenderBinnedMeshInstanceIndex`]es aren't stable from frame
    /// to frame. They can change as entities are added and removed.
    pub(crate) entity_to_binned_mesh_instance_index:
        MainEntityHashMap<RenderBinnedMeshInstanceIndex>,
}

impl RenderMultidrawableBin {
    /// Creates a new, empty [`RenderMultidrawableBin`].
    fn new() -> RenderMultidrawableBin {
        RenderMultidrawableBin {
            entity_to_binned_mesh_instance_index: HashMap::default(),
        }
    }

    /// Returns true if this bin has no entities in it.
    fn is_empty(&self) -> bool {
        self.entity_to_binned_mesh_instance_index.is_empty()
    }
}

/// The index of a mesh instance in the
/// [`RenderMultidrawableBatchSetGpuBuffers::render_binned_mesh_instance_buffer`]
/// and [`RenderMultidrawableBatchSet::render_binned_mesh_instances_cpu`]
/// arrays.
///
/// These two arrays are parallel and always have the same length.
///
/// These binned mesh instance indices aren't stable from frame to frame; they
/// can change as entities are added and removed from bins. To reference a mesh
/// instance in a stable manner, simply use [`MainEntity`].
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub(crate) struct RenderBinnedMeshInstanceIndex(pub(crate) u32);

/// The GPU buffers that go along with [`RenderMultidrawableBatchSet`].
///
/// The bin unpacking shader uses these in order to produce
/// `PreprocessWorkItem`s.
///
/// See the diagram in [`RenderMultidrawableBatchSet`] for a visual explanation
/// of this data structure.
pub struct RenderMultidrawableBatchSetGpuBuffers {
    /// A mapping from each binned mesh instance
    /// (`RenderBinnedMeshInstanceIndex`) to its input uniform index
    /// ([`InputUniformIndex`]) and bin index (`RenderBinIndex`).
    pub render_binned_mesh_instance_buffer: RawBufferVec<GpuRenderBinnedMeshInstance>,
    /// A mapping from each `RenderBinnedMeshInstanceIndex` to the offset of its
    /// indirect draw parameters.
    pub bin_index_to_indirect_parameters_offset_buffer: RawBufferVec<u32>,
}

/// Information about each binned mesh instance that the
/// [`RenderMultidrawableBatchSet`] keeps on CPU.
#[derive(Clone, Copy)]
pub(crate) struct CpuRenderBinnedMeshInstance {
    /// The entity associated with this mesh instance.
    pub(crate) main_entity: MainEntity,

    /// The index of the bin that the entity is in.
    ///
    /// Note that bin indices are stable from frame to frame.
    bin_index: RenderBinIndex,
}

impl Default for CpuRenderBinnedMeshInstance {
    fn default() -> Self {
        CpuRenderBinnedMeshInstance {
            main_entity: MainEntity::from(Entity::PLACEHOLDER),
            bin_index: RenderBinIndex::default(),
        }
    }
}

impl RenderMultidrawableBatchSetGpuBuffers {
    /// Creates a new set of GPU buffers for a multidrawable batch set.
    fn new() -> RenderMultidrawableBatchSetGpuBuffers {
        let mut render_bin_entry_buffer = RawBufferVec::new(BufferUsages::STORAGE);
        render_bin_entry_buffer.set_label(Some("render bin entry buffer"));
        let mut bin_index_to_indirect_parameters_offset_buffer =
            RawBufferVec::new(BufferUsages::STORAGE);
        bin_index_to_indirect_parameters_offset_buffer
            .set_label(Some("bin index to indirect parameters offset buffer"));

        RenderMultidrawableBatchSetGpuBuffers {
            render_binned_mesh_instance_buffer: render_bin_entry_buffer,
            bin_index_to_indirect_parameters_offset_buffer,
        }
    }

    /// Inserts an entity into the GPU buffers.
    fn insert(
        &mut self,
        bin: &mut RenderMultidrawableBin,
        cpu_binned_mesh_instance_buffer: &mut Vec<CpuRenderBinnedMeshInstance>,
        main_entity: MainEntity,
        input_uniform_index: InputUniformIndex,
        bin_index: RenderBinIndex,
    ) {
        // Creates a `GpuRenderBinnedMeshInstance`.
        let gpu_render_bin_entry = GpuRenderBinnedMeshInstance {
            input_uniform_index: input_uniform_index.0,
            bin_index: bin_index.0,
        };

        // Fetch the index of this entity in the
        // `render_binned_mesh_instance_buffer`. If there isn't one, then
        // allocate one.
        let render_binned_mesh_instance_buffer_index =
            match bin.entity_to_binned_mesh_instance_index.entry(main_entity) {
                Entry::Occupied(occupied_entry) => *occupied_entry.get(),
                Entry::Vacant(vacant_entry) => {
                    let render_bin_buffer_index = RenderBinnedMeshInstanceIndex(
                        self.render_binned_mesh_instance_buffer
                            .push(GpuRenderBinnedMeshInstance::default())
                            as u32,
                    );
                    cpu_binned_mesh_instance_buffer.push(CpuRenderBinnedMeshInstance::default());
                    vacant_entry.insert(render_bin_buffer_index);
                    render_bin_buffer_index
                }
            };

        // Place the entry in the instance buffer at the proper spot. Also, save
        // the entity and bin index in the CPU-side array.
        self.render_binned_mesh_instance_buffer.values_mut()
            [render_binned_mesh_instance_buffer_index.0 as usize] = gpu_render_bin_entry;
        cpu_binned_mesh_instance_buffer[render_binned_mesh_instance_buffer_index.0 as usize] =
            CpuRenderBinnedMeshInstance {
                main_entity,
                bin_index,
            };

        // The GPU-side `render_binned_mesh_instance_buffer` and the CPU-side
        // `cpu_binned_mesh_instance_buffer` are parallel arrays and must have
        // the same length, so assert that in debug mode.
        debug_assert_eq!(
            self.render_binned_mesh_instance_buffer.len(),
            cpu_binned_mesh_instance_buffer.len()
        );
    }

    /// Removes an entity from a bin.
    ///
    /// The entity must be present in the bin, or a panic will occur.
    ///
    /// Because binned mesh instances are tightly packed in the buffers, we use
    /// `swap_remove`, which swaps the last element to fill the place of the
    /// entity that was removed. This might change the
    /// [`RenderBinnedMeshInstanceIndex`] of some *other* entity, requiring the
    /// caller to perform additional bookkeeping. This method returns the index
    /// of the displaced entity, if there was one.
    #[must_use]
    fn remove(
        &mut self,
        bin: &mut RenderMultidrawableBin,
        cpu_binned_mesh_instance_buffer: &mut Vec<CpuRenderBinnedMeshInstance>,
        entity_to_remove: MainEntity,
    ) -> Option<(RenderBinnedMeshInstanceIndex, CpuRenderBinnedMeshInstance)> {
        // Remove the entity from the `entity_to_binned_mesh_instance_index`
        // map.
        let old_index = bin
            .entity_to_binned_mesh_instance_index
            .remove(&entity_to_remove)
            .expect("Entity not in bin");

        // Remove the entity from the reverse
        // `render_binned_mesh_instance_buffer` list, as well
        // as the parallel `render_binned_mesh_instance_buffer`.  Because binned
        // mesh instance indices must be contiguous, this requires use of
        // `swap_remove`.
        cpu_binned_mesh_instance_buffer.swap_remove(old_index.0 as usize);
        self.render_binned_mesh_instance_buffer
            .swap_remove(old_index.0 as usize);

        // Both `render_binned_mesh_instance_buffer` and
        // `cpu_binned_mesh_instance_buffer` must be parallel arrays, so assert
        // that they have the same length.
        debug_assert_eq!(
            cpu_binned_mesh_instance_buffer.len(),
            self.render_binned_mesh_instance_buffer.len()
        );

        // If an entity was displaced (i.e. has a new binned mesh instance index
        // now), then return that to the caller so that they can perform
        // whatever bookkeeping is necessary.
        cpu_binned_mesh_instance_buffer
            .get(old_index.0 as usize)
            .map(|entity_indices| (old_index, *entity_indices))
    }
}

/// The index of a bin in a [`RenderMultidrawableBatchSet`].
///
/// This bin index is stable from frame to frame for bins that have at least one
/// mesh instance in them, though it can be reused if bins are deleted.
#[derive(Clone, Copy, Default, PartialEq, Debug, Pod, Zeroable, Deref, DerefMut)]
#[repr(transparent)]
pub(crate) struct RenderBinIndex(pub(crate) u32);

/// A collection of mesh instances that can be drawn together, sorted into bins.
///
/// This data structure stores a list of entity indices corresponding to mesh
/// instances, along with the bins they live in. Each bin contains the offset of
/// the indirect parameters needed to draw that bin.
///
/// This data structure consists of both CPU and GPU parts. A schematic diagram
/// of the data structure is as follows:
///
/// ```text
///         ┌─
///         │                ─────┬──────────────┬─────
///         │                     │ Mesh Inst. 2 │
///         │  Binned Mesh    ... ├──────────────┤ ...
///         │  Instances          │ Entity 8     │
///         │                ─────┴───┬──────────┴─────
///         │                         │
///         │                         │   ┌───────────────────────────────┐
///         │                         │   │                               │
///         │                         ▼   ▼                               │
///         │               ┌───────┬───────┬───────┬─────                │
///         │  Bins         │ Bin 0 │ Bin 1 │ Bin 2 │ ...                 │
///         │               └───────┴───┬───┴───────┴─────                │
///         │                           │                                 │
///     CPU │                           │                                 │
///         │  Entity-to-               │  ┌──────────┬──────────┬─────   │
///         │  Binned-Mesh-             └─►│ Entity 3 │ Entity 8 │ ...    │
///         │  Instance-                   └──────────┴──────┬───┴─────   │
///         │  Index                                         │            │
///         │                                                │            │
///         │                                                │            │
///         │                                                │            │
///         │                                                │            │
///         │  Indirect-     ┌───────┬───────┬───────┬─────  │            │
///         │  Parameters-   │ IPO 0 │ IPO 1 │ IPO 2 │ ...   │            │
///         │  Offset-to-    └───────┴───────┴───────┴─────  │            │
///         │  Bin-Index                         ▲           │            │
///         │                                    │           │            │
///         └─                                   │           │            │
///                                      ┌───────┘           │            │
///         ┌─                           │                   │            │
///         │                            │                   │            │
///         │  Bin-to-                   ▼                   │            │
///         │  Indirect-     ┌───────┬───────┬───────┬─────  │            │
///         │  Parameters-   │ Bin 0 │ Bin 1 │ Bin 2 │ ...   │            │
///         │  Offset        └───────┴───────┴───────┴─────  │            │
///     GPU │  Buffer                                        │            │
///         │                                                │            │
///         │                                                ▼            │
///         │  Binned Mesh   ─────┬──────────────┬──────────────┬─────    │
///         │  Instance       ... │ Mesh Inst. 1 │ Mesh Inst. 2 │ ...     │
///         │  Buffer        ─────┴──────────────┴───────────┬──┴─────    │
///         │                                                │            │
///         └─                                               └────────────┘
/// ```
pub struct RenderMultidrawableBatchSet<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// The GPU buffers that store the instances in this batch set.
    pub(crate) gpu_buffers: RenderMultidrawableBatchSetGpuBuffers,

    /// A mapping from the phase item's bin key to the index of the
    /// corresponding bin.
    pub(crate) bin_key_to_bin_index: HashMap<BPI::BinKey, RenderBinIndex>,

    /// The actual entities within each bin, indexed by [`RenderBinIndex`].
    ///
    /// This list isn't tightly packed.
    bins: Vec<Option<RenderMultidrawableBin>>,

    /// A list of unused [`RenderBinIndex`]es waiting to be reused.
    ///
    /// Each [`RenderBinIndex`] in this list corresponds to an empty bin.
    bin_free_list: Vec<RenderBinIndex>,

    /// A mapping from the indirect parameters offset to the index of each bin.
    ///
    /// The *indirect parameters offset* is the index of the GPU indirect draw
    /// command for the bin, relative to the first such index for this batch
    /// set.
    indirect_parameters_offset_to_bin_index: Vec<RenderBinIndex>,

    /// Information about each binned mesh instance kept on CPU.
    pub(crate) render_binned_mesh_instances_cpu: Vec<CpuRenderBinnedMeshInstance>,
}

impl<BPI> RenderMultidrawableBatchSet<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// Creates a new [`RenderMultidrawableBatchSet`] containing an empty set of
    /// bins.
    fn new() -> RenderMultidrawableBatchSet<BPI> {
        RenderMultidrawableBatchSet {
            gpu_buffers: RenderMultidrawableBatchSetGpuBuffers::new(),
            bin_key_to_bin_index: HashMap::default(),
            bins: vec![],
            bin_free_list: vec![],
            indirect_parameters_offset_to_bin_index: vec![],
            render_binned_mesh_instances_cpu: vec![],
        }
    }

    /// Returns the first entity in the first bin (if there is one).
    pub(crate) fn representative_entity(&self) -> Option<MainEntity> {
        let first_bin_index = self.bin_key_to_bin_index.values().next()?;
        let first_bin = self.bin(*first_bin_index).expect("Bin should be present");
        first_bin
            .entity_to_binned_mesh_instance_index
            .keys()
            .next()
            .copied()
    }

    /// Returns the [`RenderMultidrawableBin`] for the given [`RenderBinIndex`].
    pub(crate) fn bin(&self, bin_index: RenderBinIndex) -> Option<&RenderMultidrawableBin> {
        self.bins
            .get(bin_index.0 as usize)
            .and_then(|bin| bin.as_ref())
    }

    /// Inserts an entity with the given uniform index into the bin with the
    /// given key.
    fn insert(
        &mut self,
        bin_key: BPI::BinKey,
        main_entity: MainEntity,
        input_uniform_index: InputUniformIndex,
    ) {
        let bin_index;
        match self.bin_key_to_bin_index.entry(bin_key) {
            Entry::Occupied(occupied_entry) => {
                bin_index = *occupied_entry.get();
            }
            Entry::Vacant(vacant_entry) => {
                // Create a bin. First, allocate a bin index.
                bin_index = self
                    .bin_free_list
                    .pop()
                    .unwrap_or(RenderBinIndex(self.bins.len() as u32));

                // Initialize the bin at that index.
                if bin_index.0 as usize == self.bins.len() {
                    self.bins.push(Some(RenderMultidrawableBin::new()));
                } else {
                    debug_assert!(self.bins[bin_index.0 as usize].is_none());
                    self.bins[bin_index.0 as usize] = Some(RenderMultidrawableBin::new());
                }
                vacant_entry.insert(bin_index);

                // Grab an indirect parameters offset.
                self.allocate_indirect_parameters(bin_index);
            }
        }

        // Update the GPU buffers.
        let bin = self.bins[bin_index.0 as usize].as_mut().unwrap();
        self.gpu_buffers.insert(
            bin,
            &mut self.render_binned_mesh_instances_cpu,
            main_entity,
            input_uniform_index,
            bin_index,
        );
    }

    /// Removes the given entity from the bin with the given key.
    ///
    /// The given entity must be present in that bin.
    fn remove(&mut self, main_entity: MainEntity, bin_key: &BPI::BinKey) {
        // Fetch the bin index.
        let bin_index = *self
            .bin_key_to_bin_index
            .get(bin_key)
            .expect("Bin key not present");
        let bin = self.bins[bin_index.0 as usize].as_mut().unwrap();

        let maybe_displaced_entity_indices =
            self.gpu_buffers
                .remove(bin, &mut self.render_binned_mesh_instances_cpu, main_entity);
        if let Some((old_render_bin_buffer_index, displaced_entity_indices)) =
            maybe_displaced_entity_indices
        {
            self.bins[displaced_entity_indices.bin_index.0 as usize]
                .as_mut()
                .expect("Bin not present")
                .entity_to_binned_mesh_instance_index
                .insert(
                    displaced_entity_indices.main_entity,
                    old_render_bin_buffer_index,
                );
        }

        self.remove_bin_if_empty(bin_key, bin_index);
    }

    /// Allocates an indirect parameters slot for a new bin.
    fn allocate_indirect_parameters(&mut self, bin_index: RenderBinIndex) {
        // Indirect parameters must be tightly packed, so we always add one to
        // the end of the list. Record the bin index for the new indirect
        // parameters offset.
        let indirect_parameters_offset = self.indirect_parameters_offset_to_bin_index.len() as u32;
        self.indirect_parameters_offset_to_bin_index.push(bin_index);

        // Update the reverse mapping from bin index to indirect parameters offset.
        if bin_index.0 as usize
            == self
                .gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .len()
        {
            self.gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .push(indirect_parameters_offset);
        } else {
            self.gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .values_mut()[bin_index.0 as usize] = indirect_parameters_offset;
        }
    }

    /// A helper method that removes a bin if it just became empty.
    fn remove_bin_if_empty(&mut self, bin_key: &BPI::BinKey, bin_index: RenderBinIndex) {
        // Is the bin empty? If not, bail.
        let bin = self.bins[bin_index.0 as usize].as_mut().unwrap();
        if !bin.is_empty() {
            return;
        }

        // Remove the bin.
        self.bin_key_to_bin_index.remove(bin_key);
        self.bin_free_list.push(bin_index);
        self.bins[bin_index.0 as usize] = None;

        // Remove the indirect parameters offset corresponding to the bin. Note
        // that indirect parameters must be tightly packed. Thus we must use
        // `swap_remove`.
        let indirect_parameters_offset = mem::replace(
            &mut self
                .gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .values_mut()[bin_index.0 as usize],
            u32::MAX,
        );
        let removed_bin_index = self
            .indirect_parameters_offset_to_bin_index
            .swap_remove(indirect_parameters_offset as usize);
        debug_assert_eq!(bin_index, removed_bin_index);

        // `swap_remove` may have changed the indirect parameter index of some
        // other bin (specifically, the one that was previously at the end of
        // the `Self::indirect_parameters_offset_to_bin_index` list). If it did,
        // then we need to update the
        // `Self::bin_index_to_indirect_parameters_offset_buffer` table to
        // reflect the new offset of that displaced bin.
        if let Some(displaced_bin_index) = self
            .indirect_parameters_offset_to_bin_index
            .get(indirect_parameters_offset as usize)
        {
            self.gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .set(displaced_bin_index.0, indirect_parameters_offset);
        }
    }

    fn is_empty(&self) -> bool {
        self.bin_free_list.len() == self.bins.len()
    }

    pub(crate) fn bin_count(&self) -> usize {
        self.bin_key_to_bin_index.len()
    }
}

/// A single mesh instance in a bin.
///
/// This is a data structure shared between CPU and GPU. It is *not* sorted in
/// the [`RenderMultidrawableBatchSetGpuBuffers`]: mesh instances in any given
/// bin are not guaranteed to be adjacent.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct GpuRenderBinnedMeshInstance {
    /// The index of the `MeshInputUniform` in the buffer.
    ///
    /// This should be an [`InputUniformIndex`], but `encase` doesn't support
    /// newtype structs.
    pub(crate) input_uniform_index: u32,

    /// The index of the bin in this batch set.
    ///
    /// This is the index tracked by [`RenderMultidrawableBatchSet::bins`].
    ///
    /// This should be a [`RenderBinIndex`], but `encase` doesn't support that
    bin_index: u32,
}

/// Information that we keep about an entity currently within a bin.
pub struct CachedBinnedEntity<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// Information that we use to identify a cached entity in a bin.
    pub cached_bin_key: Option<CachedBinKey<BPI>>,
}

/// Information that we use to identify a cached entity in a bin.
pub struct CachedBinKey<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// The key of the batch set containing the entity.
    pub batch_set_key: BPI::BatchSetKey,
    /// The key of the bin containing the entity.
    pub bin_key: BPI::BinKey,
    /// The type of render phase that we use to render the entity: multidraw,
    /// plain batch, etc.
    pub phase_type: BinnedRenderPhaseType,
}

impl<BPI> Clone for CachedBinnedEntity<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn clone(&self) -> Self {
        CachedBinnedEntity {
            cached_bin_key: self.cached_bin_key.clone(),
        }
    }
}

impl<BPI> Clone for CachedBinKey<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn clone(&self) -> Self {
        CachedBinKey {
            batch_set_key: self.batch_set_key.clone(),
            bin_key: self.bin_key.clone(),
            phase_type: self.phase_type,
        }
    }
}

impl<BPI> PartialEq for CachedBinKey<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn eq(&self, other: &Self) -> bool {
        self.batch_set_key == other.batch_set_key
            && self.bin_key == other.bin_key
            && self.phase_type == other.phase_type
    }
}

/// How we store and render the batch sets.
///
/// Each one of these corresponds to a [`GpuPreprocessingMode`].
pub enum BinnedRenderPhaseBatchSets<BK> {
    /// Batches are grouped into batch sets based on dynamic uniforms.
    ///
    /// This corresponds to [`GpuPreprocessingMode::None`].
    DynamicUniforms(Vec<SmallVec<[BinnedRenderPhaseBatch; 1]>>),

    /// Batches are never grouped into batch sets.
    ///
    /// This corresponds to [`GpuPreprocessingMode::PreprocessingOnly`].
    Direct(Vec<BinnedRenderPhaseBatch>),

    /// Batches are grouped together into batch sets based on their ability to
    /// be multi-drawn together.
    ///
    /// This corresponds to [`GpuPreprocessingMode::Culling`].
    MultidrawIndirect(Vec<BinnedRenderPhaseBatchSet<BK>>),
}

/// A group of entities that will be batched together into a single multi-draw
/// call.
pub struct BinnedRenderPhaseBatchSet<BK> {
    /// The first batch in this batch set.
    pub(crate) first_batch: BinnedRenderPhaseBatch,
    /// The key of the bin that the first batch corresponds to.
    pub(crate) bin_key: BK,
    /// The number of batches.
    pub(crate) batch_count: u32,
    /// The index of the batch set in the GPU buffer.
    pub(crate) index: u32,
    /// The index of the first preprocessing work item for this batch set in the
    /// preprocessing work item buffer.
    pub(crate) first_work_item_index: u32,
}

impl<BK> BinnedRenderPhaseBatchSets<BK> {
    fn clear(&mut self) {
        match *self {
            BinnedRenderPhaseBatchSets::DynamicUniforms(ref mut vec) => vec.clear(),
            BinnedRenderPhaseBatchSets::Direct(ref mut vec) => vec.clear(),
            BinnedRenderPhaseBatchSets::MultidrawIndirect(ref mut vec) => vec.clear(),
        }
    }
}

/// Information about a single batch of entities rendered using binned phase
/// items.
#[derive(Debug)]
pub struct BinnedRenderPhaseBatch {
    /// An entity that's *representative* of this batch.
    ///
    /// Bevy uses this to fetch the mesh. It can be any entity in the batch.
    pub representative_entity: (Entity, MainEntity),
    /// The range of instance indices in this batch.
    pub instance_range: Range<u32>,

    /// The dynamic offset of the batch.
    ///
    /// Note that dynamic offsets are only used on platforms that don't support
    /// storage buffers.
    pub extra_index: PhaseItemExtraIndex,
}

/// Information about the unbatchable entities in a bin.
pub struct UnbatchableBinnedEntities {
    /// The entities.
    pub entities: MainEntityHashMap<Entity>,

    /// The GPU array buffer indices of each unbatchable binned entity.
    pub(crate) buffer_indices: UnbatchableBinnedEntityIndexSet,
}

/// Information about [`BinnedRenderPhaseType::NonMesh`] entities.
pub struct NonMeshEntities {
    /// The entities.
    pub entities: MainEntityHashMap<Entity>,
}

/// Stores instance indices and dynamic offsets for unbatchable entities in a
/// binned render phase.
///
/// This is conceptually `Vec<UnbatchableBinnedEntityDynamicOffset>`, but it
/// avoids the overhead of storing dynamic offsets on platforms that support
/// them. In other words, this allows a fast path that avoids allocation on
/// platforms that aren't WebGL 2.
#[derive(Default)]

pub(crate) enum UnbatchableBinnedEntityIndexSet {
    /// There are no unbatchable entities in this bin (yet).
    #[default]
    NoEntities,

    /// The instances for all unbatchable entities in this bin are contiguous,
    /// and there are no dynamic uniforms.
    ///
    /// This is the typical case on platforms other than WebGL 2. We special
    /// case this to avoid allocation on those platforms.
    Sparse {
        /// The range of indices.
        instance_range: Range<u32>,
        /// The index of the first indirect instance parameters.
        ///
        /// The other indices immediately follow these.
        first_indirect_parameters_index: Option<NonMaxU32>,
    },

    /// Dynamic uniforms are present for unbatchable entities in this bin.
    ///
    /// We fall back to this on WebGL 2.
    Dense(Vec<UnbatchableBinnedEntityIndices>),
}

/// The instance index and dynamic offset (if present) for an unbatchable entity.
///
/// This is only useful on platforms that don't support storage buffers.
#[derive(Clone)]
pub(crate) struct UnbatchableBinnedEntityIndices {
    /// The instance index.
    pub(crate) instance_index: u32,
    /// The [`PhaseItemExtraIndex`], if present.
    pub(crate) extra_index: PhaseItemExtraIndex,
}

/// Identifies the list within [`BinnedRenderPhase`] that a phase item is to be
/// placed in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BinnedRenderPhaseType {
    /// The item is a mesh that's eligible for multi-draw indirect rendering and
    /// can be batched with other meshes of the same type.
    MultidrawableMesh,

    /// The item is a mesh that can be batched with other meshes of the same type and
    /// drawn in a single draw call.
    BatchableMesh,

    /// The item is a mesh that's eligible for indirect rendering, but can't be
    /// batched with other meshes of the same type.
    UnbatchableMesh,

    /// The item isn't a mesh at all.
    ///
    /// Bevy will simply invoke the drawing commands for such items one after
    /// another, with no further processing.
    ///
    /// The engine itself doesn't enqueue any items of this type, but it's
    /// available for use in your application and/or plugins.
    NonMesh,
}

impl<T> From<GpuArrayBufferIndex<T>> for UnbatchableBinnedEntityIndices
where
    T: Clone + ShaderSize + WriteInto,
{
    fn from(value: GpuArrayBufferIndex<T>) -> Self {
        UnbatchableBinnedEntityIndices {
            instance_index: value.index,
            extra_index: PhaseItemExtraIndex::maybe_dynamic_offset(value.dynamic_offset),
        }
    }
}

impl<BPI> Default for ViewBinnedRenderPhases<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn default() -> Self {
        Self(default())
    }
}

impl<BPI> ViewBinnedRenderPhases<BPI>
where
    BPI: BinnedPhaseItem,
{
    pub fn prepare_for_new_frame(
        &mut self,
        retained_view_entity: RetainedViewEntity,
        gpu_preprocessing: GpuPreprocessingMode,
    ) {
        match self.entry(retained_view_entity) {
            Entry::Occupied(mut entry) => entry.get_mut().prepare_for_new_frame(),
            Entry::Vacant(entry) => {
                entry.insert(BinnedRenderPhase::<BPI>::new(gpu_preprocessing));
            }
        }
    }
}

/// The index of the uniform describing this object in the GPU buffer, when GPU
/// preprocessing is enabled.
///
/// For example, for 3D meshes, this is the index of the `MeshInputUniform` in
/// the buffer.
///
/// This field is ignored if GPU preprocessing isn't in use, such as (currently)
/// in the case of 2D meshes. In that case, it can be safely set to
/// [`core::default::Default::default`].
#[derive(Clone, Copy, PartialEq, Default, Deref, DerefMut, Debug, Pod, Zeroable)]
#[repr(transparent)]
pub struct InputUniformIndex(pub u32);

impl<BPI> BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    /// Bins a new entity.
    ///
    /// The `phase_type` parameter specifies whether the entity is a
    /// preprocessable mesh and whether it can be binned with meshes of the same
    /// type.
    pub fn add(
        &mut self,
        batch_set_key: BPI::BatchSetKey,
        bin_key: BPI::BinKey,
        (entity, main_entity): (Entity, MainEntity),
        input_uniform_index: InputUniformIndex,
        mut phase_type: BinnedRenderPhaseType,
    ) {
        // If the user has overridden indirect drawing for this view, we need to
        // force the phase type to be batchable instead.
        if self.gpu_preprocessing_mode == GpuPreprocessingMode::PreprocessingOnly
            && phase_type == BinnedRenderPhaseType::MultidrawableMesh
        {
            phase_type = BinnedRenderPhaseType::BatchableMesh;
        }

        match phase_type {
            BinnedRenderPhaseType::MultidrawableMesh => {
                match self.multidrawable_meshes.entry(batch_set_key.clone()) {
                    indexmap::map::Entry::Occupied(mut entry) => {
                        entry
                            .get_mut()
                            .insert(bin_key.clone(), main_entity, input_uniform_index);
                    }
                    indexmap::map::Entry::Vacant(entry) => {
                        let mut new_batch_set = RenderMultidrawableBatchSet::new();
                        new_batch_set.insert(bin_key.clone(), main_entity, input_uniform_index);
                        entry.insert(new_batch_set);
                    }
                }
            }

            BinnedRenderPhaseType::BatchableMesh => {
                match self
                    .batchable_meshes
                    .entry((batch_set_key.clone(), bin_key.clone()).clone())
                {
                    indexmap::map::Entry::Occupied(mut entry) => {
                        entry.get_mut().insert(main_entity, input_uniform_index);
                    }
                    indexmap::map::Entry::Vacant(entry) => {
                        entry.insert(RenderBin::from_entity(main_entity, input_uniform_index));
                    }
                }
            }

            BinnedRenderPhaseType::UnbatchableMesh => {
                match self
                    .unbatchable_meshes
                    .entry((batch_set_key.clone(), bin_key.clone()))
                {
                    indexmap::map::Entry::Occupied(mut entry) => {
                        entry.get_mut().entities.insert(main_entity, entity);
                    }
                    indexmap::map::Entry::Vacant(entry) => {
                        let mut entities = MainEntityHashMap::default();
                        entities.insert(main_entity, entity);
                        entry.insert(UnbatchableBinnedEntities {
                            entities,
                            buffer_indices: default(),
                        });
                    }
                }
            }

            BinnedRenderPhaseType::NonMesh => {
                // We don't process these items further.
                match self
                    .non_mesh_items
                    .entry((batch_set_key.clone(), bin_key.clone()).clone())
                {
                    indexmap::map::Entry::Occupied(mut entry) => {
                        entry.get_mut().entities.insert(main_entity, entity);
                    }
                    indexmap::map::Entry::Vacant(entry) => {
                        let mut entities = MainEntityHashMap::default();
                        entities.insert(main_entity, entity);
                        entry.insert(NonMeshEntities { entities });
                    }
                }
            }
        }

        // Update the cache.
        self.update_cache(
            main_entity,
            Some(CachedBinKey {
                batch_set_key,
                bin_key,
                phase_type,
            }),
        );
    }

    /// Inserts an entity into the cache with the given change tick.
    pub fn update_cache(
        &mut self,
        main_entity: MainEntity,
        cached_bin_key: Option<CachedBinKey<BPI>>,
    ) {
        let new_cached_binned_entity = CachedBinnedEntity { cached_bin_key };
        self.cached_entity_bin_keys
            .insert(main_entity, new_cached_binned_entity);
    }

    /// Encodes the GPU commands needed to render all entities in this phase.
    pub fn render<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        {
            let draw_functions = world.resource::<DrawFunctions<BPI>>();
            let mut draw_functions = draw_functions.write();
            draw_functions.prepare(world);
            // Make sure to drop the reader-writer lock here to avoid recursive
            // locks.
        }

        self.render_batchable_meshes(render_pass, world, view)?;
        self.render_unbatchable_meshes(render_pass, world, view)?;
        self.render_non_meshes(render_pass, world, view)?;

        Ok(())
    }

    /// Renders all batchable meshes queued in this phase.
    fn render_batchable_meshes<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        let draw_functions = world.resource::<DrawFunctions<BPI>>();
        let mut draw_functions = draw_functions.write();

        let render_device = world.resource::<RenderDevice>();
        let render_adapter_info = world.resource::<RenderAdapterInfo>();
        let multi_draw_indirect_count_supported = render_device
            .features()
            .contains(Features::MULTI_DRAW_INDIRECT_COUNT)
            // TODO: https://github.com/gfx-rs/wgpu/issues/7974
            && !matches!(render_adapter_info.backend, wgpu::Backend::Dx12);

        match self.batch_sets {
            BinnedRenderPhaseBatchSets::DynamicUniforms(ref batch_sets) => {
                debug_assert_eq!(self.batchable_meshes.len(), batch_sets.len());

                for ((batch_set_key, bin_key), batch_set) in
                    self.batchable_meshes.keys().zip(batch_sets.iter())
                {
                    for batch in batch_set {
                        let binned_phase_item = BPI::new(
                            batch_set_key.clone(),
                            bin_key.clone(),
                            batch.representative_entity,
                            batch.instance_range.clone(),
                            batch.extra_index.clone(),
                        );

                        // Fetch the draw function.
                        let Some(draw_function) =
                            draw_functions.get_mut(binned_phase_item.draw_function())
                        else {
                            continue;
                        };

                        draw_function.draw(world, render_pass, view, &binned_phase_item)?;
                    }
                }
            }

            BinnedRenderPhaseBatchSets::Direct(ref batch_set) => {
                for (batch, (batch_set_key, bin_key)) in
                    batch_set.iter().zip(self.batchable_meshes.keys())
                {
                    let binned_phase_item = BPI::new(
                        batch_set_key.clone(),
                        bin_key.clone(),
                        batch.representative_entity,
                        batch.instance_range.clone(),
                        batch.extra_index.clone(),
                    );

                    // Fetch the draw function.
                    let Some(draw_function) =
                        draw_functions.get_mut(binned_phase_item.draw_function())
                    else {
                        continue;
                    };

                    draw_function.draw(world, render_pass, view, &binned_phase_item)?;
                }
            }

            BinnedRenderPhaseBatchSets::MultidrawIndirect(ref batch_sets) => {
                for (batch_set_key, batch_set) in self
                    .multidrawable_meshes
                    .keys()
                    .chain(
                        self.batchable_meshes
                            .keys()
                            .map(|(batch_set_key, _)| batch_set_key),
                    )
                    .zip(batch_sets.iter())
                {
                    let batch = &batch_set.first_batch;

                    let batch_set_index = if multi_draw_indirect_count_supported {
                        NonMaxU32::new(batch_set.index)
                    } else {
                        None
                    };

                    let binned_phase_item = BPI::new(
                        batch_set_key.clone(),
                        batch_set.bin_key.clone(),
                        batch.representative_entity,
                        batch.instance_range.clone(),
                        match batch.extra_index {
                            PhaseItemExtraIndex::None => PhaseItemExtraIndex::None,
                            PhaseItemExtraIndex::DynamicOffset(ref dynamic_offset) => {
                                PhaseItemExtraIndex::DynamicOffset(*dynamic_offset)
                            }
                            PhaseItemExtraIndex::IndirectParametersIndex { ref range, .. } => {
                                PhaseItemExtraIndex::IndirectParametersIndex {
                                    range: range.start..(range.start + batch_set.batch_count),
                                    batch_set_index,
                                }
                            }
                        },
                    );

                    // Fetch the draw function.
                    let Some(draw_function) =
                        draw_functions.get_mut(binned_phase_item.draw_function())
                    else {
                        continue;
                    };

                    draw_function.draw(world, render_pass, view, &binned_phase_item)?;
                }
            }
        }

        Ok(())
    }

    /// Renders all unbatchable meshes queued in this phase.
    fn render_unbatchable_meshes<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        let draw_functions = world.resource::<DrawFunctions<BPI>>();
        let mut draw_functions = draw_functions.write();

        for (batch_set_key, bin_key) in self.unbatchable_meshes.keys() {
            let unbatchable_entities =
                &self.unbatchable_meshes[&(batch_set_key.clone(), bin_key.clone())];
            for (entity_index, entity) in unbatchable_entities.entities.iter().enumerate() {
                let unbatchable_dynamic_offset = match &unbatchable_entities.buffer_indices {
                    UnbatchableBinnedEntityIndexSet::NoEntities => {
                        // Shouldn't happen…
                        continue;
                    }
                    UnbatchableBinnedEntityIndexSet::Sparse {
                        instance_range,
                        first_indirect_parameters_index,
                    } => UnbatchableBinnedEntityIndices {
                        instance_index: instance_range.start + entity_index as u32,
                        extra_index: match first_indirect_parameters_index {
                            None => PhaseItemExtraIndex::None,
                            Some(first_indirect_parameters_index) => {
                                let first_indirect_parameters_index_for_entity =
                                    u32::from(*first_indirect_parameters_index)
                                        + entity_index as u32;
                                PhaseItemExtraIndex::IndirectParametersIndex {
                                    range: first_indirect_parameters_index_for_entity
                                        ..(first_indirect_parameters_index_for_entity + 1),
                                    batch_set_index: None,
                                }
                            }
                        },
                    },
                    UnbatchableBinnedEntityIndexSet::Dense(dynamic_offsets) => {
                        dynamic_offsets[entity_index].clone()
                    }
                };

                let binned_phase_item = BPI::new(
                    batch_set_key.clone(),
                    bin_key.clone(),
                    (*entity.1, *entity.0),
                    unbatchable_dynamic_offset.instance_index
                        ..(unbatchable_dynamic_offset.instance_index + 1),
                    unbatchable_dynamic_offset.extra_index,
                );

                // Fetch the draw function.
                let Some(draw_function) = draw_functions.get_mut(binned_phase_item.draw_function())
                else {
                    continue;
                };

                draw_function.draw(world, render_pass, view, &binned_phase_item)?;
            }
        }
        Ok(())
    }

    /// Renders all objects of type [`BinnedRenderPhaseType::NonMesh`].
    ///
    /// These will have been added by plugins or the application.
    fn render_non_meshes<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        let draw_functions = world.resource::<DrawFunctions<BPI>>();
        let mut draw_functions = draw_functions.write();

        for ((batch_set_key, bin_key), non_mesh_entities) in &self.non_mesh_items {
            for (main_entity, entity) in non_mesh_entities.entities.iter() {
                // Come up with a fake batch range and extra index. The draw
                // function is expected to manage any sort of batching logic itself.
                let binned_phase_item = BPI::new(
                    batch_set_key.clone(),
                    bin_key.clone(),
                    (*entity, *main_entity),
                    0..1,
                    PhaseItemExtraIndex::None,
                );

                let Some(draw_function) = draw_functions.get_mut(binned_phase_item.draw_function())
                else {
                    continue;
                };

                draw_function.draw(world, render_pass, view, &binned_phase_item)?;
            }
        }

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.multidrawable_meshes.is_empty()
            && self.batchable_meshes.is_empty()
            && self.unbatchable_meshes.is_empty()
            && self.non_mesh_items.is_empty()
    }

    pub fn prepare_for_new_frame(&mut self) {
        self.batch_sets.clear();

        for unbatchable_bin in self.unbatchable_meshes.values_mut() {
            unbatchable_bin.buffer_indices.clear();
        }
    }

    /// Removes a single entity from its bin.
    ///
    /// If doing so makes the bin empty, this method removes the bin as well.
    pub fn remove(&mut self, main_entity: MainEntity) {
        let Some(cached_binned_entity) = self.cached_entity_bin_keys.remove(&main_entity) else {
            return;
        };

        if let Some(ref cached_bin_key) = cached_binned_entity.cached_bin_key {
            remove_entity_from_bin(
                main_entity,
                cached_bin_key,
                &mut self.multidrawable_meshes,
                &mut self.batchable_meshes,
                &mut self.unbatchable_meshes,
                &mut self.non_mesh_items,
            );
        }
    }
}

/// Removes an entity from a bin.
///
/// If this makes the bin empty, this function removes the bin as well.
///
/// This is a standalone function instead of a method on [`BinnedRenderPhase`]
/// for borrow check reasons.
fn remove_entity_from_bin<BPI>(
    entity: MainEntity,
    entity_bin_key: &CachedBinKey<BPI>,
    multidrawable_meshes: &mut IndexMap<BPI::BatchSetKey, RenderMultidrawableBatchSet<BPI>>,
    batchable_meshes: &mut IndexMap<(BPI::BatchSetKey, BPI::BinKey), RenderBin>,
    unbatchable_meshes: &mut IndexMap<(BPI::BatchSetKey, BPI::BinKey), UnbatchableBinnedEntities>,
    non_mesh_items: &mut IndexMap<(BPI::BatchSetKey, BPI::BinKey), NonMeshEntities>,
) where
    BPI: BinnedPhaseItem,
{
    match entity_bin_key.phase_type {
        BinnedRenderPhaseType::MultidrawableMesh => {
            if let indexmap::map::Entry::Occupied(mut batch_set_entry) =
                multidrawable_meshes.entry(entity_bin_key.batch_set_key.clone())
            {
                batch_set_entry
                    .get_mut()
                    .remove(entity, &entity_bin_key.bin_key);

                // If the batch set is now empty, remove it. This will perturb
                // the order, but that's OK because we're going to sort the bin
                // afterwards.
                if batch_set_entry.get_mut().is_empty() {
                    batch_set_entry.swap_remove();
                }
            }
        }

        BinnedRenderPhaseType::BatchableMesh => {
            if let indexmap::map::Entry::Occupied(mut bin_entry) = batchable_meshes.entry((
                entity_bin_key.batch_set_key.clone(),
                entity_bin_key.bin_key.clone(),
            )) {
                bin_entry.get_mut().remove(entity);

                // If the bin is now empty, remove the bin.
                if bin_entry.get_mut().is_empty() {
                    bin_entry.swap_remove();
                }
            }
        }

        BinnedRenderPhaseType::UnbatchableMesh => {
            if let indexmap::map::Entry::Occupied(mut bin_entry) = unbatchable_meshes.entry((
                entity_bin_key.batch_set_key.clone(),
                entity_bin_key.bin_key.clone(),
            )) {
                bin_entry.get_mut().entities.remove(&entity);

                // If the bin is now empty, remove the bin.
                if bin_entry.get_mut().entities.is_empty() {
                    bin_entry.swap_remove();
                }
            }
        }

        BinnedRenderPhaseType::NonMesh => {
            if let indexmap::map::Entry::Occupied(mut bin_entry) = non_mesh_items.entry((
                entity_bin_key.batch_set_key.clone(),
                entity_bin_key.bin_key.clone(),
            )) {
                bin_entry.get_mut().entities.remove(&entity);

                // If the bin is now empty, remove the bin.
                if bin_entry.get_mut().entities.is_empty() {
                    bin_entry.swap_remove();
                }
            }
        }
    }
}

impl<BPI> BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem,
{
    fn new(gpu_preprocessing: GpuPreprocessingMode) -> Self {
        Self {
            multidrawable_meshes: IndexMap::default(),
            batchable_meshes: IndexMap::default(),
            unbatchable_meshes: IndexMap::default(),
            non_mesh_items: IndexMap::default(),
            batch_sets: match gpu_preprocessing {
                GpuPreprocessingMode::Culling => {
                    BinnedRenderPhaseBatchSets::MultidrawIndirect(vec![])
                }
                GpuPreprocessingMode::PreprocessingOnly => {
                    BinnedRenderPhaseBatchSets::Direct(vec![])
                }
                GpuPreprocessingMode::None => BinnedRenderPhaseBatchSets::DynamicUniforms(vec![]),
            },
            cached_entity_bin_keys: MainEntityHashMap::default(),
            gpu_preprocessing_mode: gpu_preprocessing,
        }
    }
}

impl UnbatchableBinnedEntityIndexSet {
    /// Returns the [`UnbatchableBinnedEntityIndices`] for the given entity.
    fn indices_for_entity_index(
        &self,
        entity_index: u32,
    ) -> Option<UnbatchableBinnedEntityIndices> {
        match self {
            UnbatchableBinnedEntityIndexSet::NoEntities => None,
            UnbatchableBinnedEntityIndexSet::Sparse { instance_range, .. }
                if entity_index >= instance_range.len() as u32 =>
            {
                None
            }
            UnbatchableBinnedEntityIndexSet::Sparse {
                instance_range,
                first_indirect_parameters_index: None,
            } => Some(UnbatchableBinnedEntityIndices {
                instance_index: instance_range.start + entity_index,
                extra_index: PhaseItemExtraIndex::None,
            }),
            UnbatchableBinnedEntityIndexSet::Sparse {
                instance_range,
                first_indirect_parameters_index: Some(first_indirect_parameters_index),
            } => {
                let first_indirect_parameters_index_for_this_batch =
                    u32::from(*first_indirect_parameters_index) + entity_index;
                Some(UnbatchableBinnedEntityIndices {
                    instance_index: instance_range.start + entity_index,
                    extra_index: PhaseItemExtraIndex::IndirectParametersIndex {
                        range: first_indirect_parameters_index_for_this_batch
                            ..(first_indirect_parameters_index_for_this_batch + 1),
                        batch_set_index: None,
                    },
                })
            }
            UnbatchableBinnedEntityIndexSet::Dense(indices) => {
                indices.get(entity_index as usize).cloned()
            }
        }
    }
}

/// A convenient abstraction for adding all the systems necessary for a binned
/// render phase to the render app.
///
/// This is the version used when the pipeline supports GPU preprocessing: e.g.
/// 3D PBR meshes.
pub struct BinnedRenderPhasePlugin<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
    phantom: PhantomData<(BPI, GFBD)>,
}

impl<BPI, GFBD> BinnedRenderPhasePlugin<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    pub fn new(debug_flags: RenderDebugFlags) -> Self {
        Self {
            debug_flags,
            phantom: PhantomData,
        }
    }
}

impl<BPI, GFBD> Plugin for BinnedRenderPhasePlugin<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData + Sync + Send + 'static,
{
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<ViewBinnedRenderPhases<BPI>>()
            .allow_ambiguous_resource::<ViewBinnedRenderPhases<BPI>>()
            .init_gpu_resource::<PhaseBatchedInstanceBuffers<BPI, GFBD::BufferData>>()
            .init_gpu_resource::<PhaseIndirectParametersBuffers<BPI>>()
            .add_systems(
                Render,
                (
                    batching::sort_binned_render_phase::<BPI>.in_set(RenderSystems::PhaseSort),
                    (
                        no_gpu_preprocessing::batch_and_prepare_binned_render_phase::<BPI, GFBD>
                            .run_if(resource_exists::<BatchedInstanceBuffer<GFBD::BufferData>>),
                        gpu_preprocessing::batch_and_prepare_binned_render_phase::<BPI, GFBD>
                            .run_if(
                                resource_exists::<
                                    BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
                                >,
                            ),
                    )
                        .in_set(RenderSystems::PrepareResourcesBatchPhases)
                        .ambiguous_with(RenderSystems::PrepareResourcesBatchPhases),
                    gpu_preprocessing::write_binned_instance_buffers::<BPI, GFBD>
                        .run_if(
                            resource_exists::<
                                BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
                            >,
                        )
                        .in_set(RenderSystems::PrepareResourcesWritePhaseBuffers)
                        .ambiguous_with(RenderSystems::PrepareResourcesWritePhaseBuffers),
                    gpu_preprocessing::collect_buffers_for_phase::<BPI, GFBD>
                        .run_if(
                            resource_exists::<
                                BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
                            >,
                        )
                        .in_set(RenderSystems::PrepareResourcesCollectPhaseBuffers),
                ),
            );
    }
}

/// Stores the rendering instructions for a single phase that sorts items in all
/// views.
///
/// They're cleared out every frame, but storing them in a resource like this
/// allows us to reuse allocations.
#[derive(Resource, Deref, DerefMut)]
pub struct ViewSortedRenderPhases<SPI>(pub HashMap<RetainedViewEntity, SortedRenderPhase<SPI>>)
where
    SPI: SortedPhaseItem;

impl<SPI> Default for ViewSortedRenderPhases<SPI>
where
    SPI: SortedPhaseItem,
{
    fn default() -> Self {
        Self(default())
    }
}

impl<SPI> ViewSortedRenderPhases<SPI>
where
    SPI: SortedPhaseItem,
{
    /// Ensures that a set of phases are present for the given
    /// [`RetainedViewEntity`].
    pub fn prepare_for_new_frame(&mut self, retained_view_entity: RetainedViewEntity) {
        match self.entry(retained_view_entity) {
            Entry::Occupied(mut entry) => {
                let render_phase = entry.get_mut();
                for (render_entity, main_entity) in render_phase.transient_items.drain(..) {
                    render_phase
                        .items
                        .swap_remove(&(render_entity, main_entity));
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(default());
            }
        }
    }
}

/// A convenient abstraction for adding all the systems necessary for a sorted
/// render phase to the render app.
///
/// This is the version used when the pipeline supports GPU preprocessing: e.g.
/// 3D PBR meshes.
pub struct SortedRenderPhasePlugin<SPI, GFBD>
where
    SPI: SortedPhaseItem,
    GFBD: GetFullBatchData,
{
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
    phantom: PhantomData<(SPI, GFBD)>,
}

impl<SPI, GFBD> SortedRenderPhasePlugin<SPI, GFBD>
where
    SPI: SortedPhaseItem,
    GFBD: GetFullBatchData,
{
    pub fn new(debug_flags: RenderDebugFlags) -> Self {
        Self {
            debug_flags,
            phantom: PhantomData,
        }
    }
}

impl<SPI, GFBD> Plugin for SortedRenderPhasePlugin<SPI, GFBD>
where
    SPI: SortedPhaseItem + CachedRenderPipelinePhaseItem,
    GFBD: GetFullBatchData + Sync + Send + 'static,
{
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<ViewSortedRenderPhases<SPI>>()
            .allow_ambiguous_resource::<ViewSortedRenderPhases<SPI>>()
            .init_gpu_resource::<PhaseBatchedInstanceBuffers<SPI, GFBD::BufferData>>()
            .init_gpu_resource::<PhaseIndirectParametersBuffers<SPI>>()
            .add_systems(
                Render,
                (
                    (
                        no_gpu_preprocessing::batch_and_prepare_sorted_render_phase::<SPI, GFBD>
                            .run_if(resource_exists::<BatchedInstanceBuffer<GFBD::BufferData>>),
                        gpu_preprocessing::batch_and_prepare_sorted_render_phase::<SPI, GFBD>
                            .run_if(
                                resource_exists::<
                                    BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
                                >,
                            ),
                    )
                        .in_set(RenderSystems::PrepareResourcesBatchPhases),
                    gpu_preprocessing::collect_buffers_for_phase::<SPI, GFBD>
                        .run_if(
                            resource_exists::<
                                BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
                            >,
                        )
                        .in_set(RenderSystems::PrepareResourcesCollectPhaseBuffers),
                ),
            );
    }
}

impl UnbatchableBinnedEntityIndexSet {
    /// Adds a new entity to the list of unbatchable binned entities.
    pub fn add(&mut self, indices: UnbatchableBinnedEntityIndices) {
        match self {
            UnbatchableBinnedEntityIndexSet::NoEntities => {
                match indices.extra_index {
                    PhaseItemExtraIndex::DynamicOffset(_) => {
                        // This is the first entity we've seen, and we don't have
                        // compute shaders. Initialize an array.
                        *self = UnbatchableBinnedEntityIndexSet::Dense(vec![indices]);
                    }
                    PhaseItemExtraIndex::None => {
                        // This is the first entity we've seen, and we have compute
                        // shaders. Initialize the fast path.
                        *self = UnbatchableBinnedEntityIndexSet::Sparse {
                            instance_range: indices.instance_index..indices.instance_index + 1,
                            first_indirect_parameters_index: None,
                        }
                    }
                    PhaseItemExtraIndex::IndirectParametersIndex {
                        range: ref indirect_parameters_index,
                        ..
                    } => {
                        // This is the first entity we've seen, and we have compute
                        // shaders. Initialize the fast path.
                        *self = UnbatchableBinnedEntityIndexSet::Sparse {
                            instance_range: indices.instance_index..indices.instance_index + 1,
                            first_indirect_parameters_index: NonMaxU32::new(
                                indirect_parameters_index.start,
                            ),
                        }
                    }
                }
            }

            UnbatchableBinnedEntityIndexSet::Sparse {
                instance_range,
                first_indirect_parameters_index,
            } if instance_range.end == indices.instance_index
                && ((first_indirect_parameters_index.is_none()
                    && indices.extra_index == PhaseItemExtraIndex::None)
                    || first_indirect_parameters_index.is_some_and(
                        |first_indirect_parameters_index| match indices.extra_index {
                            PhaseItemExtraIndex::IndirectParametersIndex {
                                range: ref this_range,
                                ..
                            } => {
                                u32::from(first_indirect_parameters_index) + instance_range.end
                                    - instance_range.start
                                    == this_range.start
                            }
                            PhaseItemExtraIndex::DynamicOffset(_) | PhaseItemExtraIndex::None => {
                                false
                            }
                        },
                    )) =>
            {
                // This is the normal case on non-WebGL 2.
                instance_range.end += 1;
            }

            UnbatchableBinnedEntityIndexSet::Sparse { instance_range, .. } => {
                // We thought we were in non-WebGL 2 mode, but we got a dynamic
                // offset or non-contiguous index anyway. This shouldn't happen,
                // but let's go ahead and do the sensible thing anyhow: demote
                // the compressed `NoDynamicOffsets` field to the full
                // `DynamicOffsets` array.
                warn!(
                    "Unbatchable binned entity index set was demoted from sparse to dense. \
                    This is a bug in the renderer. Please report it.",
                );
                let new_dynamic_offsets = (0..instance_range.len() as u32)
                    .flat_map(|entity_index| self.indices_for_entity_index(entity_index))
                    .chain(iter::once(indices))
                    .collect();
                *self = UnbatchableBinnedEntityIndexSet::Dense(new_dynamic_offsets);
            }

            UnbatchableBinnedEntityIndexSet::Dense(dense_indices) => {
                dense_indices.push(indices);
            }
        }
    }

    /// Clears the unbatchable binned entity index set.
    fn clear(&mut self) {
        match self {
            UnbatchableBinnedEntityIndexSet::Dense(dense_indices) => dense_indices.clear(),
            UnbatchableBinnedEntityIndexSet::Sparse { .. } => {
                *self = UnbatchableBinnedEntityIndexSet::NoEntities;
            }
            _ => {}
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
pub struct SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
    /// The items within this [`SortedRenderPhase`].
    pub items: IndexMap<(Entity, MainEntity), I, EntityHash>,
    /// Items within this render phase that will be automatically removed after
    /// this frame.
    pub transient_items: Vec<(Entity, MainEntity)>,
}

impl<I> Default for SortedRenderPhase<I>
where
    I: SortedPhaseItem,
{
    fn default() -> Self {
        Self {
            items: IndexMap::default(),
            transient_items: vec![],
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
        self.items.insert((item.entity(), item.main_entity()), item);
    }

    /// Adds a [`PhaseItem`] which will be automatically removed after this
    /// frame to this phase.
    #[inline]
    pub fn add_transient(&mut self, item: I) {
        let key = (item.entity(), item.main_entity());
        self.items.insert(key, item);
        self.transient_items.push(key);
    }

    /// Removes the [`PhaseItem`] corresponding to the given main-world entity
    /// from this render phase.
    #[inline]
    pub fn remove(&mut self, render_entity: Entity, main_entity: MainEntity) {
        self.items.swap_remove(&(render_entity, main_entity));
    }

    /// Removes all [`PhaseItem`]s from this render phase.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Populates whatever internal fields are necessary in order to perform the
    /// sort.
    ///
    /// For example, for transparent 3D phases, this calculates the distance
    /// from each object to the view.
    pub fn recalculate_sort_keys(&mut self, view: &ExtractedView) {
        I::recalculate_sort_keys(&mut self.items, view);
    }

    /// Sorts all of its [`PhaseItem`]s.
    pub fn sort(&mut self) {
        I::sort(&mut self.items);
    }

    /// An [`Iterator`] through the associated [`Entity`] for each [`PhaseItem`] in order.
    #[inline]
    pub fn iter_entities(&'_ self) -> impl Iterator<Item = Entity> + '_ {
        self.items.values().map(PhaseItem::entity)
    }

    /// Renders all of its [`PhaseItem`]s using their corresponding draw functions.
    pub fn render<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        self.render_range(render_pass, world, view, ..)
    }

    /// Renders all [`PhaseItem`]s in the provided `range` (based on their index in `self.items`) using their corresponding draw functions.
    pub fn render_range<'w>(
        &self,
        render_pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
        range: impl RangeBounds<usize>,
    ) -> Result<(), DrawError> {
        let items = self
            .items
            .get_range(range)
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
                draw_function.draw(world, render_pass, view, item)?;
                index += batch_range.len();
            }
        }
        Ok(())
    }
}

/// An item (entity of the render world) which will be drawn to a texture or the screen,
/// as part of a render phase.
///
/// The data required for rendering an entity is extracted from the main world in the
/// [`ExtractSchedule`](crate::ExtractSchedule).
/// Then it has to be queued up for rendering during the [`RenderSystems::Queue`],
/// by adding a corresponding phase item to a render phase.
/// Afterwards it will be possibly sorted and rendered automatically in the
/// [`RenderSystems::PhaseSort`] and [`RenderSystems::Render`], respectively.
///
/// `PhaseItem`s come in two flavors: [`BinnedPhaseItem`]s and
/// [`SortedPhaseItem`]s.
///
/// * Binned phase items have a `BinKey` which specifies what bin they're to be
///   placed in. All items in the same bin are eligible to be batched together.
///   The `BinKey`s are sorted, but the individual bin items aren't. Binned phase
///   items are good for opaque meshes, in which the order of rendering isn't
///   important. Generally, binned phase items are faster than sorted phase items.
///
/// * Sorted phase items, on the other hand, are placed into one large buffer
///   and then sorted all at once. This is needed for transparent meshes, which
///   have to be sorted back-to-front to render with the painter's algorithm.
///   These types of phase items are generally slower than binned phase items.
pub trait PhaseItem: Sized + Send + Sync + 'static {
    /// Whether or not this `PhaseItem` should be subjected to automatic batching. (Default: `true`)
    const AUTOMATIC_BATCHING: bool = true;

    /// The corresponding entity that will be drawn.
    ///
    /// This is used to fetch the render data of the entity, required by the draw function,
    /// from the render world .
    fn entity(&self) -> Entity;

    /// The main world entity represented by this `PhaseItem`.
    fn main_entity(&self) -> MainEntity;

    /// Specifies the [`Draw`] function used to render the item.
    fn draw_function(&self) -> DrawFunctionId;

    /// The range of instances that the batch covers. After doing a batched draw, batch range
    /// length phase items will be skipped. This design is to avoid having to restructure the
    /// render phase unnecessarily.
    fn batch_range(&self) -> &Range<u32>;
    fn batch_range_mut(&mut self) -> &mut Range<u32>;

    /// Returns the [`PhaseItemExtraIndex`].
    ///
    /// If present, this is either a dynamic offset or an indirect parameters
    /// index.
    fn extra_index(&self) -> PhaseItemExtraIndex;

    /// Returns a pair of mutable references to both the batch range and extra
    /// index.
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex);
}

/// The "extra index" associated with some [`PhaseItem`]s, alongside the
/// indirect instance index.
///
/// Sometimes phase items require another index in addition to the range of
/// instances they already have. These can be:
///
/// * The *dynamic offset*: a `wgpu` dynamic offset into the uniform buffer of
///   instance data. This is used on platforms that don't support storage
///   buffers, to work around uniform buffer size limitations.
///
/// * The *indirect parameters index*: an index into the buffer that specifies
///   the indirect parameters for this [`PhaseItem`]'s drawcall. This is used when
///   indirect mode is on (as used for GPU culling).
///
/// Note that our indirect draw functionality requires storage buffers, so it's
/// impossible to have both a dynamic offset and an indirect parameters index.
/// This convenient fact allows us to pack both indices into a single `u32`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum PhaseItemExtraIndex {
    /// No extra index is present.
    None,
    /// A `wgpu` dynamic offset into the uniform buffer of instance data. This
    /// is used on platforms that don't support storage buffers, to work around
    /// uniform buffer size limitations.
    DynamicOffset(u32),
    /// An index into the buffer that specifies the indirect parameters for this
    /// [`PhaseItem`]'s drawcall. This is used when indirect mode is on (as used
    /// for GPU culling).
    IndirectParametersIndex {
        /// The range of indirect parameters within the indirect parameters array.
        ///
        /// If we're using `multi_draw_indirect_count`, this specifies the
        /// maximum range of indirect parameters within that array. If batches
        /// are ultimately culled out on the GPU, the actual number of draw
        /// commands might be lower than the length of this range.
        range: Range<u32>,
        /// If `multi_draw_indirect_count` is in use, and this phase item is
        /// part of a batch set, specifies the index of the batch set that this
        /// phase item is a part of.
        ///
        /// If `multi_draw_indirect_count` isn't in use, or this phase item
        /// isn't part of a batch set, this is `None`.
        batch_set_index: Option<NonMaxU32>,
    },
}

impl PhaseItemExtraIndex {
    /// Returns either an indirect parameters index or
    /// [`PhaseItemExtraIndex::None`], as appropriate.
    pub fn maybe_indirect_parameters_index(
        indirect_parameters_index: Option<NonMaxU32>,
    ) -> PhaseItemExtraIndex {
        match indirect_parameters_index {
            Some(indirect_parameters_index) => PhaseItemExtraIndex::IndirectParametersIndex {
                range: u32::from(indirect_parameters_index)
                    ..(u32::from(indirect_parameters_index) + 1),
                batch_set_index: None,
            },
            None => PhaseItemExtraIndex::None,
        }
    }

    /// Returns either a dynamic offset index or [`PhaseItemExtraIndex::None`],
    /// as appropriate.
    pub fn maybe_dynamic_offset(dynamic_offset: Option<NonMaxU32>) -> PhaseItemExtraIndex {
        match dynamic_offset {
            Some(dynamic_offset) => PhaseItemExtraIndex::DynamicOffset(dynamic_offset.into()),
            None => PhaseItemExtraIndex::None,
        }
    }
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
    type BinKey: Clone + Send + Sync + PartialEq + Eq + Ord + Hash;

    /// The key used to combine batches into batch sets.
    ///
    /// A *batch set* is a set of meshes that can potentially be multi-drawn
    /// together.
    type BatchSetKey: PhaseItemBatchSetKey;

    /// Creates a new binned phase item from the key and per-entity data.
    ///
    /// Unlike [`SortedPhaseItem`]s, this is generally called "just in time"
    /// before rendering. The resulting phase item isn't stored in any data
    /// structures, resulting in significant memory savings.
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self;
}

/// A key used to combine batches into batch sets.
///
/// A *batch set* is a set of meshes that can potentially be multi-drawn
/// together.
pub trait PhaseItemBatchSetKey: Clone + Send + Sync + PartialEq + Eq + Ord + Hash {
    /// Returns true if this batch set key describes indexed meshes or false if
    /// it describes non-indexed meshes.
    ///
    /// Bevy uses this in order to determine which kind of indirect draw
    /// parameters to use, if indirect drawing is enabled.
    fn indexed(&self) -> bool;
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
    /// which uses an unstable sort, as this provides the best balance of CPU and GPU
    /// performance.
    ///
    /// Implementers can optionally not sort the list at all. This is generally advisable if and
    /// only if the renderer supports a depth prepass, which is by default not supported by
    /// the rest of Bevy's first party rendering crates. Even then, this may have a negative
    /// impact on GPU-side performance due to overdraw.
    ///
    /// It's advised to always profile for performance changes when changing this implementation.
    #[inline]
    fn sort(items: &mut IndexMap<(Entity, MainEntity), Self, EntityHash>) {
        items.sort_unstable_by_key(|_, value| Self::sort_key(value));
    }

    /// Populates whatever internal fields are necessary in order to perform the
    /// sort.
    ///
    /// The renderer calls this method right before calling [`Self::sort`]. For
    /// 3D transparent phases that need to be depth sorted, it populates the
    /// `distance` field with the actual distance from the view. For other
    /// phases, this method is generally a no-op.
    fn recalculate_sort_keys(
        items: &mut IndexMap<(Entity, MainEntity), Self, EntityHash>,
        view: &ExtractedView,
    );

    /// Whether this phase item targets indexed meshes (those with both vertex
    /// and index buffers as opposed to just vertex buffers).
    ///
    /// Bevy needs this information in order to properly group phase items
    /// together for multi-draw indirect, because the GPU layout of indirect
    /// commands differs between indexed and non-indexed meshes.
    ///
    /// If you're implementing a custom phase item that doesn't describe a mesh,
    /// you can safely return false here.
    fn indexed(&self) -> bool;
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
            RenderCommandResult::Skip
        }
    }
}

/// This system sorts the [`PhaseItem`]s of all [`SortedRenderPhase`]s of this
/// type.
pub fn sort_phase_system<I>(
    views: Query<&ExtractedView>,
    mut render_phases: ResMut<ViewSortedRenderPhases<I>>,
) where
    I: SortedPhaseItem,
{
    for view in &views {
        let Some(phase) = render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        phase.recalculate_sort_keys(view);
        phase.sort();
    }
}

impl BinnedRenderPhaseType {
    pub fn mesh(
        batchable: bool,
        gpu_preprocessing_support: &GpuPreprocessingSupport,
    ) -> BinnedRenderPhaseType {
        match (batchable, gpu_preprocessing_support.max_supported_mode) {
            (true, GpuPreprocessingMode::Culling) => BinnedRenderPhaseType::MultidrawableMesh,
            (true, _) => BinnedRenderPhaseType::BatchableMesh,
            (false, _) => BinnedRenderPhaseType::UnbatchableMesh,
        }
    }
}

impl RenderBin {
    /// Creates a [`RenderBin`] containing a single entity.
    fn from_entity(entity: MainEntity, uniform_index: InputUniformIndex) -> RenderBin {
        let mut entities = IndexMap::default();
        entities.insert(entity, uniform_index);
        RenderBin { entities }
    }

    /// Inserts an entity into the bin.
    fn insert(&mut self, entity: MainEntity, uniform_index: InputUniformIndex) {
        self.entities.insert(entity, uniform_index);
    }

    /// Removes an entity from the bin.
    fn remove(&mut self, entity_to_remove: MainEntity) {
        self.entities.swap_remove(&entity_to_remove);
    }

    /// Returns true if the bin contains no entities.
    fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns the [`IndexMap`] containing all the entities in the bin, along
    /// with the cached [`InputUniformIndex`] of each.
    #[inline]
    pub fn entities(&self) -> &IndexMap<MainEntity, InputUniformIndex, EntityHash> {
        &self.entities
    }
}

#[cfg(test)]
mod tests {
    use proptest_derive::Arbitrary;

    use crate::render_phase::GpuRenderBinnedMeshInstance;

    /// A `proptest`-based randomized test for `RenderMultidrawableBatchSet`.
    ///
    /// `proptest` works by generating random test cases and performing checks.
    /// We use it to generate random sets of entity bin insertion and removal
    /// operations, then verify that the data structure is consistent and that
    /// all invariants are upheld.
    #[test]
    #[expect(
        non_local_definitions,
        reason = "`derive(Arbitrary)` generates an impl here"
    )]
    fn render_multidrawable_batch_set() {
        use super::RenderMultidrawableBatchSet;

        use core::ops::Range;

        use bevy_ecs::entity::{Entity, EntityIndex};
        use bevy_material::labels::DrawFunctionId;
        use proptest::{bool, collection, test_runner::TestRunner};

        use crate::{
            render_phase::{
                BinnedPhaseItem, InputUniformIndex, PhaseItem, PhaseItemBatchSetKey, RenderBinIndex,
            },
            sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet},
        };

        /// A fake `BinnedPhaseItem` that we use for testing.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        struct MockBinnedPhaseItem;

        /// A fake `BinnedPhaseItem::BinKey` that we use for testing.
        ///
        /// The bin key should match the bin index.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        struct MockBinnedPhaseItemBinKey(u32);

        /// A fake `BinnedPhaseItem::BatchKey` that we use for testing.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        struct MockBinnedPhaseItemBatchSetKey;

        impl BinnedPhaseItem for MockBinnedPhaseItem {
            type BinKey = MockBinnedPhaseItemBinKey;

            type BatchSetKey = MockBinnedPhaseItemBatchSetKey;

            fn new(
                _: Self::BatchSetKey,
                _: Self::BinKey,
                _: (Entity, MainEntity),
                _: Range<u32>,
                _: super::PhaseItemExtraIndex,
            ) -> Self {
                Self
            }
        }

        impl PhaseItem for MockBinnedPhaseItem {
            fn entity(&self) -> Entity {
                unimplemented!()
            }

            fn main_entity(&self) -> MainEntity {
                unimplemented!()
            }

            fn draw_function(&self) -> DrawFunctionId {
                unimplemented!()
            }

            fn batch_range(&self) -> &Range<u32> {
                unimplemented!()
            }

            fn batch_range_mut(&mut self) -> &mut Range<u32> {
                unimplemented!()
            }

            fn extra_index(&self) -> super::PhaseItemExtraIndex {
                unimplemented!()
            }

            fn batch_range_and_extra_index_mut(
                &mut self,
            ) -> (&mut Range<u32>, &mut super::PhaseItemExtraIndex) {
                unimplemented!()
            }
        }

        impl PhaseItemBatchSetKey for MockBinnedPhaseItemBatchSetKey {
            fn indexed(&self) -> bool {
                // Doesn't matter. We arbitrarily return true.
                true
            }
        }

        /// A single operation that we perform on the multidrawable batch set.
        #[derive(Arbitrary, Debug)]
        enum RenderMultidrawableBatchSetOperation {
            /// Add an entity (mock mesh instance) to the batch set.
            Add {
                /// The ID of the entity.
                #[proptest(strategy = "0..32u32")]
                entity_id: u32,
                /// The index of the bin that we place the entity into.
                #[proptest(strategy = "0..8u32")]
                bin_index: u32,
                /// The input uniform index associated with the entity.
                #[proptest(strategy = "0..1024u32")]
                input_uniform_index: u32,
            },
            /// Remove an entity (mock mesh instance) from the batch set.
            Remove {
                /// The ID of the entity.
                #[proptest(strategy = "0..32u32")]
                entity_id: u32,
            },
        }

        /// A "control" structure that stores the expected contents of the
        /// multidrawable batch set.
        ///
        /// This is essentially a simpler, but inefficient, version of
        /// `RenderMultidrawableBatchSet` that we use to check that the
        /// invariants of `RenderMultidrawableBatchSet` are being upheld.
        struct ExpectedMultidrawableBatchSet {
            /// A mapping from each bin index to the entities within it.
            bin_index_to_entities: Vec<MainEntityHashSet>,
            /// A mapping from each entity ID to the binned mesh instance data.
            entity_to_binned_mesh_instance: MainEntityHashMap<GpuRenderBinnedMeshInstance>,
        }

        impl ExpectedMultidrawableBatchSet {
            /// Inserts an entity into the given bin of the control structure.
            fn insert(
                &mut self,
                entity: MainEntity,
                bin_index: RenderBinIndex,
                input_uniform_index: InputUniformIndex,
            ) {
                self.entity_to_binned_mesh_instance.insert(
                    entity,
                    GpuRenderBinnedMeshInstance {
                        bin_index: bin_index.0,
                        input_uniform_index: input_uniform_index.0,
                    },
                );
                self.bin_index_to_entities[bin_index.0 as usize].insert(entity);
            }

            /// Removes an entity from the control structure and returns its instance.
            fn remove(&mut self, entity: MainEntity) -> GpuRenderBinnedMeshInstance {
                let render_binned_mesh_instance =
                    self.entity_to_binned_mesh_instance.remove(&entity).unwrap();
                self.bin_index_to_entities[render_binned_mesh_instance.bin_index as usize]
                    .remove(&entity);
                render_binned_mesh_instance
            }
        }

        let mut runner = TestRunner::default();
        runner
            .run(
                // Generate up to 1024 random operations.
                //
                // Invalid operations (attempting to bin an entity that's
                // already binned or attempting to unbin an entity that wasn't
                // binned) will be skipped.
                &collection::vec(
                    proptest::prelude::any::<RenderMultidrawableBatchSetOperation>(),
                    0..1024,
                ),
                |ops| {
                    // Create the data structure to test.
                    let mut batch_set = RenderMultidrawableBatchSet::<MockBinnedPhaseItem>::new();

                    // Create the control data structure.
                    let mut expected = ExpectedMultidrawableBatchSet {
                        bin_index_to_entities: vec![MainEntityHashSet::default(); 1024],
                        entity_to_binned_mesh_instance: MainEntityHashMap::default(),
                    };

                    // Process each operation, skipping invalid ones.
                    for op in ops.iter() {
                        match *op {
                            RenderMultidrawableBatchSetOperation::Add {
                                entity_id,
                                bin_index,
                                input_uniform_index,
                            } => {
                                let entity = MainEntity::from(Entity::from_index(
                                    EntityIndex::from_raw_u32(entity_id).unwrap(),
                                ));
                                let input_uniform_index = InputUniformIndex(input_uniform_index);

                                // Skip this operation if it's trying to add an entity that's already binned.
                                if expected
                                    .entity_to_binned_mesh_instance
                                    .contains_key(&entity)
                                {
                                    continue;
                                }

                                // Insert into the expected and actual data
                                // structures.
                                expected.insert(
                                    entity,
                                    RenderBinIndex(bin_index),
                                    input_uniform_index,
                                );
                                batch_set.insert(
                                    MockBinnedPhaseItemBinKey(bin_index),
                                    entity,
                                    input_uniform_index,
                                );
                            }

                            RenderMultidrawableBatchSetOperation::Remove { entity_id } => {
                                let entity = MainEntity::from(Entity::from_index(
                                    EntityIndex::from_raw_u32(entity_id).unwrap(),
                                ));

                                // Skip this operation if it's trying to remove
                                // an entity that wasn't already binned.
                                if !expected
                                    .entity_to_binned_mesh_instance
                                    .contains_key(&entity)
                                {
                                    continue;
                                }

                                // Insert into the expected and actual data
                                // structures.
                                let render_binned_mesh_instance = expected.remove(entity);
                                batch_set.remove(
                                    entity,
                                    &MockBinnedPhaseItemBinKey(
                                        render_binned_mesh_instance.bin_index,
                                    ),
                                );
                            }
                        }
                    }

                    // Verify that the batch set invariants are upheld.
                    verify(&batch_set, &expected);

                    Ok(())
                },
            )
            .unwrap();

        // Verifies that the given `batch_set` matches the expected batch set
        // and ensures that the invariants of that batch set are upheld.
        fn verify(
            batch_set: &RenderMultidrawableBatchSet<MockBinnedPhaseItem>,
            expected: &ExpectedMultidrawableBatchSet,
        ) {
            // Verify every entity is present.
            verify_entity_presence(
                batch_set,
                &expected.bin_index_to_entities,
                &expected.entity_to_binned_mesh_instance,
            );

            // Verify that the binned mesh instance GPU buffer is correct.
            verify_render_binned_mesh_instance_buffer(batch_set, &expected.bin_index_to_entities);

            // Verify that no indirect parameter offsets overlap.
            verify_indirect_parameters_offsets(batch_set);
        }

        /// Verifies that every entity is present in the multidrawable batch
        /// set after modifications.
        fn verify_entity_presence(
            batch_set: &RenderMultidrawableBatchSet<MockBinnedPhaseItem>,
            expected: &[MainEntityHashSet],
            entity_to_bin_index_and_input_uniform_index: &MainEntityHashMap<
                GpuRenderBinnedMeshInstance,
            >,
        ) {
            for (bin_key_index, expected_entities) in expected.iter().enumerate() {
                let bin_key = MockBinnedPhaseItemBinKey(bin_key_index as u32);
                if expected_entities.is_empty() {
                    assert!(!batch_set.bin_key_to_bin_index.contains_key(&bin_key));
                    continue;
                }

                let Some(render_bin_index) = batch_set.bin_key_to_bin_index.get(&bin_key) else {
                    panic!("Bin not present: key {:?}", bin_key);
                };
                let Some(render_bin) = batch_set.bin(*render_bin_index) else {
                    panic!("Bin not present: index {:?}", render_bin_index);
                };
                for expected_entity in expected_entities {
                    let Some(GpuRenderBinnedMeshInstance {
                        bin_index,
                        input_uniform_index,
                    }) = entity_to_bin_index_and_input_uniform_index.get(expected_entity)
                    else {
                        panic!(
                            "Test harness bug: entity-to-bin-index-and-input-uniform-index \
                                table and expected table don't agree"
                        );
                    };
                    assert_eq!(MockBinnedPhaseItemBinKey(*bin_index), bin_key);

                    let Some(render_bin_buffer_index) = render_bin
                        .entity_to_binned_mesh_instance_index
                        .get(expected_entity)
                    else {
                        panic!("Buffer index not present");
                    };
                    let render_bin_entry = batch_set
                        .gpu_buffers
                        .render_binned_mesh_instance_buffer
                        .values()[render_bin_buffer_index.0 as usize];
                    assert_eq!(render_bin_entry.bin_index, **render_bin_index);
                    assert_eq!(render_bin_entry.input_uniform_index, *input_uniform_index);
                }
            }
        }

        /// Verifies that the
        /// `RenderMultidrawableBatchSet::render_binned_mesh_instances_cpu`
        /// contains the correct entity and bin index.
        fn verify_render_binned_mesh_instance_buffer(
            batch_set: &RenderMultidrawableBatchSet<MockBinnedPhaseItem>,
            expected: &[MainEntityHashSet],
        ) {
            for (render_bin_buffer_index, gpu_render_binned_mesh_instance) in batch_set
                .gpu_buffers
                .render_binned_mesh_instance_buffer
                .values()
                .iter()
                .enumerate()
            {
                let binned_mesh_instance_cpu =
                    &batch_set.render_binned_mesh_instances_cpu[render_bin_buffer_index];

                // Make sure that the `GpuRenderBinnedMeshInstance::bin_index`
                // matches the `CpuRenderBinnedMeshInstance::bin_index`.
                let gpu_render_bin_index = gpu_render_binned_mesh_instance.bin_index;
                assert_eq!(gpu_render_bin_index, *binned_mesh_instance_cpu.bin_index);

                let render_bin = batch_set.bins[gpu_render_bin_index as usize]
                    .as_ref()
                    .unwrap();

                // Make sure that the entity in the
                // `RenderMultidrawableBin::entity_to_binned_mesh_instance_index`
                // table matches the entity in the
                // `CpuRenderBinnedMeshInstance`.
                let Some(entity) = render_bin
                    .entity_to_binned_mesh_instance_index
                    .iter()
                    .find_map(|(entity, buffer_index)| {
                        if render_bin_buffer_index as u32 == buffer_index.0 {
                            Some(entity)
                        } else {
                            None
                        }
                    })
                else {
                    panic!(
                        "Entity at buffer index {:?} not found in bin {:?}",
                        render_bin_buffer_index, gpu_render_bin_index
                    );
                };
                assert_eq!(binned_mesh_instance_cpu.main_entity, *entity);

                // Make sure that the bin with the appropriate bin key should
                // actually contain the entity.
                let Some(bin_key) =
                    batch_set
                        .bin_key_to_bin_index
                        .iter()
                        .find_map(|(bin_key, bin_index)| {
                            if bin_index.0 == gpu_render_bin_index {
                                Some(*bin_key)
                            } else {
                                None
                            }
                        })
                else {
                    panic!(
                        "Couldn't find a bin key for bin index {:?}",
                        gpu_render_bin_index
                    );
                };
                assert!(expected[bin_key.0 as usize].contains(entity));
            }
        }

        fn verify_indirect_parameters_offsets(
            batch_set: &RenderMultidrawableBatchSet<MockBinnedPhaseItem>,
        ) {
            for (render_bin_index, indirect_parameters_offset) in batch_set
                .gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .values()
                .iter()
                .enumerate()
            {
                if *indirect_parameters_offset == u32::MAX {
                    continue;
                }
                assert_eq!(
                    batch_set.indirect_parameters_offset_to_bin_index
                        [*indirect_parameters_offset as usize],
                    RenderBinIndex(render_bin_index as u32)
                );
            }
            for (indirect_parameters_offset, render_bin_index) in batch_set
                .indirect_parameters_offset_to_bin_index
                .iter()
                .enumerate()
            {
                assert!(batch_set.bins[render_bin_index.0 as usize].is_some());
                assert_eq!(
                    *batch_set
                        .gpu_buffers
                        .bin_index_to_indirect_parameters_offset_buffer
                        .get(render_bin_index.0)
                        .unwrap(),
                    indirect_parameters_offset as u32
                );
            }
        }
    }
}
