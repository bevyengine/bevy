//! Batching functionality when GPU preprocessing is in use.

use bevy_app::{App, Plugin};
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    query::{Has, With},
    schedule::IntoSystemConfigs as _,
    system::{Query, Res, ResMut, Resource, StaticSystemParam},
    world::{FromWorld, World},
};
use bevy_encase_derive::ShaderType;
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};
use nonmax::NonMaxU32;
use tracing::error;
use wgpu::{BindingResource, BufferUsages, DownlevelFlags, Features};

use crate::{
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhaseBatch, BinnedRenderPhaseBatchSet,
        BinnedRenderPhaseBatchSets, CachedRenderPipelinePhaseItem, PhaseItemExtraIndex,
        SortedPhaseItem, SortedRenderPhase, UnbatchableBinnedEntityIndices, ViewBinnedRenderPhases,
        ViewSortedRenderPhases,
    },
    render_resource::{Buffer, BufferVec, GpuArrayBufferable, RawBufferVec, UninitBufferVec},
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    view::{ExtractedView, NoIndirectDrawing},
    Render, RenderApp, RenderSet,
};

use super::{BatchMeta, GetBatchData, GetFullBatchData};

pub struct BatchingPlugin;

impl Plugin for BatchingPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(IndirectParametersBuffer::new())
            .add_systems(
                Render,
                write_indirect_parameters_buffer.in_set(RenderSet::PrepareResourcesFlush),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<GpuPreprocessingSupport>();
    }
}

/// Records whether GPU preprocessing and/or GPU culling are supported on the
/// device.
///
/// No GPU preprocessing is supported on WebGL because of the lack of compute
/// shader support.  GPU preprocessing is supported on DirectX 12, but due to [a
/// `wgpu` limitation] GPU culling is not.
///
/// [a `wgpu` limitation]: https://github.com/gfx-rs/wgpu/issues/2471
#[derive(Clone, Copy, PartialEq, Resource)]
pub struct GpuPreprocessingSupport {
    /// The maximum amount of GPU preprocessing available on this platform.
    pub max_supported_mode: GpuPreprocessingMode,
}

impl GpuPreprocessingSupport {
    /// Returns true if this GPU preprocessing support level isn't `None`.
    #[inline]
    pub fn is_available(&self) -> bool {
        self.max_supported_mode != GpuPreprocessingMode::None
    }

    /// Returns the given GPU preprocessing mode, capped to the current
    /// preprocessing mode.
    pub fn min(&self, mode: GpuPreprocessingMode) -> GpuPreprocessingMode {
        match (self.max_supported_mode, mode) {
            (GpuPreprocessingMode::None, _) | (_, GpuPreprocessingMode::None) => {
                GpuPreprocessingMode::None
            }
            (mode, GpuPreprocessingMode::Culling) | (GpuPreprocessingMode::Culling, mode) => mode,
            (GpuPreprocessingMode::PreprocessingOnly, GpuPreprocessingMode::PreprocessingOnly) => {
                GpuPreprocessingMode::PreprocessingOnly
            }
        }
    }
}

/// The amount of GPU preprocessing (compute and indirect draw) that we do.
#[derive(Clone, Copy, PartialEq)]
pub enum GpuPreprocessingMode {
    /// No GPU preprocessing is in use at all.
    ///
    /// This is used when GPU compute isn't available.
    None,

    /// GPU preprocessing is in use, but GPU culling isn't.
    ///
    /// This is used when the [`NoIndirectDrawing`] component is present on the
    /// camera.
    PreprocessingOnly,

    /// Both GPU preprocessing and GPU culling are in use.
    ///
    /// This is used by default.
    Culling,
}

/// The GPU buffers holding the data needed to render batches.
///
/// For example, in the 3D PBR pipeline this holds `MeshUniform`s, which are the
/// `BD` type parameter in that mode.
///
/// We have a separate *buffer data input* type (`BDI`) here, which a compute
/// shader is expected to expand to the full buffer data (`BD`) type. GPU
/// uniform building is generally faster and uses less system RAM to VRAM bus
/// bandwidth, but only implemented for some pipelines (for example, not in the
/// 2D pipeline at present) and only when compute shader is available.
#[derive(Resource)]
pub struct BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod + Default,
{
    /// A storage area for the buffer data that the GPU compute shader is
    /// expected to write to.
    ///
    /// There will be one entry for each index.
    pub data_buffer: UninitBufferVec<BD>,

