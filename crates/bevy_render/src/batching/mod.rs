use std::marker::PhantomData;

use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Res,
    system::{Query, ResMut, Resource, StaticSystemParam, SystemParam, SystemParamItem},
};
use bytemuck::Pod;
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
#[derive(Resource)]
pub enum BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
{
    /// The single buffer containing instances, used when GPU uniform building
    /// isn't available.
    CpuBuilt(GpuArrayBuffer<BD>),

    /// The buffers containing per-instance data used when GPU uniform building
    /// is in use.
    GpuBuilt {
        /// A storage area for the buffer data that the GPU compute shader is
        /// expected to write to.
        ///
        /// There will be one entry for each index.
        data_buffer: UninitBufferVec<BD>,

        /// The index of the buffer data in the current input buffer that
        /// corresponds to each instance.
        ///
        /// It's entirely possible for indices to be duplicated in this list.
        /// This typically occurs when an entity is visible from multiple views:
        /// e.g. the main camera plus a shadow map.
        index_buffer: BufferVec<u32>,

        /// The uniform data inputs for the current frame.
        ///
        /// These are uploaded during the extraction phase.
        current_input_buffer: BufferVec<BDI>,

        /// The uniform data inputs for the previous frame.
        ///
        /// The indices don't generally line up between `current_input_buffer`
        /// and `previous_input_buffer`, because, among other reasons, entities
        /// can spawn or despawn between frames. Instead, each current buffer
        /// data input uniform is expected to contain the index of the
        /// corresponding buffer data input uniform in this list.
        previous_input_buffer: BufferVec<BDI>,

        /// The number of indices this frame.
        ///
        /// This is different from `index_buffer.len()` because `index_buffer`
        /// gets cleared during `write_batched_instance_buffer`.
        index_count: usize,
    },
}

impl<BD, BDI> BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
{
    /// Creates new buffers.
    pub fn new(render_device: &RenderDevice, using_gpu_uniform_builder: bool) -> Self {
        if !using_gpu_uniform_builder {
            return BatchedInstanceBuffers::CpuBuilt(GpuArrayBuffer::new(render_device));
        }

        BatchedInstanceBuffers::GpuBuilt {
            data_buffer: UninitBufferVec::new(BufferUsages::STORAGE),
            index_buffer: BufferVec::new(BufferUsages::STORAGE),
            current_input_buffer: BufferVec::new(BufferUsages::STORAGE),
            previous_input_buffer: BufferVec::new(BufferUsages::STORAGE),
            index_count: 0,
        }
    }

    /// Returns the binding of the uniform buffer that contains the per-instance
    /// data.
    ///
    /// If we're in the GPU uniform building mode, this buffer needs to be
    /// filled in via a compute shader.
    pub fn uniform_binding(&self) -> Option<BindingResource> {
        match *self {
            BatchedInstanceBuffers::CpuBuilt(ref buffer) => buffer.binding(),
            BatchedInstanceBuffers::GpuBuilt {
                ref data_buffer, ..
            } => data_buffer
                .buffer()
                .map(|buffer| buffer.as_entire_binding()),
        }
    }
}

/// A trait to support getting data used for batching draw commands via phase
/// items.
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
    /// The per-instance data that was inserted into the [`BufferVec`] during
    /// extraction.
    ///
    /// This is only used when building uniforms on GPU. If this pipeline
    /// doesn't support GPU uniform building (e.g. the 2D mesh pipeline), this
    /// can safely be `()`.
    type BufferInputData: Pod + Sync + Send;
    /// Get the per-instance data to be inserted into the [`GpuArrayBuffer`].
    /// If the instance can be batched, also return the data used for
    /// comparison when deciding whether draws can be batched, else return None
    /// for the `CompareData`.
    ///
    /// This is only called when building uniforms on CPU. In the GPU uniform
    /// building path, we use [`GetBatchData::get_batch_index`] instead.
    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)>;
    /// Returns the index of the mesh instance in the buffer, if GPU uniform
    /// building is in use.
    ///
    /// This needs only the index, because we already inserted the
    /// [`GetBatchData::BufferInputData`] during the extraction phase before we
    /// got here. If CPU uniform building is in use, this function will never be
    /// called.
    fn get_batch_index(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<(u32, Option<Self::CompareData>)>;
}

/// When implemented on a pipeline, this trait allows the batching logic to
/// compute the per-batch data that will be uploaded to the GPU.
///
/// This includes things like the mesh transforms.
pub trait GetBinnedBatchData {
    /// The system parameters [`GetBinnedBatchData::get_batch_data`] needs
    /// in order to compute the batch data.
    type Param: SystemParam + 'static;
    /// The per-instance data to be inserted into the [`GpuArrayBuffer`]
    /// containing these data for all instances.
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;
    /// The per-instance data that was inserted into the [`BufferVec`] during
    /// extraction.
    ///
    /// This is only used when building uniforms on GPU. If this pipeline
    /// doesn't support GPU uniform building (e.g. the 2D mesh pipeline), this
    /// can safely be `()`.
    type BufferInputData: Pod + Sync + Send;

    /// Get the per-instance data to be inserted into the [`GpuArrayBuffer`].
    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<Self::BufferData>;
    /// Returns the index of the mesh instance in the buffer, if GPU uniform
    /// building is in use.
    ///
    /// This needs only the index, because we already inserted the
    /// [`GetBatchData::BufferInputData`] during the extraction phase before we
    /// got here. If CPU uniform building is in use, this function will never be
    /// called.
    fn get_batch_index(param: &SystemParamItem<Self::Param>, query_item: Entity) -> Option<u32>;
}

