use std::marker::PhantomData;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Res,
    query::With,
    system::{Query, ResMut, Resource, StaticSystemParam, SystemParam, SystemParamItem},
};
use bevy_utils::EntityHashMap;
use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use nonmax::NonMaxU32;
use smallvec::{smallvec, SmallVec};
use wgpu::{BindingResource, BufferUsages};

use crate::{
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhase, BinnedRenderPhaseBatch, CachedRenderPipelinePhaseItem,
        DrawFunctionId, SortedPhaseItem, SortedRenderPhase,
    },
    render_resource::{
        BufferVec, CachedRenderPipelineId, GpuArrayBuffer, GpuArrayBufferIndex, GpuArrayBufferable,
        UninitBufferVec,
    },
    renderer::{RenderDevice, RenderQueue},
    view::ViewTarget,
};

/// Add this component to mesh entities to disable automatic batching
#[derive(Component)]
pub struct NoAutomaticBatching;

/// Data necessary to be equal for two draw commands to be mergeable
///
/// This is based on the following assumptions:
/// - Only entities with prepared assets (pipelines, materials, meshes) are
///   queued to phases
/// - View bindings are constant across a phase for a given draw function as
///   phases are per-view
/// - `batch_and_prepare_render_phase` is the only system that performs this
///   batching and has sole responsibility for preparing the per-object data.
///   As such the mesh binding and dynamic offsets are assumed to only be
///   variable as a result of the `batch_and_prepare_render_phase` system, e.g.
///   due to having to split data across separate uniform bindings within the
///   same buffer due to the maximum uniform buffer binding size.
#[derive(PartialEq)]
struct BatchMeta<T: PartialEq> {
    /// The pipeline id encompasses all pipeline configuration including vertex
    /// buffers and layouts, shaders and their specializations, bind group
    /// layouts, etc.
    pipeline_id: CachedRenderPipelineId,
    /// The draw function id defines the RenderCommands that are called to
    /// set the pipeline and bindings, and make the draw command
    draw_function_id: DrawFunctionId,
    dynamic_offset: Option<NonMaxU32>,
    user_data: T,
}

impl<T: PartialEq> BatchMeta<T> {
    fn new(item: &impl CachedRenderPipelinePhaseItem, user_data: T) -> Self {
        BatchMeta {
            pipeline_id: item.cached_pipeline(),
            draw_function_id: item.draw_function(),
            dynamic_offset: item.dynamic_offset(),
            user_data,
        }
    }
}

/// The GPU buffers holding the data needed to render batches.
///
/// For example, in the 3D PBR pipeline this holds `MeshUniform`s, which are the
/// `BD` type parameter in that mode.
///
/// There are two setups here, one for CPU uniform building and one for GPU
/// uniform building. The CPU uniform setup is simple: there's one *buffer data*
/// (`BD`) type per instance. GPU uniform building has a separate *buffer data
/// input* type (`BDI`), which a compute shader is expected to expand to the
/// full buffer data (`BD`) type. GPU uniform building is generally faster and
/// uses less GPU bus bandwidth, but only implemented for some pipelines (for
/// example, not in the 2D pipeline at present) and only when compute shader is
/// available.
#[derive(Resource, Deref, DerefMut)]
pub struct BatchedCpuBuiltInstanceBuffer<BD>(pub GpuArrayBuffer<BD>)
where
    BD: GpuArrayBufferable + Sync + Send + 'static;

#[derive(Resource)]
pub struct BatchedGpuBuiltInstanceBuffers<BD, BDI>
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
    pub work_item_buffers: EntityHashMap<Entity, BufferVec<PreprocessWorkItem>>,

    /// The uniform data inputs for the current frame.
    ///
    /// These are uploaded during the extraction phase.
    pub current_input_buffer: BufferVec<BDI>,

    /// The uniform data inputs for the previous frame.
    ///
    /// The indices don't generally line up between `current_input_buffer`
    /// and `previous_input_buffer`, because, among other reasons, entities
    /// can spawn or despawn between frames. Instead, each current buffer
    /// data input uniform is expected to contain the index of the
    /// corresponding buffer data input uniform in this list.
    pub previous_input_buffer: BufferVec<BDI>,
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
    pub output_index: u32,
}