    /// The index of the buffer data in the current input buffer that
    /// corresponds to each instance.
    ///
    /// This is keyed off each view. Each view has a separate buffer.
    pub work_item_buffers: EntityHashMap<PreprocessWorkItemBuffer>,

    /// The uniform data inputs for the current frame.
    ///
    /// These are uploaded during the extraction phase.
    pub current_input_buffer: InstanceInputUniformBuffer<BDI>,

    /// The uniform data inputs for the previous frame.
    ///
    /// The indices don't generally line up between `current_input_buffer`
    /// and `previous_input_buffer`, because, among other reasons, entities
    /// can spawn or despawn between frames. Instead, each current buffer
    /// data input uniform is expected to contain the index of the
    /// corresponding buffer data input uniform in this list.
    pub previous_input_buffer: InstanceInputUniformBuffer<BDI>,
}

/// Holds the GPU buffer of instance input data, which is the data about each
/// mesh instance that the CPU provides.
///
/// `BDI` is the *buffer data input* type, which the GPU mesh preprocessing
/// shader is expected to expand to the full *buffer data* type.
pub struct InstanceInputUniformBuffer<BDI>
where
    BDI: Pod + Default,
{
    /// The buffer containing the data that will be uploaded to the GPU.
    buffer: RawBufferVec<BDI>,

    /// Indices of slots that are free within the buffer.
    ///
    /// When adding data, we preferentially overwrite these slots first before
    /// growing the buffer itself.
    free_uniform_indices: Vec<u32>,
}

impl<BDI> InstanceInputUniformBuffer<BDI>
where
    BDI: Pod + Default,
{
    /// Creates a new, empty buffer.
    pub fn new() -> InstanceInputUniformBuffer<BDI> {
        InstanceInputUniformBuffer {
            buffer: RawBufferVec::new(BufferUsages::STORAGE),
            free_uniform_indices: vec![],
        }
    }

    /// Clears the buffer and entity list out.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.free_uniform_indices.clear();
    }

    /// Returns the [`RawBufferVec`] corresponding to this input uniform buffer.
    #[inline]
    pub fn buffer(&self) -> &RawBufferVec<BDI> {
        &self.buffer
    }

    /// Adds a new piece of buffered data to the uniform buffer and returns its
    /// index.
    pub fn add(&mut self, element: BDI) -> u32 {
        match self.free_uniform_indices.pop() {
            Some(uniform_index) => {
                self.buffer.values_mut()[uniform_index as usize] = element;
                uniform_index
            }
            None => self.buffer.push(element) as u32,
        }
    }

    /// Removes a piece of buffered data from the uniform buffer.
    ///
    /// This simply marks the data as free.
    pub fn remove(&mut self, uniform_index: u32) {
        self.free_uniform_indices.push(uniform_index);
    }

    /// Returns the piece of buffered data at the given index.
    ///
    /// Returns [`None`] if the index is out of bounds or the data is removed.
    pub fn get(&self, uniform_index: u32) -> Option<BDI> {
        if (uniform_index as usize) >= self.buffer.len()
            || self.free_uniform_indices.contains(&uniform_index)
        {
            None
        } else {
            Some(self.get_unchecked(uniform_index))
        }
    }

    /// Returns the piece of buffered data at the given index.
    /// Can return data that has previously been removed.
    ///
    /// # Panics
    /// if `uniform_index` is not in bounds of [`Self::buffer`].
    pub fn get_unchecked(&self, uniform_index: u32) -> BDI {
        self.buffer.values()[uniform_index as usize]
    }

    /// Stores a piece of buffered data at the given index.
    ///
    /// # Panics
    /// if `uniform_index` is not in bounds of [`Self::buffer`].
    pub fn set(&mut self, uniform_index: u32, element: BDI) {
        self.buffer.values_mut()[uniform_index as usize] = element;
    }

    // Ensures that the buffers are nonempty, which the GPU requires before an
    // upload can take place.
    pub fn ensure_nonempty(&mut self) {
        if self.buffer.is_empty() {
            self.buffer.push(default());
        }
    }
}

