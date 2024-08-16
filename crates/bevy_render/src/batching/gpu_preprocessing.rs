//! Batching functionality when GPU preprocessing is in use.

use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::Entity,
    query::{Has, With},
    schedule::IntoSystemConfigs as _,
    system::{Query, Res, ResMut, Resource, StaticSystemParam},
    world::{FromWorld, World},
};
use bevy_encase_derive::ShaderType;
use bevy_utils::EntityHashMap;
use bytemuck::{Pod, Zeroable};
use nonmax::NonMaxU32;
use smallvec::smallvec;
use wgpu::{BindingResource, BufferUsages, DownlevelFlags, Features};

use crate::{
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhaseBatch, CachedRenderPipelinePhaseItem,
        PhaseItemExtraIndex, SortedPhaseItem, SortedRenderPhase, UnbatchableBinnedEntityIndices,
        ViewBinnedRenderPhases, ViewSortedRenderPhases,
    },
    render_resource::{BufferVec, GpuArrayBufferable, RawBufferVec, UninitBufferVec},
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    view::{GpuCulling, ViewTarget},
    Render, RenderApp, RenderSet,
};

use super::{BatchMeta, GetBatchData, GetFullBatchData};

pub struct BatchingPlugin;

impl Plugin for BatchingPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
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
pub enum GpuPreprocessingSupport {
    /// No GPU preprocessing support is available at all.
    None,
    /// GPU preprocessing is available, but GPU culling isn't.
    PreprocessingOnly,
    /// Both GPU preprocessing and GPU culling are available.
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
    BDI: Pod,
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
    pub work_item_buffers: EntityHashMap<Entity, PreprocessWorkItemBuffer>,

    /// The uniform data inputs for the current frame.
    ///
    /// These are uploaded during the extraction phase.
    pub current_input_buffer: RawBufferVec<BDI>,

    /// The uniform data inputs for the previous frame.
    ///
    /// The indices don't generally line up between `current_input_buffer`
    /// and `previous_input_buffer`, because, among other reasons, entities
    /// can spawn or despawn between frames. Instead, each current buffer
    /// data input uniform is expected to contain the index of the
    /// corresponding buffer data input uniform in this list.
    pub previous_input_buffer: RawBufferVec<BDI>,
}

/// The buffer of GPU preprocessing work items for a single view.
pub struct PreprocessWorkItemBuffer {
    /// The buffer of work items.
    pub buffer: BufferVec<PreprocessWorkItem>,
    /// True if we're using GPU culling.
    pub gpu_culling: bool,
}

