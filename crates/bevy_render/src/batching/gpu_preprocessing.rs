//! Batching functionality when GPU preprocessing is in use.

use std::marker::PhantomData;

use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Query, Res, ResMut, Resource, StaticSystemParam},
};
use bevy_encase_derive::ShaderType;
use bevy_utils::EntityHashMap;
use bytemuck::{Pod, Zeroable};
use smallvec::smallvec;
use wgpu::{BindingResource, BufferUsages};

use crate::{
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhase, BinnedRenderPhaseBatch, CachedRenderPipelinePhaseItem,
        SortedPhaseItem, SortedRenderPhase,
    },
    render_resource::{BufferVec, GpuArrayBufferIndex, GpuArrayBufferable, UninitBufferVec},
    renderer::{RenderDevice, RenderQueue},
    view::ViewTarget,
};

use super::{BatchMeta, GetFullBatchData};

/// The GPU buffers holding the data needed to render batches.
///
/// For example, in the 3D PBR pipeline this holds `MeshUniform`s, which are the
/// `BD` type parameter in that mode.
///
/// We have a separate *buffer data input* type (`BDI`) here, which a compute
/// shader is expected to expand to the full buffer data (`BD`) type. GPU
/// uniform building is generally faster and uses less GPU bus bandwidth, but
/// only implemented for some pipelines (for example, not in the 2D pipeline at
/// present) and only when compute shader is available.
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

impl<BD, BDI> Default for BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod,
{
    fn default() -> Self {
        Self::new()
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
        ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
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
pub fn batch_and_prepare_sorted_render_phase<I, GFBD>(
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
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
    let BatchedInstanceBuffers {
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

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase<BPI, GFBD>(
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
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
    let BatchedInstanceBuffers {
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

/// A system that writes all instance buffers to the GPU.
pub fn write_batched_instance_buffers<GFBD>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_batched_instance_buffers: Option<
        ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
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