impl<BDI> Default for InstanceInputUniformBuffer<BDI>
where
    BDI: Pod + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

/// The buffer of GPU preprocessing work items for a single view.
pub struct PreprocessWorkItemBuffer {
    /// The buffer of work items.
    pub buffer: BufferVec<PreprocessWorkItem>,
    /// True if we're drawing directly instead of indirectly.
    pub no_indirect_drawing: bool,
}

/// One invocation of the preprocessing shader: i.e. one mesh instance in a
/// view.
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct PreprocessWorkItem {
    /// The index of the batch input data in the input buffer that the shader
    /// reads from.
    pub input_index: u32,
    /// The index of the `MeshUniform` in the output buffer that we write to.
    /// In direct mode, this is the index of the uniform. In indirect mode, this
    /// is the first index uniform in the batch set.
    pub output_index: u32,
    /// The index of the [`IndirectParameters`] in the
    /// [`IndirectParametersBuffer`].
    pub indirect_parameters_index: u32,
}

/// The `wgpu` indirect parameters structure.
///
/// This is actually a union of the two following structures:
///
/// ```
/// #[repr(C)]
/// struct ArrayIndirectParameters {
///     vertex_count: u32,
///     instance_count: u32,
///     first_vertex: u32,
///     first_instance: u32,
/// }
///
/// #[repr(C)]
/// struct ElementIndirectParameters {
///     index_count: u32,
///     instance_count: u32,
///     first_vertex: u32,
///     base_vertex: u32,
///     first_instance: u32,
/// }
/// ```
///
/// We actually generally treat these two variants identically in code. To do
/// that, we make the following two observations:
///
/// 1. `instance_count` is in the same place in both structures. So we can
///     access it regardless of the structure we're looking at.
///
/// 2. The second structure is one word larger than the first. Thus we need to
///     pad out the first structure by one word in order to place both structures in
///     an array. If we pad out `ArrayIndirectParameters` by copying the
///     `first_instance` field into the padding, then the resulting union structure
///     will always have a read-only copy of `first_instance` in the final word. We
///     take advantage of this in the shader to reduce branching.
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct IndirectParameters {
    /// For `ArrayIndirectParameters`, `vertex_count`; for
    /// `ElementIndirectParameters`, `index_count`.
    pub vertex_or_index_count: u32,

    /// The number of instances we're going to draw.
    ///
    /// This field is in the same place in both structures.
    pub instance_count: u32,

    /// For `ArrayIndirectParameters`, `first_vertex`; for
    /// `ElementIndirectParameters`, `first_index`.
    pub first_vertex_or_first_index: u32,

    /// For `ArrayIndirectParameters`, `first_instance`; for
    /// `ElementIndirectParameters`, `base_vertex`.
    pub base_vertex_or_first_instance: u32,

    /// For `ArrayIndirectParameters`, this is padding; for
    /// `ElementIndirectParameters`, this is `first_instance`.
    ///
    /// Conventionally, we copy `first_instance` into this field when padding
    /// out `ArrayIndirectParameters`. That way, shader code can read this value
    /// at the same place, regardless of the specific structure this represents.
    pub first_instance: u32,
}

/// The buffer containing the list of [`IndirectParameters`], for draw commands.
#[derive(Resource)]
pub struct IndirectParametersBuffer {
    /// The actual buffer.
    buffer: RawBufferVec<IndirectParameters>,
}

impl IndirectParametersBuffer {
    /// Creates the indirect parameters buffer.
    pub fn new() -> IndirectParametersBuffer {
        IndirectParametersBuffer {
            buffer: RawBufferVec::new(BufferUsages::STORAGE | BufferUsages::INDIRECT),
        }
    }