impl<BD> BatchedCpuBuiltInstanceBuffer<BD>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
{
    /// Creates new buffers.
    pub fn new(render_device: &RenderDevice) -> Self {
        BatchedCpuBuiltInstanceBuffer(GpuArrayBuffer::new(render_device))
    }

    /// Returns the binding of the buffer that contains the per-instance data.
    ///
    /// If we're in the GPU instance buffer building mode, this buffer needs to
    /// be filled in via a compute shader.
    pub fn instance_data_binding(&self) -> Option<BindingResource> {
        self.binding()
    }
}

impl<BD, BDI> BatchedGpuBuiltInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
{
    /// Creates new buffers.
    pub fn new() -> Self {
        BatchedGpuBuiltInstanceBuffers {
            data_buffer: UninitBufferVec::new(BufferUsages::STORAGE),
            work_item_buffers: EntityHashMap::default(),
            current_input_buffer: BufferVec::new(BufferUsages::STORAGE),
            previous_input_buffer: BufferVec::new(BufferUsages::STORAGE),
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
}

impl<BD, BDI> Default for BatchedGpuBuiltInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A trait to support getting data used for batching draw commands via phase
/// items.
///
/// This is a simple version that only allows for sorting, not binning, as well
/// as only CPU processing, not GPU preprocessing. For these fancier features,
/// see [`GetFullBatchData`].
pub trait GetBatchData {
    /// The system parameters [`GetBatchData::get_batch_data`] needs in
    /// order to compute the batch data.
    type Param: SystemParam + 'static;
    /// Data used for comparison between phase items. If the pipeline id, draw
    /// function id, per-instance data buffer dynamic offset and this data
    /// matches, the draws can be batched.
    type CompareData: PartialEq;
    /// The per-instance data to be inserted into the [`GpuArrayBuffer`]
    /// containing these data for all instances.
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;
    /// Get the per-instance data to be inserted into the [`GpuArrayBuffer`].
    /// If the instance can be batched, also return the data used for
    /// comparison when deciding whether draws can be batched, else return None
    /// for the `CompareData`.
    ///
    /// This is only called when building instance data on CPU. In the GPU
    /// uniform building path, we use
    /// [`GetBatchData::get_batch_preprocess_work_item`] instead.
    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)>;
}

/// A trait to support getting data used for batching draw commands via phase
/// items.
///
/// This version allows for binning and GPU preprocessing.
pub trait GetFullBatchData: GetBatchData {
    /// The per-instance data that was inserted into the [`BufferVec`] during
    /// extraction.
    type BufferInputData: Pod + Sync + Send;