/// One invocation of the preprocessing shader: i.e. one mesh instance in a
/// view.
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct PreprocessWorkItem {
    /// The index of the batch input data in the input buffer that the shader
    /// reads from.
    pub input_index: u32,
    /// In direct mode, this is the index of the `MeshUniform` in the output
    /// buffer that we write to. In indirect mode, this is the index of the
    /// [`IndirectParameters`].
    pub output_index: u32,
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

    /// The index of the first vertex we're to draw.
    pub first_vertex: u32,

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
#[derive(Resource, Deref, DerefMut)]
pub struct IndirectParametersBuffer(pub BufferVec<IndirectParameters>);

impl IndirectParametersBuffer {
    /// Creates the indirect parameters buffer.
    pub fn new() -> IndirectParametersBuffer {
        IndirectParametersBuffer(BufferVec::new(
            BufferUsages::STORAGE | BufferUsages::INDIRECT,
        ))
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

        if device.limits().max_compute_workgroup_size_x == 0 ||
            // filter lower end / older devices on Android as they crash when using GPU preprocessing
            (cfg!(target_os = "android") && adapter.get_info().name.starts_with("Adreno (TM) 6"))
        {
            GpuPreprocessingSupport::None
        } else if !device
            .features()
            .contains(Features::INDIRECT_FIRST_INSTANCE) ||
            !adapter.get_downlevel_capabilities().flags.contains(
        DownlevelFlags::VERTEX_AND_INSTANCE_INDEX_RESPECTS_RESPECTIVE_FIRST_VALUE_IN_INDIRECT_DRAW)
        {
            GpuPreprocessingSupport::PreprocessingOnly
        } else {
            GpuPreprocessingSupport::Culling
        }
    }
}

impl<BD, BDI> BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
{
    /// Creates new buffers.
    pub fn new() -> Self {
        BatchedInstanceBuffers {
            data_buffer: UninitBufferVec::new(BufferUsages::STORAGE),
            work_item_buffers: EntityHashMap::default(),
            current_input_buffer: RawBufferVec::new(BufferUsages::STORAGE),
            previous_input_buffer: RawBufferVec::new(BufferUsages::STORAGE),
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
        self.current_input_buffer.clear();
        self.previous_input_buffer.clear();
        for work_item_buffer in self.work_item_buffers.values_mut() {
            work_item_buffer.buffer.clear();
        }
    }
}

impl<BD, BDI> Default for BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
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
/// deleted [`ViewTarget`]s.
///
/// This is a separate system from [`clear_batched_gpu_instance_buffers`]
/// because [`ViewTarget`]s aren't created until after the extraction phase is
/// completed.
pub fn delete_old_work_item_buffers<GFBD>(
    mut gpu_batched_instance_buffers: ResMut<
        BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
    >,
    view_targets: Query<Entity, With<ViewTarget>>,
) where
    GFBD: GetFullBatchData,
{
    gpu_batched_instance_buffers
        .work_item_buffers
        .retain(|entity, _| view_targets.contains(*entity));
}

/// Batch the items in a sorted render phase, when GPU instance buffer building
/// is in use. This means comparing metadata needed to draw each phase item and
/// trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase<I, GFBD>(
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    mut indirect_parameters_buffer: ResMut<IndirectParametersBuffer>,
    mut sorted_render_phases: ResMut<ViewSortedRenderPhases<I>>,
    mut views: Query<(Entity, Has<GpuCulling>)>,
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

    for (view, gpu_culling) in &mut views {
        let Some(phase) = sorted_render_phases.get_mut(&view) else {
            continue;
        };

        // Create the work item buffer if necessary.
        let work_item_buffer =
            work_item_buffers
                .entry(view)
                .or_insert_with(|| PreprocessWorkItemBuffer {
                    buffer: BufferVec::new(BufferUsages::STORAGE),
                    gpu_culling,
                });

        // Walk through the list of phase items, building up batches as we go.
        let mut batch: Option<SortedRenderBatch<GFBD>> = None;
        for current_index in 0..phase.items.len() {
            // Get the index of the input data, and comparison metadata, for
            // this entity.
            let current_batch_input_index = GFBD::get_index_and_compare_data(
                &system_param_item,
                phase.items[current_index].entity(),
            );

            // Unpack that index and metadata. Note that it's possible for index
            // and/or metadata to not be present, which signifies that this
            // entity is unbatchable. In that case, we break the batch here.
            let (mut current_input_index, mut current_meta) = (None, None);
            if let Some((input_index, maybe_meta)) = current_batch_input_index {
                current_input_index = Some(input_index);
                current_meta =
                    maybe_meta.map(|meta| BatchMeta::new(&phase.items[current_index], meta));
            }

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
            let current_entity = phase.items[current_index].entity();
            let output_index = data_buffer.add() as u32;

            // If we can't batch, break the existing batch and make a new one.
            if !can_batch {
                // Break a batch if we need to.
                if let Some(batch) = batch.take() {
                    batch.flush(output_index, phase);
                }

                // Start a new batch.
                let indirect_parameters_index = if gpu_culling {
                    GFBD::get_batch_indirect_parameters_index(
                        &system_param_item,
                        &mut indirect_parameters_buffer,
                        current_entity,
                        output_index,
                    )
                } else {
                    None
                };
                batch = Some(SortedRenderBatch {
                    phase_item_start_index: current_index as u32,
                    instance_start_index: output_index,
                    indirect_parameters_index,
                    meta: current_meta,
                });
            }

            // Add a new preprocessing work item so that the preprocessing
            // shader will copy the per-instance data over.
            if let (Some(batch), Some(input_index)) = (batch.as_ref(), current_input_index.as_ref())
            {
                work_item_buffer.buffer.push(PreprocessWorkItem {
                    input_index: (*input_index).into(),
                    output_index: match batch.indirect_parameters_index {
                        Some(indirect_parameters_index) => indirect_parameters_index.into(),
                        None => output_index,
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
    mut views: Query<(Entity, Has<GpuCulling>)>,
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

    for (view, gpu_culling) in &mut views {
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
                    gpu_culling,
                });

        // Prepare batchables.

        for key in &phase.batchable_keys {
            let mut batch: Option<BinnedRenderPhaseBatch> = None;
            for &entity in &phase.batchable_values[key] {
                let Some(input_index) = GFBD::get_binned_index(&system_param_item, entity) else {
                    continue;
                };
                let output_index = data_buffer.add() as u32;

                match batch {
                    Some(ref mut batch) => {
                        batch.instance_range.end = output_index + 1;
                        work_item_buffer.buffer.push(PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index: batch
                                .extra_index
                                .as_indirect_parameters_index()
                                .unwrap_or(output_index),
                        });
                    }

                    None if gpu_culling => {
                        let indirect_parameters_index = GFBD::get_batch_indirect_parameters_index(
                            &system_param_item,
                            &mut indirect_parameters_buffer,
                            entity,
                            output_index,
                        );
                        work_item_buffer.buffer.push(PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index: indirect_parameters_index.unwrap_or_default().into(),
                        });
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: entity,
                            instance_range: output_index..output_index + 1,
                            extra_index: PhaseItemExtraIndex::maybe_indirect_parameters_index(
                                indirect_parameters_index,
                            ),
                        });
                    }

                    None => {
                        work_item_buffer.buffer.push(PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index,
                        });
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: entity,
                            instance_range: output_index..output_index + 1,
                            extra_index: PhaseItemExtraIndex::NONE,
                        });
                    }
                }
            }

            if let Some(batch) = batch {
                phase.batch_sets.push(smallvec![batch]);
            }
        }

        // Prepare unbatchables.
        for key in &phase.unbatchable_keys {
            let unbatchables = phase.unbatchable_values.get_mut(key).unwrap();
            for &entity in &unbatchables.entities {
                let Some(input_index) = GFBD::get_binned_index(&system_param_item, entity) else {
                    continue;
                };
                let output_index = data_buffer.add() as u32;

                if gpu_culling {
                    let indirect_parameters_index = GFBD::get_batch_indirect_parameters_index(
                        &system_param_item,
                        &mut indirect_parameters_buffer,
                        entity,
                        output_index,
                    )
                    .unwrap_or_default();
                    work_item_buffer.buffer.push(PreprocessWorkItem {
                        input_index: input_index.into(),
                        output_index: indirect_parameters_index.into(),
                    });
                    unbatchables
                        .buffer_indices
                        .add(UnbatchableBinnedEntityIndices {
                            instance_index: indirect_parameters_index.into(),
                            extra_index: PhaseItemExtraIndex::indirect_parameters_index(
                                indirect_parameters_index.into(),
                            ),
                        });
                } else {
                    work_item_buffer.buffer.push(PreprocessWorkItem {
                        input_index: input_index.into(),
                        output_index,
                    });
                    unbatchables
                        .buffer_indices
                        .add(UnbatchableBinnedEntityIndices {
                            instance_index: output_index,
                            extra_index: PhaseItemExtraIndex::NONE,
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
        previous_input_buffer: _,
    } = gpu_array_buffer.into_inner();

    data_buffer.write_buffer(&render_device);
    current_input_buffer.write_buffer(&render_device, &render_queue);
    // There's no need to write `previous_input_buffer`, as we wrote
    // that on the previous frame, and it hasn't changed.

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
    indirect_parameters_buffer.write_buffer(&render_device, &render_queue);
    indirect_parameters_buffer.clear();
}