    /// Returns the underlying GPU buffer.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.buffer()
    }

    /// Adds a new set of indirect parameters to the buffer.
    pub fn allocate(&mut self, count: u32) -> u32 {
        let length = self.buffer.len();
        self.buffer.reserve_internal(count as usize);
        for _ in 0..count {
            self.buffer.push(Zeroable::zeroed());
        }
        length as u32
    }

    pub fn set(&mut self, index: u32, value: IndirectParameters) {
        self.buffer.set(index, value);
    }
}

impl Default for IndirectParametersBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl FromWorld for GpuPreprocessingSupport {
    fn from_world(world: &mut World) -> Self {
        let adapter = world.resource::<RenderAdapter>();
        let device = world.resource::<RenderDevice>();

        // Filter some Qualcomm devices on Android as they crash when using GPU
        // preprocessing.
        // We filter out Adreno 730 and earlier GPUs (except 720, as it's newer
        // than 730).
        fn is_non_supported_android_device(adapter: &RenderAdapter) -> bool {
            crate::get_adreno_model(adapter).is_some_and(|model| model != 720 && model <= 730)
        }

        let max_supported_mode = if device.limits().max_compute_workgroup_size_x == 0 ||
            is_non_supported_android_device(adapter)
        {
            GpuPreprocessingMode::None
        } else if !device
            .features()
            .contains(Features::INDIRECT_FIRST_INSTANCE | Features::MULTI_DRAW_INDIRECT) ||
            !adapter.get_downlevel_capabilities().flags.contains(
        DownlevelFlags::VERTEX_AND_INSTANCE_INDEX_RESPECTS_RESPECTIVE_FIRST_VALUE_IN_INDIRECT_DRAW)
        {
            GpuPreprocessingMode::PreprocessingOnly
        } else {
            GpuPreprocessingMode::Culling
        };

        GpuPreprocessingSupport { max_supported_mode }
    }
}

impl<BD, BDI> BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod + Default,
{
    /// Creates new buffers.
    pub fn new() -> Self {
        BatchedInstanceBuffers {
            data_buffer: UninitBufferVec::new(BufferUsages::STORAGE),
            work_item_buffers: EntityHashMap::default(),
            current_input_buffer: InstanceInputUniformBuffer::new(),
            previous_input_buffer: InstanceInputUniformBuffer::new(),
        }
    }

    /// Returns the binding of the buffer that contains the per-instance data.
    ///
    /// This buffer needs to be filled in via a compute shader.
    pub fn instance_data_binding(&self) -> Option<BindingResource> {
        self.data_buffer
            .buffer()
            .map(|buffer| buffer.as_entire_binding())
    }

    /// Clears out the buffers in preparation for a new frame.
    pub fn clear(&mut self) {
        self.data_buffer.clear();
        for work_item_buffer in self.work_item_buffers.values_mut() {
            work_item_buffer.buffer.clear();
        }
    }
}

impl<BD, BDI> Default for BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a render batch that we're building up during a sorted
/// render phase.
struct SortedRenderBatch<F>
where
    F: GetBatchData,
{
    /// The index of the first phase item in this batch in the list of phase
    /// items.
    phase_item_start_index: u32,

    /// The index of the first instance in this batch in the instance buffer.
    instance_start_index: u32,

    /// The index of the indirect parameters for this batch in the
    /// [`IndirectParametersBuffer`].
    ///
    /// If CPU culling is being used, then this will be `None`.
    indirect_parameters_index: Option<NonMaxU32>,

    /// Metadata that can be used to determine whether an instance can be placed
    /// into this batch.
    ///
    /// If `None`, the item inside is unbatchable.
    meta: Option<BatchMeta<F::CompareData>>,
}