    /// Get the per-instance data to be inserted into the [`GpuArrayBuffer`].
    ///
    /// This is only called when building uniforms on CPU. In the GPU instance
    /// buffer building path, we use
    /// [`GetBinnedBatchData::get_batch_input_index`]
    /// instead.
    fn get_binned_batch_data(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<Self::BufferData>;

    /// Returns the index of the [`GetBatchData::BufferInputData`] that the GPU
    /// preprocessing phase will use.
    ///
    /// We already inserted the [`GetBatchData::BufferInputData`] during the
    /// extraction phase before we got here, so this function shouldn't need to
    /// look up any render data. If CPU instance buffer building is in use, this
    /// function will never be called.
    fn get_batch_input_index(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<(u32, Option<Self::CompareData>)>;

    /// Returns the index of the [`GetBatchData::BufferInputData`] that the GPU
    /// preprocessing phase will use, for the binning path.
    ///
    /// We already inserted the [`GetBatchData::BufferInputData`] during the
    /// extraction phase before we got here, so this function shouldn't need to
    /// look up any render data. If CPU instance buffer building is in use, this
    /// function will never be called.
    fn get_binned_batch_input_index(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<u32>;
}

/// A system that runs early in extraction and clears out all the
/// [`BatchedInstanceBuffers`] for the frame.
///
/// We have to run this during extraction because, if GPU preprocessing is in
/// use, the extraction phase will write to the mesh input uniform buffers
/// directly, so the buffers need to be cleared before then.
pub fn clear_batched_instance_buffers<GFBD>(
    cpu_batched_instance_buffer: Option<ResMut<BatchedCpuBuiltInstanceBuffer<GFBD::BufferData>>>,
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedGpuBuiltInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    >,
) where
    GFBD: GetFullBatchData,
{
    if let Some(mut cpu_batched_instance_buffer) = cpu_batched_instance_buffer {
        cpu_batched_instance_buffer.clear();
    }

    if let Some(mut gpu_batched_instance_buffers) = gpu_batched_instance_buffers {
        gpu_batched_instance_buffers.data_buffer.clear();
        gpu_batched_instance_buffers.current_input_buffer.clear();
        gpu_batched_instance_buffers.previous_input_buffer.clear();
        for work_item_buffer in gpu_batched_instance_buffers.work_item_buffers.values_mut() {
            work_item_buffer.clear();
        }
    }
}

/// A system that removes GPU preprocessing work item buffers that correspond to
/// deleted [`ViewTarget`]s.
///
/// This is a separate system from [`clear_batched_instance_buffers`] because
/// [`ViewTarget`]s aren't created until after the extraction phase is
/// completed.
pub fn delete_old_work_item_buffers<GFBD>(
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedGpuBuiltInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    >,
    view_targets: Query<Entity, With<ViewTarget>>,
) where
    GFBD: GetFullBatchData,
{
    if let Some(mut gpu_batched_instance_buffers) = gpu_batched_instance_buffers {
        gpu_batched_instance_buffers
            .work_item_buffers
            .retain(|entity, _| view_targets.contains(*entity));
    }
}

/// Batch the items in a sorted render phase, when GPU instance buffer building
/// isn't in use. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase_no_gpu_preprocessing<I, GBD>(
    cpu_batched_instance_buffer: Option<ResMut<BatchedCpuBuiltInstanceBuffer<GBD::BufferData>>>,
    mut views: Query<&mut SortedRenderPhase<I>>,
    param: StaticSystemParam<GBD::Param>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    GBD: GetBatchData,
{
    let system_param_item = param.into_inner();

    let process_item = |item: &mut I, buffer: &mut GpuArrayBuffer<GBD::BufferData>| {
        let (buffer_data, compare_data) = GBD::get_batch_data(&system_param_item, item.entity())?;
        let buffer_index = buffer.push(buffer_data);

        let index = buffer_index.index;
        *item.batch_range_mut() = index..index + 1;
        *item.dynamic_offset_mut() = buffer_index.dynamic_offset;

        if I::AUTOMATIC_BATCHING {
            compare_data.map(|compare_data| BatchMeta::new(item, compare_data))
        } else {
            None
        }
    };

    // We only process CPU-built batch data in this function.
    let Some(cpu_batched_instance_buffers) = cpu_batched_instance_buffer else {
        return;
    };
    let cpu_batched_instance_buffers = cpu_batched_instance_buffers.into_inner();

    for mut phase in &mut views {
        let items = phase.items.iter_mut().map(|item| {
            let batch_data = process_item(item, cpu_batched_instance_buffers);
            (item.batch_range_mut(), batch_data)
        });
        items.reduce(|(start_range, prev_batch_meta), (range, batch_meta)| {
            if batch_meta.is_some() && prev_batch_meta == batch_meta {
                start_range.end = range.end;
                (start_range, prev_batch_meta)
            } else {
                (range, batch_meta)
            }
        });
    }
}

/// Batch the items in a sorted render phase, when GPU instance buffer building
/// isn't in use. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase_for_gpu_preprocessing<I, GFBD>(
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedGpuBuiltInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    >,
    mut views: Query<(Entity, &mut SortedRenderPhase<I>)>,
    param: StaticSystemParam<GFBD::Param>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    GFBD: GetFullBatchData,
{
    let system_param_item = param.into_inner();

    let process_item =
        |item: &mut I,
         data_buffer: &mut UninitBufferVec<GFBD::BufferData>,
         work_item_buffer: &mut BufferVec<PreprocessWorkItem>| {
            let (input_index, compare_data) =
                GFBD::get_batch_input_index(&system_param_item, item.entity())?;
            let output_index = data_buffer.add() as u32;

            work_item_buffer.push(PreprocessWorkItem {
                input_index,
                output_index,
            });

            *item.batch_range_mut() = output_index..output_index + 1;

            if I::AUTOMATIC_BATCHING {
                compare_data.map(|compare_data| BatchMeta::new(item, compare_data))
            } else {
                None
            }
        };

    // We only process GPU-built batch data in this function.
    let Some(gpu_batched_instance_buffers) = gpu_batched_instance_buffers else {
        return;
    };
    let BatchedGpuBuiltInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ..
    } = gpu_batched_instance_buffers.into_inner();

    for (view, mut phase) in &mut views {
        // Create the work item buffer if necessary; otherwise, just mark it as
        // used this frame.
        let work_item_buffer = work_item_buffers
            .entry(view)
            .or_insert_with(|| BufferVec::new(BufferUsages::STORAGE));

        let items = phase.items.iter_mut().map(|item| {
            let batch_data = process_item(item, data_buffer, work_item_buffer);
            (item.batch_range_mut(), batch_data)
        });
        items.reduce(|(start_range, prev_batch_meta), (range, batch_meta)| {
            if batch_meta.is_some() && prev_batch_meta == batch_meta {
                start_range.end = range.end;
                (start_range, prev_batch_meta)
            } else {
                (range, batch_meta)
            }
        });
    }
}

/// Sorts a render phase that uses bins.
pub fn sort_binned_render_phase<BPI>(mut views: Query<&mut BinnedRenderPhase<BPI>>)
where
    BPI: BinnedPhaseItem,
{
    for mut phase in &mut views {
        phase.batchable_keys.sort_unstable();
        phase.unbatchable_keys.sort_unstable();
    }
}

/// Creates batches for a render phase that uses bins, when GPU batch data
/// building isn't in use.
pub fn batch_and_prepare_binned_render_phase_no_gpu_preprocessing<BPI, GFBD>(
    cpu_batched_instance_buffer: Option<ResMut<BatchedCpuBuiltInstanceBuffer<GFBD::BufferData>>>,
    mut views: Query<&mut BinnedRenderPhase<BPI>>,
    param: StaticSystemParam<GFBD::Param>,
) where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    let system_param_item = param.into_inner();

    // We only process CPU-built batch data in this function.
    let Some(mut buffer) = cpu_batched_instance_buffer else {
        return;
    };

    for mut phase in &mut views {
        let phase = &mut *phase; // Borrow checker.

        // Prepare batchables.

        for key in &phase.batchable_keys {
            let mut batch_set: SmallVec<[BinnedRenderPhaseBatch; 1]> = smallvec![];
            for &entity in &phase.batchable_values[key] {
                let Some(buffer_data) = GFBD::get_binned_batch_data(&system_param_item, entity)
                else {
                    continue;
                };
                let instance = buffer.push(buffer_data);

                // If the dynamic offset has changed, flush the batch.
                //
                // This is the only time we ever have more than one batch per
                // bin. Note that dynamic offsets are only used on platforms
                // with no storage buffers.
                if !batch_set.last().is_some_and(|batch| {
                    batch.instance_range.end == instance.index
                        && batch.dynamic_offset == instance.dynamic_offset
                }) {
                    batch_set.push(BinnedRenderPhaseBatch {
                        representative_entity: entity,
                        instance_range: instance.index..instance.index,
                        dynamic_offset: instance.dynamic_offset,
                    });
                }

                if let Some(batch) = batch_set.last_mut() {
                    batch.instance_range.end = instance.index + 1;
                }
            }

            phase.batch_sets.push(batch_set);
        }

        // Prepare unbatchables.
        for key in &phase.unbatchable_keys {
            let unbatchables = phase.unbatchable_values.get_mut(key).unwrap();
            for &entity in &unbatchables.entities {
                let Some(buffer_data) = GFBD::get_binned_batch_data(&system_param_item, entity)
                else {
                    continue;
                };
                let instance = buffer.push(buffer_data);
                unbatchables.buffer_indices.add(instance);
            }
        }
    }
}

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase_for_gpu_preprocessing<BPI, GFBD>(
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedGpuBuiltInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    >,
    mut views: Query<(Entity, &mut BinnedRenderPhase<BPI>)>,
    param: StaticSystemParam<GFBD::Param>,
) where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    let system_param_item = param.into_inner();