/// Batch the items in a sorted render phase. This means comparing metadata
/// needed to draw each phase item and trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase<I, F>(
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<F::BufferData, F::BufferInputData>>,
    mut views: Query<&mut SortedRenderPhase<I>>,
    param: StaticSystemParam<F::Param>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    F: GetBatchData,
{
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    let system_param_item = param.into_inner();

    let mut process_item = |item: &mut I| {
        let compare_data = match gpu_array_buffer {
            BatchedInstanceBuffers::CpuBuilt(ref mut buffer) => {
                let (buffer_data, compare_data) =
                    F::get_batch_data(&system_param_item, item.entity())?;
                let buffer_index = buffer.push(buffer_data);

                let index = buffer_index.index;
                *item.batch_range_mut() = index..index + 1;
                *item.dynamic_offset_mut() = buffer_index.dynamic_offset;

                compare_data
            }

            BatchedInstanceBuffers::GpuBuilt {
                index_buffer,
                data_buffer,
                ..
            } => {
                let (batch_index, compare_data) =
                    F::get_batch_index(&system_param_item, item.entity())?;
                let index_buffer_index = index_buffer.push(batch_index) as u32;
                let data_buffer_index = data_buffer.add() as u32;
                debug_assert_eq!(index_buffer_index, data_buffer_index);
                *item.batch_range_mut() = data_buffer_index..data_buffer_index + 1;

                compare_data
            }
        };

        if I::AUTOMATIC_BATCHING {
            compare_data.map(|compare_data| BatchMeta::new(item, compare_data))
        } else {
            None
        }
    };

    for mut phase in &mut views {
        let items = phase.items.iter_mut().map(|item| {
            let batch_data = process_item(item);
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

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase<BPI, GBBD>(
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<GBBD::BufferData, GBBD::BufferInputData>>,
    mut views: Query<&mut BinnedRenderPhase<BPI>>,
    param: StaticSystemParam<GBBD::Param>,
) where
    BPI: BinnedPhaseItem,
    GBBD: GetBinnedBatchData,
{
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    let system_param_item = param.into_inner();

    for mut phase in &mut views {
        let phase = &mut *phase; // Borrow checker.

        // Prepare batchables.

        for key in &phase.batchable_keys {
            let mut batch_set: SmallVec<[BinnedRenderPhaseBatch; 1]> = smallvec![];
            for &entity in &phase.batchable_values[key] {
                let Some(instance) = add_batch_data_for_binned_render_phase::<GBBD>(
                    entity,
                    &system_param_item,
                    gpu_array_buffer,
                ) else {
                    continue;
                };

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
                if let Some(instance) = add_batch_data_for_binned_render_phase::<GBBD>(
                    entity,
                    &system_param_item,
                    gpu_array_buffer,
                ) {
                    unbatchables.buffer_indices.add(instance);
                }
            }
        }
    }
}

/// Adds the batch data necessary to render one instance of an entity that's in
/// a binned render phase.
fn add_batch_data_for_binned_render_phase<GBBD>(
    entity: Entity,
    system_param_item: &<GBBD::Param as SystemParam>::Item<'_, '_>,
    gpu_array_buffer: &mut BatchedInstanceBuffers<GBBD::BufferData, GBBD::BufferInputData>,
) -> Option<GpuArrayBufferIndex<GBBD::BufferData>>
where
    GBBD: GetBinnedBatchData,
{
    match *gpu_array_buffer {
        BatchedInstanceBuffers::CpuBuilt(ref mut buffer) => {
            let buffer_data = GBBD::get_batch_data(system_param_item, entity)?;
            Some(buffer.push(buffer_data))
        }

        BatchedInstanceBuffers::GpuBuilt {
            ref mut index_buffer,
            ref mut data_buffer,
            ..
        } => {
            let batch_index = GBBD::get_batch_index(system_param_item, entity)?;
            let index_buffer_index = index_buffer.push(batch_index) as u32;
            let data_buffer_index = data_buffer.add() as u32;
            debug_assert_eq!(index_buffer_index, data_buffer_index);
            Some(GpuArrayBufferIndex {
                index: index_buffer_index,
                dynamic_offset: None,
                element_type: PhantomData,
            })
        }
    }
}

pub fn write_batched_instance_buffer<F: GetBatchData>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<F::BufferData, F::BufferInputData>>,
) {
    let gpu_array_buffer = gpu_array_buffer.into_inner();
    match gpu_array_buffer {
        BatchedInstanceBuffers::CpuBuilt(ref mut gpu_array_buffer) => {
            gpu_array_buffer.write_buffer(&render_device, &render_queue);
            gpu_array_buffer.clear();
        }
        BatchedInstanceBuffers::GpuBuilt {
            ref mut data_buffer,
            ref mut index_buffer,
            ref mut current_input_buffer,
            ref mut index_count,
            previous_input_buffer: _,
        } => {
            data_buffer.write_buffer(&render_device);
            index_buffer.write_buffer(&render_device, &render_queue);

            // Save the index count before we clear it out. Rendering will need
            // it.
            *index_count = index_buffer.len();

            current_input_buffer.write_buffer(&render_device, &render_queue);
            // There's no need to write `previous_input_buffer`, as we wrote
            // that on the previous frame, and it hasn't changed.

            data_buffer.clear();
            index_buffer.clear();
            current_input_buffer.clear();
        }
    }
}