impl<F> SortedRenderBatch<F>
where
    F: GetBatchData,
{
    /// Finalizes this batch and updates the [`SortedRenderPhase`] with the
    /// appropriate indices.
    ///
    /// `instance_end_index` is the index of the last instance in this batch
    /// plus one.
    fn flush<I>(self, instance_end_index: u32, phase: &mut SortedRenderPhase<I>)
    where
        I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    {
        let (batch_range, batch_extra_index) =
            phase.items[self.phase_item_start_index as usize].batch_range_and_extra_index_mut();
        *batch_range = self.instance_start_index..instance_end_index;
        *batch_extra_index =
            PhaseItemExtraIndex::maybe_indirect_parameters_index(self.indirect_parameters_index);
    }
}

/// A system that runs early in extraction and clears out all the
/// [`BatchedInstanceBuffers`] for the frame.
///
/// We have to run this during extraction because, if GPU preprocessing is in
/// use, the extraction phase will write to the mesh input uniform buffers
/// directly, so the buffers need to be cleared before then.
pub fn clear_batched_gpu_instance_buffers<GFBD>(
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    >,
) where
    GFBD: GetFullBatchData,
{
    if let Some(mut gpu_batched_instance_buffers) = gpu_batched_instance_buffers {
        gpu_batched_instance_buffers.clear();
    }
}

/// A system that removes GPU preprocessing work item buffers that correspond to
/// deleted [`ExtractedView`]s.
///
/// This is a separate system from [`clear_batched_gpu_instance_buffers`]
/// because [`ExtractedView`]s aren't created until after the extraction phase
/// is completed.
pub fn delete_old_work_item_buffers<GFBD>(
    mut gpu_batched_instance_buffers: ResMut<
        BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
    >,
    extracted_views: Query<Entity, With<ExtractedView>>,
) where
    GFBD: GetFullBatchData,
{
    gpu_batched_instance_buffers
        .work_item_buffers
        .retain(|entity, _| extracted_views.contains(*entity));
}

