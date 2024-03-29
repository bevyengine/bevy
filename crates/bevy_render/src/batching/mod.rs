use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Res,
    system::{Query, ResMut, Resource, StaticSystemParam, SystemParam, SystemParamItem},
};
use bytemuck::Pod;
use nonmax::NonMaxU32;
use wgpu::{BindingResource, BufferUsages};

use crate::{
    render_phase::{CachedRenderPipelinePhaseItem, DrawFunctionId, RenderPhase},
    render_resource::{
        BufferVec, CachedRenderPipelineId, GpuArrayBuffer, GpuArrayBufferable, UninitBufferVec,
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

#[derive(Resource)]
pub enum BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
{
    CpuBuilt(GpuArrayBuffer<BD>),
    GpuBuilt {
        data_buffer: UninitBufferVec<BD>,
        index_buffer: BufferVec<u32>,
        current_input_buffer: BufferVec<BDI>,
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
    type Param: SystemParam + 'static;
    /// Data used for comparison between phase items. If the pipeline id, draw
    /// function id, per-instance data buffer dynamic offset and this data
    /// matches, the draws can be batched.
    type CompareData: PartialEq;
    /// The per-instance data to be inserted into the [`GpuArrayBuffer`]
    /// containing these data for all instances.
    type BufferData: GpuArrayBufferable + Sync + Send + 'static;
    type BufferInputData: Pod + Sync + Send;
    /// Get the per-instance data to be inserted into the [`GpuArrayBuffer`].
    /// If the instance can be batched, also return the data used for
    /// comparison when deciding whether draws can be batched, else return None
    /// for the `CompareData`.
    fn get_batch_data(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)>;
    /// Same as the above, but for GPU uniform building.
    fn get_batch_index(
        param: &SystemParamItem<Self::Param>,
        query_item: Entity,
    ) -> Option<(u32, Option<Self::CompareData>)>;
}

/// Batch the items in a render phase. This means comparing metadata needed to draw each phase item
/// and trying to combine the draws into a batch.
pub fn batch_and_prepare_render_phase<I, F>(
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<F::BufferData, F::BufferInputData>>,
    mut views: Query<&mut RenderPhase<I>>,
    param: StaticSystemParam<F::Param>,
) where
    I: CachedRenderPipelinePhaseItem,
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