    // We only process GPU-built batch data in this function.
    let Some(gpu_batched_instance_buffers) = gpu_batched_instance_buffers else {
        return;
    };
    let BatchedGpuBuiltInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ..
    } = gpu_batched_instance_buffers.into_inner();

    for (view, mut phase) in &mut views {
        let phase = &mut *phase; // Borrow checker.

        // Create the work item buffer if necessary; otherwise, just mark it as
        // used this frame.
        let work_item_buffer = work_item_buffers
            .entry(view)
            .or_insert_with(|| BufferVec::new(BufferUsages::STORAGE));

        // Prepare batchables.

        for key in &phase.batchable_keys {
            let mut batch: Option<BinnedRenderPhaseBatch> = None;
            for &entity in &phase.batchable_values[key] {
                let Some(input_index) =
                    GFBD::get_binned_batch_input_index(&system_param_item, entity)
                else {
                    continue;
                };
                let output_index = data_buffer.add() as u32;

                work_item_buffer.push(PreprocessWorkItem {
                    input_index,
                    output_index,
                });

                batch
                    .get_or_insert(BinnedRenderPhaseBatch {
                        representative_entity: entity,
                        instance_range: output_index..output_index,
                        dynamic_offset: None,
                    })
                    .instance_range
                    .end = output_index + 1;
            }

            if let Some(batch) = batch {
                phase.batch_sets.push(smallvec![batch]);
            }
        }

        // Prepare unbatchables.
        for key in &phase.unbatchable_keys {
            let unbatchables = phase.unbatchable_values.get_mut(key).unwrap();
            for &entity in &unbatchables.entities {
                let Some(input_index) =
                    GFBD::get_binned_batch_input_index(&system_param_item, entity)
                else {
                    continue;
                };
                let output_index = data_buffer.add() as u32;

                work_item_buffer.push(PreprocessWorkItem {
                    input_index,
                    output_index,
                });

                unbatchables
                    .buffer_indices
                    .add(GpuArrayBufferIndex::<GFBD::BufferData> {
                        index: output_index,
                        dynamic_offset: None,
                        element_type: PhantomData,
                    });
            }
        }
    }
}

pub fn write_cpu_built_batched_instance_buffers<GBD>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    cpu_batched_instance_buffer: Option<ResMut<BatchedCpuBuiltInstanceBuffer<GBD::BufferData>>>,
) where
    GBD: GetBatchData,
{
    if let Some(mut cpu_batched_instance_buffer) = cpu_batched_instance_buffer {
        cpu_batched_instance_buffer.write_buffer(&render_device, &render_queue);
    }
}

pub fn write_gpu_built_batched_instance_buffers<GFBD>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedGpuBuiltInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    >,
) where
    GFBD: GetFullBatchData,
{
    let Some(mut gpu_batched_instance_buffers) = gpu_batched_instance_buffers else {
        return;
    };

    gpu_batched_instance_buffers
        .data_buffer
        .write_buffer(&render_device);
    gpu_batched_instance_buffers
        .current_input_buffer
        .write_buffer(&render_device, &render_queue);
    // There's no need to write `previous_input_buffer`, as we wrote
    // that on the previous frame, and it hasn't changed.

    for work_item_buffer in gpu_batched_instance_buffers.work_item_buffers.values_mut() {
        work_item_buffer.write_buffer(&render_device, &render_queue);
    }
}