/// Batch the items in a sorted render phase, when GPU instance buffer building
/// is in use. This means comparing metadata needed to draw each phase item and
/// trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase<I, GFBD>(
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    mut indirect_parameters_buffer: ResMut<IndirectParametersBuffer>,
    mut sorted_render_phases: ResMut<ViewSortedRenderPhases<I>>,
    mut views: Query<(Entity, Has<NoIndirectDrawing>), With<ExtractedView>>,
    system_param_item: StaticSystemParam<GFBD::Param>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    GFBD: GetFullBatchData,
{
    // We only process GPU-built batch data in this function.
    let BatchedInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ..
    } = gpu_array_buffer.into_inner();

    for (view, no_indirect_drawing) in &mut views {
        let Some(phase) = sorted_render_phases.get_mut(&view) else {
            continue;
        };

        // Create the work item buffer if necessary.
        let work_item_buffer =
            work_item_buffers
                .entry(view)
                .or_insert_with(|| PreprocessWorkItemBuffer {
                    buffer: BufferVec::new(BufferUsages::STORAGE),
                    no_indirect_drawing,
                });

        // Walk through the list of phase items, building up batches as we go.
        let mut batch: Option<SortedRenderBatch<GFBD>> = None;

        // Allocate the indirect parameters if necessary.
        let mut indirect_parameters_offset = if no_indirect_drawing {
            None
        } else {
            Some(indirect_parameters_buffer.allocate(phase.items.len() as u32))
        };

        let mut first_output_index = data_buffer.len() as u32;

        for current_index in 0..phase.items.len() {
            // Get the index of the input data, and comparison metadata, for
            // this entity.
            let item = &phase.items[current_index];
            let entity = item.main_entity();
            let current_batch_input_index =
                GFBD::get_index_and_compare_data(&system_param_item, entity);

            // Unpack that index and metadata. Note that it's possible for index
            // and/or metadata to not be present, which signifies that this
            // entity is unbatchable. In that case, we break the batch here.
            // If the index isn't present the item is not part of this pipeline and so will be skipped.
            let Some((current_input_index, current_meta)) = current_batch_input_index else {
                // Break a batch if we need to.
                if let Some(batch) = batch.take() {
                    batch.flush(data_buffer.len() as u32, phase);
                }

                continue;
            };
            let current_meta =
                current_meta.map(|meta| BatchMeta::new(&phase.items[current_index], meta));

            // Determine if this entity can be included in the batch we're
            // building up.
            let can_batch = batch.as_ref().is_some_and(|batch| {
                // `None` for metadata indicates that the items are unbatchable.
                match (&current_meta, &batch.meta) {
                    (Some(current_meta), Some(batch_meta)) => current_meta == batch_meta,
                    (_, _) => false,
                }
            });

            // Make space in the data buffer for this instance.
            let item = &phase.items[current_index];
            let entity = item.main_entity();
            let output_index = data_buffer.add() as u32;

            // If we can't batch, break the existing batch and make a new one.
            if !can_batch {
                // Break a batch if we need to.
                if let Some(batch) = batch.take() {
                    batch.flush(output_index, phase);
                }

                // Start a new batch.
                if let Some(indirect_parameters_offset) = indirect_parameters_offset {
                    GFBD::write_batch_indirect_parameters(
                        &system_param_item,
                        &mut indirect_parameters_buffer,
                        indirect_parameters_offset,
                        entity,
                    );
                };

                batch = Some(SortedRenderBatch {
                    phase_item_start_index: current_index as u32,
                    instance_start_index: output_index,
                    indirect_parameters_index: indirect_parameters_offset.and_then(NonMaxU32::new),
                    meta: current_meta,
                });

                if let Some(ref mut indirect_parameters_offset) = indirect_parameters_offset {
                    *indirect_parameters_offset += 1;
                }

                first_output_index = output_index;
            }

            // Add a new preprocessing work item so that the preprocessing
            // shader will copy the per-instance data over.
            if let Some(batch) = batch.as_ref() {
                work_item_buffer.buffer.push(PreprocessWorkItem {
                    input_index: current_input_index.into(),
                    output_index: if no_indirect_drawing {
                        output_index
                    } else {
                        first_output_index
                    },
                    indirect_parameters_index: match batch.indirect_parameters_index {
                        Some(indirect_parameters_index) => indirect_parameters_index.into(),
                        None => 0,
                    },
                });
            }
        }

        // Flush the final batch if necessary.
        if let Some(batch) = batch.take() {
            batch.flush(data_buffer.len() as u32, phase);
        }
    }
}

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase<BPI, GFBD>(
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    mut indirect_parameters_buffer: ResMut<IndirectParametersBuffer>,
    mut binned_render_phases: ResMut<ViewBinnedRenderPhases<BPI>>,
    mut views: Query<(Entity, Has<NoIndirectDrawing>), With<ExtractedView>>,
    param: StaticSystemParam<GFBD::Param>,
) where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    let system_param_item = param.into_inner();

    let BatchedInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ..
    } = gpu_array_buffer.into_inner();

    for (view, no_indirect_drawing) in &mut views {
        let Some(phase) = binned_render_phases.get_mut(&view) else {
            continue;
        };

        // Create the work item buffer if necessary; otherwise, just mark it as
        // used this frame.
        let work_item_buffer =
            work_item_buffers
                .entry(view)
                .or_insert_with(|| PreprocessWorkItemBuffer {
                    buffer: BufferVec::new(BufferUsages::STORAGE),
                    no_indirect_drawing,
                });

        // Prepare multidrawables.

        for batch_set_key in &phase.multidrawable_mesh_keys {
            let mut batch_set = None;
            for (bin_key, bin) in &phase.multidrawable_mesh_values[batch_set_key] {
                let first_output_index = data_buffer.len() as u32;
                let mut batch: Option<BinnedRenderPhaseBatch> = None;

                for &(entity, main_entity) in &bin.entities {
                    let Some(input_index) = GFBD::get_binned_index(&system_param_item, main_entity)
                    else {
                        continue;
                    };
                    let output_index = data_buffer.add() as u32;

                    match batch {
                        Some(ref mut batch) => {
                            // Append to the current batch.
                            batch.instance_range.end = output_index + 1;
                            work_item_buffer.buffer.push(PreprocessWorkItem {
                                input_index: input_index.into(),
                                output_index: first_output_index,
                                indirect_parameters_index: match batch.extra_index {
                                    PhaseItemExtraIndex::IndirectParametersIndex(ref range) => {
                                        range.start
                                    }
                                    PhaseItemExtraIndex::DynamicOffset(_)
                                    | PhaseItemExtraIndex::None => 0,
                                },
                            });
                        }

                        None => {
                            // Start a new batch, in indirect mode.
                            let indirect_parameters_index = indirect_parameters_buffer.allocate(1);
                            GFBD::write_batch_indirect_parameters(
                                &system_param_item,
                                &mut indirect_parameters_buffer,
                                indirect_parameters_index,
                                main_entity,
                            );
                            work_item_buffer.buffer.push(PreprocessWorkItem {
                                input_index: input_index.into(),
                                output_index: first_output_index,
                                indirect_parameters_index,
                            });
                            batch = Some(BinnedRenderPhaseBatch {
                                representative_entity: (entity, main_entity),
                                instance_range: output_index..output_index + 1,
                                extra_index: PhaseItemExtraIndex::maybe_indirect_parameters_index(
                                    NonMaxU32::new(indirect_parameters_index),
                                ),
                            });
                        }
                    }
                }

                if let Some(batch) = batch {
                    match batch_set {
                        None => {
                            batch_set = Some(BinnedRenderPhaseBatchSet {
                                batches: vec![batch],
                                bin_key: bin_key.clone(),
                            });
                        }
                        Some(ref mut batch_set) => {
                            batch_set.batches.push(batch);
                        }
                    }
                }
            }

            if let BinnedRenderPhaseBatchSets::MultidrawIndirect(ref mut batch_sets) =
                phase.batch_sets
            {
                if let Some(batch_set) = batch_set {
                    batch_sets.push(batch_set);
                }
            }
        }

        // Prepare batchables.

        for key in &phase.batchable_mesh_keys {
            let first_output_index = data_buffer.len() as u32;

            let mut batch: Option<BinnedRenderPhaseBatch> = None;
            for &(entity, main_entity) in &phase.batchable_mesh_values[key].entities {
                let Some(input_index) = GFBD::get_binned_index(&system_param_item, main_entity)
                else {
                    continue;
                };
                let output_index = data_buffer.add() as u32;

                match batch {
                    Some(ref mut batch) => {
                        batch.instance_range.end = output_index + 1;

                        // Append to the current batch.
                        //
                        // If we're in indirect mode, then we write the first
                        // output index of this batch, so that we have a
                        // tightly-packed buffer if GPU culling discards some of
                        // the instances. Otherwise, we can just write the
                        // output index directly.
                        work_item_buffer.buffer.push(PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index: if no_indirect_drawing {
                                output_index
                            } else {
                                first_output_index
                            },
                            indirect_parameters_index: match batch.extra_index {
                                PhaseItemExtraIndex::IndirectParametersIndex(ref range) => {
                                    range.start
                                }
                                PhaseItemExtraIndex::DynamicOffset(_)
                                | PhaseItemExtraIndex::None => 0,
                            },
                        });
                    }

                    None if !no_indirect_drawing => {
                        // Start a new batch, in indirect mode.
                        let indirect_parameters_index = indirect_parameters_buffer.allocate(1);
                        GFBD::write_batch_indirect_parameters(
                            &system_param_item,
                            &mut indirect_parameters_buffer,
                            indirect_parameters_index,
                            main_entity,
                        );
                        work_item_buffer.buffer.push(PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index: first_output_index,
                            indirect_parameters_index,
                        });
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: (entity, main_entity),
                            instance_range: output_index..output_index + 1,
                            extra_index: PhaseItemExtraIndex::maybe_indirect_parameters_index(
                                NonMaxU32::new(indirect_parameters_index),
                            ),
                        });
                    }

                    None => {
                        // Start a new batch, in direct mode.
                        work_item_buffer.buffer.push(PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index,
                            indirect_parameters_index: 0,
                        });
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: (entity, main_entity),
                            instance_range: output_index..output_index + 1,
                            extra_index: PhaseItemExtraIndex::None,
                        });
                    }
                }
            }

            if let Some(batch) = batch {
                match phase.batch_sets {
                    BinnedRenderPhaseBatchSets::DynamicUniforms(_) => {
                        error!("Dynamic uniform batch sets shouldn't be used here");
                    }
                    BinnedRenderPhaseBatchSets::Direct(ref mut vec) => {
                        vec.push(batch);
                    }
                    BinnedRenderPhaseBatchSets::MultidrawIndirect(ref mut vec) => {
                        // The Bevy renderer will never mark a mesh as batchable
                        // but not multidrawable if multidraw is in use.
                        // However, custom render pipelines might do so, such as
                        // the `specialized_mesh_pipeline` example.
                        vec.push(BinnedRenderPhaseBatchSet {
                            batches: vec![batch],
                            bin_key: key.1.clone(),
                        });
                    }
                }
            }
        }

        // Prepare unbatchables.
        for key in &phase.unbatchable_mesh_keys {
            let unbatchables = phase.unbatchable_mesh_values.get_mut(key).unwrap();

            // Allocate the indirect parameters if necessary.
            let mut indirect_parameters_offset = if no_indirect_drawing {
                None
            } else {
                Some(indirect_parameters_buffer.allocate(unbatchables.entities.len() as u32))
            };

            for &(_, main_entity) in &unbatchables.entities {
                let Some(input_index) = GFBD::get_binned_index(&system_param_item, main_entity)
                else {
                    continue;
                };
                let output_index = data_buffer.add() as u32;

                if let Some(ref mut indirect_parameters_index) = indirect_parameters_offset {
                    // We're in indirect mode, so add an indirect parameters
                    // index.
                    GFBD::write_batch_indirect_parameters(
                        &system_param_item,
                        &mut indirect_parameters_buffer,
                        *indirect_parameters_index,
                        main_entity,
                    );
                    work_item_buffer.buffer.push(PreprocessWorkItem {
                        input_index: input_index.into(),
                        output_index,
                        indirect_parameters_index: *indirect_parameters_index,
                    });
                    unbatchables
                        .buffer_indices
                        .add(UnbatchableBinnedEntityIndices {
                            instance_index: *indirect_parameters_index,
                            extra_index: PhaseItemExtraIndex::IndirectParametersIndex(
                                *indirect_parameters_index..(*indirect_parameters_index + 1),
                            ),
                        });
                    *indirect_parameters_index += 1;
                } else {
                    work_item_buffer.buffer.push(PreprocessWorkItem {
                        input_index: input_index.into(),
                        output_index,
                        indirect_parameters_index: 0,
                    });
                    unbatchables
                        .buffer_indices
                        .add(UnbatchableBinnedEntityIndices {
                            instance_index: output_index,
                            extra_index: PhaseItemExtraIndex::None,
                        });
                }
            }
        }
    }
}

/// A system that writes all instance buffers to the GPU.
pub fn write_batched_instance_buffers<GFBD>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
) where
    GFBD: GetFullBatchData,
{
    let BatchedInstanceBuffers {
        ref mut data_buffer,
        work_item_buffers: ref mut index_buffers,
        ref mut current_input_buffer,
        ref mut previous_input_buffer,
    } = gpu_array_buffer.into_inner();

    data_buffer.write_buffer(&render_device);
    current_input_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
    previous_input_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);

    for index_buffer in index_buffers.values_mut() {
        index_buffer
            .buffer
            .write_buffer(&render_device, &render_queue);
    }
}

pub fn write_indirect_parameters_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut indirect_parameters_buffer: ResMut<IndirectParametersBuffer>,
) {
    indirect_parameters_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
    indirect_parameters_buffer.buffer.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_buffer_correct_behavior() {
        let mut instance_buffer = InstanceInputUniformBuffer::new();

        let index = instance_buffer.add(2);
        instance_buffer.remove(index);
        assert_eq!(instance_buffer.get_unchecked(index), 2);
        assert_eq!(instance_buffer.get(index), None);

        instance_buffer.add(5);
        assert_eq!(instance_buffer.buffer().len(), 1);
    }
}
