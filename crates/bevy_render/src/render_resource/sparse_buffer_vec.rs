//! GPU buffers that support sparse updates if only a small number of elements
//! have changed.

use alloc::sync::{Arc, Weak};
use core::{
    slice,
    sync::atomic::{AtomicU64, Ordering},
};

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use bevy_log::{debug, error, info};
use bevy_material::{
    bind_group_layout_entries::{
        binding_types::{storage_buffer, storage_buffer_read_only, uniform_buffer},
        BindGroupLayoutEntries,
    },
    descriptor::{BindGroupLayoutDescriptor, CachedComputePipelineId, ComputePipelineDescriptor},
};
use bevy_shader::Shader;
use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use weak_table::WeakKeyHashMap;
use wgpu::{BufferDescriptor, BufferUsages, ComputePassDescriptor, ShaderStages};

use crate::{
    diagnostic::RecordDiagnostics as _,
    render_resource::{
        AtomicPod, BindGroup, BindGroupEntries, Buffer, PipelineCache, RawBufferVec,
        SpecializedComputePipeline, SpecializedComputePipelines, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue},
    ExtractSchedule, RenderApp,
};

/// A plugin that allows sparse updates of GPU buffers if only a small number of
/// elements have changed.
pub struct SparseBufferPlugin;

impl Plugin for SparseBufferPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sparse_buffer_update.wgsl");
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SparseBufferUpdateJobs>()
            .init_resource::<SparseBufferUpdatePipelines>()
            .init_resource::<SpecializedComputePipelines<SparseBufferUpdatePipelines>>()
            .init_resource::<SparseBufferUpdateBindGroups>()
            .add_systems(ExtractSchedule, clear_sparse_buffer_jobs)
            .add_systems(
                RenderGraph,
                // We perform sparse buffer updates very early so that sparse
                // buffers can be used in any render pass.
                update_sparse_buffers.in_set(RenderGraphSystems::Begin),
            );
    }
}

/// A globally-unique ID that identifies this sparse buffer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deref, DerefMut)]
pub struct SparseBufferId(pub u64);

/// An object that allows the sparse buffer ID to be query and holds the bind
/// group for that sparse buffer alive.
///
/// Each sparse buffer holds a strong reference to this handle, and the
/// [`SparseBufferUpdateBindGroups`] resource contains a weak map from this
/// handle to the bind group. This setup ensures that, when the sparse buffer is
/// freed, the bind groups for that sparse buffer are freed as well.
pub type SparseBufferHandle = Arc<SparseBufferId>;

/// The next sparse buffer ID to be assigned.
static NEXT_SPARSE_BUFFER_ID: AtomicU64 = AtomicU64::new(0);

/// The size of a single workgroup in the sparse buffer shader.
const SPARSE_BUFFER_UPDATE_WORKGROUP_SIZE: u32 = 256;

/// The fraction of the buffer that may be changed before we fall back to full
/// reupload.
///
/// This is set to 15% by default. This was obtained experimentally by testing
/// very large scenes and roughly matches the values used by other engines that
/// perform sparse buffer updates.
const SPARSE_UPLOAD_THRESHOLD: f64 = 0.15;

/// The WebGPU limit on the number of workgroups that can be dispatched.
const MAX_WORKGROUPS: u32 = 65535;

/// We round all allocations up to the nearest power of this.
const REALLOCATION_FACTOR: f64 = 1.5;
/// We round all allocations up to the nearest multiple of this.
const REALLOCATION_SIZE_MULTIPLE: usize = 256;

/// The number of bits packed into each [`AtomicU64`] word.
const BITS_PER_WORD: u32 = 64;

/// Pipelines for the sparse buffer update shader.
///
/// This shader is shared among all sparse buffer vectors.
#[derive(Resource)]
pub struct SparseBufferUpdatePipelines {
    /// The bind group layout.
    ///
    /// We only have one bind group layout shared among all sparse buffer
    /// vectors.
    bind_group_layout: Option<BindGroupLayoutDescriptor>,
    /// The shader that performs the scatter operation.
    shader: Option<Handle<Shader>>,
}

/// A resource, part of the render world, that stores the bind groups for each
/// sparse buffer.
#[derive(Resource)]
pub struct SparseBufferUpdateBindGroups {
    /// The bind groups for each sparse buffer.
    ///
    /// These are stored in a weak map so that when the sparse buffer goes away,
    /// the bind group for that buffer goes away as well.
    bind_groups: WeakKeyHashMap<Weak<SparseBufferId>, SparseBufferUpdateBindGroup>,
    /// The ID of the update shader pipeline shared among all sparse buffers.
    pipeline_id: CachedComputePipelineId,
}

/// A single bind group for the sparse buffer update shader.
pub struct SparseBufferUpdateBindGroup {
    /// The actual bind group.
    bind_group: BindGroup,
}

/// A resource, part of the render world, that stores all pending sparse updates
/// to buffers.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct SparseBufferUpdateJobs(pub Vec<SparseBufferUpdateJob>);

/// Describes a sparse update operation for a buffer.
pub struct SparseBufferUpdateJob {
    /// A handle to the buffer to be updated.
    sparse_buffer_handle: SparseBufferHandle,
    /// The number of elements to update.
    updated_element_count: u32,
    /// The size of each element in 32-bit words.
    element_word_size: u32,
}

impl SparseBufferUpdateJob {
    /// Calculates the number of words that need to be updated.
    fn words_to_update(&self) -> u32 {
        self.updated_element_count * self.element_word_size
    }

    /// Calculates the number of workgroups that need to be dispatched.
    fn workgroup_count(&self) -> u32 {
        self.words_to_update()
            .div_ceil(SPARSE_BUFFER_UPDATE_WORKGROUP_SIZE)
    }
}

/// A GPU type that describes a sparse update that is to be performed.
#[derive(Clone, Copy, Default, ShaderType, Pod, Zeroable)]
#[repr(C)]
struct GpuSparseBufferUpdateMetadata {
    /// The size of a single element in 32-bit words.
    element_size: u32,
    /// The number of elements that need to be updated.
    updated_element_count: u32,
}

/// A system, part of the render graph, that performs sparse buffer updates to
/// buffers for which only a small number of elements have changed.
///
/// This runs as early in the pipeline as possible so that sparse buffers can be
/// used for any subsequent pass.
fn update_sparse_buffers(
    sparse_buffer_update_jobs: Res<SparseBufferUpdateJobs>,
    sparse_buffer_update_bind_groups: Res<SparseBufferUpdateBindGroups>,
    pipeline_cache: Res<PipelineCache>,
    mut render_context: RenderContext,
) {
    // Bail if we have nothing to do.
    if sparse_buffer_update_jobs.is_empty() {
        return;
    }

    let Some(compute_pipeline) =
        pipeline_cache.get_compute_pipeline(sparse_buffer_update_bind_groups.pipeline_id)
    else {
        return;
    };

    let diagnostics = render_context.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let command_encoder = render_context.command_encoder();

    let mut sparse_buffer_update_pass =
        command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("sparse buffer updates"),
            timestamp_writes: None,
        });
    sparse_buffer_update_pass.set_pipeline(compute_pipeline);
    let time_span = diagnostics.time_span(&mut sparse_buffer_update_pass, "sparse buffer updates");

    // Process each sparse buffer update job.
    for sparse_buffer_update_job in sparse_buffer_update_jobs.iter() {
        let Some(sparse_buffer_update_bind_group) = sparse_buffer_update_bind_groups
            .bind_groups
            .get(&sparse_buffer_update_job.sparse_buffer_handle)
        else {
            continue;
        };

        sparse_buffer_update_pass.set_bind_group(
            0,
            &sparse_buffer_update_bind_group.bind_group,
            &[],
        );
        sparse_buffer_update_pass.dispatch_workgroups(
            sparse_buffer_update_job.workgroup_count(),
            1,
            1,
        );
    }

    time_span.end(&mut sparse_buffer_update_pass);
}

/// A system that clears out the sparse buffer update jobs in preparation for a
/// new frame.
fn clear_sparse_buffer_jobs(mut sparse_buffer_update_jobs: ResMut<SparseBufferUpdateJobs>) {
    sparse_buffer_update_jobs.clear();
}

impl FromWorld for SparseBufferUpdatePipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let limit = render_device.limits().max_storage_buffers_per_shader_stage;

        if limit < 3 {
            info!(
                "Sparse buffer updates disabled. RenderDevice lacks support: max_storage_buffers_per_shader_stage ({}) < 3.",
                limit
            );

            return SparseBufferUpdatePipelines {
                bind_group_layout: None,
                shader: None,
            };
        }

        let bind_group_layout = BindGroupLayoutDescriptor::new(
            "sparse buffer update bind group layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // @group(0) @binding(0) var<storage, read_write> dest_buffer: array<u32>;
                    storage_buffer::<u32>(false),
                    // @group(0) @binding(1) var<storage> src_buffer: array<u32>;
                    storage_buffer_read_only::<u32>(false),
                    // @group(0) @binding(2) var<storage> indices: array<u32>;
                    storage_buffer_read_only::<u32>(false),
                    // @group(0) @binding(3) var<uniform> metadata:
                    // SparseBufferUpdateMetadata;
                    uniform_buffer::<GpuSparseBufferUpdateMetadata>(false),
                ),
            ),
        );

        SparseBufferUpdatePipelines {
            bind_group_layout: Some(bind_group_layout),
            shader: Some(load_embedded_asset!(world, "sparse_buffer_update.wgsl")),
        }
    }
}

impl SpecializedComputePipeline for SparseBufferUpdatePipelines {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("sparse buffer update pipeline".into()),
            layout: self.bind_group_layout.clone().into_iter().collect(),
            shader: self.shader.clone().unwrap_or_default(),
            shader_defs: vec![],
            ..ComputePipelineDescriptor::default()
        }
    }
}

/// The buffers that we use to sparsely scatter new data to the GPU.
///
/// There's one such set of buffers per sparse buffer vector.
struct SparseBufferStagingBuffers {
    /// All elements that have changed and need to be updated.
    source_data: RawBufferVec<u32>,

    /// The index at which we write each element in [`Self::source_data`].
    ///
    /// The length of this buffer is equal to [`Self::source_data`] divided by
    /// [`Self::element_word_size`].
    indices: RawBufferVec<u32>,

    /// The size of each element in 32-bit words.
    element_word_size: u32,
}

impl SparseBufferStagingBuffers {
    /// Creates a new set of staging buffers for a sparse buffer vector.
    fn new(label: &str, element_word_size: u32) -> SparseBufferStagingBuffers {
        let mut source_data_buffer =
            RawBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
        source_data_buffer.set_label(Some(&*format!("{} staging buffer", label)));

        let mut indices_buffer = RawBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
        indices_buffer.set_label(Some(&*format!("{} index buffer", label)));

        SparseBufferStagingBuffers {
            source_data: source_data_buffer,
            indices: indices_buffer,
            element_word_size,
        }
    }

    /// Returns the number of updated elements.
    fn updated_element_count(&self) -> u32 {
        (self.source_data.len() / self.element_word_size as usize) as u32
    }

    /// Writes the buffers that contain all the data necessary to perform a
    /// sparse upload to the GPU.
    ///
    /// This includes the buffer associated with the supplied
    /// `metadata_uniform`.
    fn write_buffers(
        &mut self,
        metadata_uniform: &mut UniformBuffer<GpuSparseBufferUpdateMetadata>,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        metadata_uniform.get_mut().updated_element_count = self.updated_element_count();
        metadata_uniform.write_buffer(render_device, render_queue);

        self.source_data.write_buffer(render_device, render_queue);
        self.indices.write_buffer(render_device, render_queue);
    }

    /// Returns true if a sparse buffer update should *not* be performed because
    /// too many words changed.
    fn should_perform_full_reupload(
        &self,
        changed_element_count: u32,
        buffer_length: usize,
    ) -> bool {
        // Calculate the number of changed words. If it's greater than the
        // maximum number of workgroups as defined by `wgpu`, we must perform a
        // full reupload.
        //
        // FIXME: This degrades performance in the exact scenarios we need it
        // the most. We should fall back to doing multiple rounds of uploads in
        // this case.
        let total_changed_word_count = changed_element_count * self.element_word_size;
        if total_changed_word_count > MAX_WORKGROUPS * SPARSE_BUFFER_UPDATE_WORKGROUP_SIZE {
            return true;
        }

        // Don't perform a sparse upload if too many words changed, as it'll end
        // up being slower than just uploading the whole buffer afresh.
        let sparse_upload_fraction = changed_element_count as f64 / buffer_length as f64;
        let should_reupload = sparse_upload_fraction > SPARSE_UPLOAD_THRESHOLD;

        debug!(
            "Sparse buffer changed {}/{} elements ({:.3}, threshold {:.3}): performing {} upload",
            changed_element_count,
            buffer_length,
            sparse_upload_fraction,
            SPARSE_UPLOAD_THRESHOLD,
            if should_reupload { "full" } else { "sparse" }
        );

        should_reupload
    }
}

/// A GPU buffer that can grow, can be updated atomically from multiple threads
/// on the CPU, and is sparsely updated on the GPU if only a small number of
/// elements have changed.
///
/// This type is similar to
/// [`crate::render_resource::buffer_vec::AtomicRawBufferVec`], but instead of
/// reuploading the entire buffer to the GPU when it's changed, it tracks
/// changes on a per-element level and uploads only the elements that changed if
/// the number of such elements is small. It uses a compute shader to scatter
/// those changed elements.
///
/// As the stored data is [`AtomicPod`], multiple threads may update the buffer
/// simultaneously. Note that, like
/// [`crate::render_resource::buffer_vec::AtomicRawBufferVec`], only existing
/// elements may be updated from multiple threads; new data still requires
/// exclusive access.
///
/// `T` must have a size that's a multiple of 4.
pub struct AtomicSparseBufferVec<T>
where
    T: AtomicPod,
{
    /// An ID that uniquely identifies this [`AtomicSparseBufferVec`].
    handle: SparseBufferHandle,
    /// The underlying values.
    ///
    /// These are stored as their blob representation to allow for thread-safe
    /// update.
    values: Vec<T::Blob>,
    /// The GPU buffer, if allocated.
    data_buffer: Option<Buffer>,
    /// The GPU buffers that data is copied into in preparation to be scattered
    /// to the [`Self::data_buffer`].
    staging_buffers: SparseBufferStagingBuffers,
    /// A GPU buffer that stores information such as the element size and stride
    /// that's needed to perform sparse updates.
    metadata_uniform: UniformBuffer<GpuSparseBufferUpdateMetadata>,
    /// The capacity of the GPU buffer in elements.
    capacity: usize,
    /// The allowed `wgpu` buffer usages for the GPU buffer.
    buffer_usages: BufferUsages,
    /// An optional debug label to identify this buffer.
    label: Arc<str>,
    /// A bit set of dirty blocks.
    ///
    /// The size of this vector in bits is the number of elements divided
    /// (rounded up) by 64: in other words, the size of this vector in *bits* is
    /// the size of the [`Self::dirty_bits`] vector in *words*. A 1 in a bit
    /// indicates that the block has changed since the last upload, while a 0
    /// indicates that the block hasn't changed.
    summary: Vec<AtomicU64>,
    /// A bit set of dirty elements.
    ///
    /// The size of this vector in bits is the number of elements, rounded up to
    /// the nearest 64. A 1 in a bit indicates that the element has changed since
    /// the last upload, while a 0 indicates that the element hasn't changed.
    ///
    /// Each group of 64 elements, corresponding to a single word in this array,
    /// is known as a *block*.
    dirty_bits: Vec<AtomicU64>,
    /// True if the entire buffer needs to be reuploaded because it resized.
    needs_full_reupload: bool,
    /// True if a sparse update is to be performed.
    sparse_update_scheduled: bool,
}

impl<T> AtomicSparseBufferVec<T>
where
    T: AtomicPod,
{
    /// Creates a new [`AtomicSparseBufferVec`] with the given set of buffer
    /// usages and label.
    ///
    /// `buffer_usages` specifies the set of allowed `wgpu` buffer usages for
    /// the buffer that [`AtomicSparseBufferVec`] manages.
    /// `BufferUsages::COPY_DST` is automatically added to this set.
    pub fn new(buffer_usages: BufferUsages, label: Arc<str>) -> Self {
        // Make sure the value is word-aligned.
        debug_assert_eq!(size_of::<T>() % 4, 0);
        let element_word_size = size_of::<T>() / 4;

        // Create a unique ID.
        let id = Arc::new(SparseBufferId(
            NEXT_SPARSE_BUFFER_ID.fetch_add(1, Ordering::Relaxed),
        ));

        Self {
            handle: id,
            values: vec![],
            data_buffer: None,
            staging_buffers: SparseBufferStagingBuffers::new(&label, element_word_size as u32),
            metadata_uniform: UniformBuffer::from(GpuSparseBufferUpdateMetadata::new::<T>()),
            capacity: 0,
            buffer_usages: buffer_usages | BufferUsages::COPY_DST,
            label,
            summary: vec![],
            dirty_bits: vec![],
            needs_full_reupload: false,
            sparse_update_scheduled: false,
        }
    }

    /// Returns the number of elements in the CPU side copy of the buffer.
    pub fn len(&self) -> u32 {
        self.values.len() as u32
    }

    /// Returns true if there are no elements in the CPU side copy of the buffer.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns a handle to the buffer, if the data has been uploaded.
    pub fn buffer(&self) -> Option<&Buffer> {
        self.data_buffer.as_ref()
    }

    /// Removes all elements from the buffer.
    pub fn clear(&mut self) {
        self.values.clear();
        self.summary.clear();
        self.dirty_bits.clear();
    }

    /// Copies a value out of the buffer.
    pub fn get(&self, index: u32) -> T {
        T::read_from_blob(&self.values[index as usize])
    }

    /// Sets the value at the given index.
    ///
    /// If the index isn't in range of the buffer, this method panics.
    ///
    /// Internally, the value is converted to its blob representation.
    ///
    /// Note that this method is thread-safe and doesn't require `&mut self`.
    /// It's your responsibility, however, to ensure synchronization; though
    /// this method is memory-safe, it's possible for other threads to observe
    /// partially-overwritten values if [`Self::get`] or similar methods are
    /// called while the write operation is occurring.
    pub fn set(&self, index: u32, value: T) {
        value.write_to_blob(&self.values[index as usize]);
        self.note_changed_index(index);
    }

    /// Adds a new value and returns its index.
    pub fn push(&mut self, value: T) -> u32 {
        let index = self.values.len() as u32;
        self.values.push(T::Blob::default());
        value.write_to_blob(&self.values[index as usize]);

        let dirty_word_index = (index / BITS_PER_WORD) as usize;
        let summary_word_index = dirty_word_index / BITS_PER_WORD as usize;
        while self.summary.len() < summary_word_index + 1 {
            self.summary.push(AtomicU64::default());
        }
        while self.dirty_bits.len() < dirty_word_index + 1 {
            self.dirty_bits.push(AtomicU64::default());
        }

        self.note_changed_index(index);
        index
    }

    /// Marks the given element index as dirty so that we know that we need to
    /// upload it.
    fn note_changed_index(&self, index: u32) {
        note_changed_index(index, &self.summary, &self.dirty_bits);
    }

    /// Ensures that the backing buffer for this buffer vector is present and
    /// appropriately sized on the GPU.
    pub fn reserve(&mut self, new_capacity: usize, render_device: &RenderDevice) {
        reserve(
            new_capacity,
            &mut self.capacity,
            &self.label,
            &mut self.data_buffer,
            self.buffer_usages,
            &mut self.needs_full_reupload,
            size_of::<T::Blob>(),
            render_device,
        );
    }

    /// Grows the buffer by adding default values so that it's at least the
    /// given size.
    ///
    /// This method sets all the newly-added values to dirty.
    ///
    /// If the buffer is already large enough, this method does nothing.
    pub fn grow(&mut self, new_len: u32) {
        let old_len = self.values.len() as u32;
        if old_len >= new_len {
            return;
        }

        self.values.reserve(new_len as usize - old_len as usize);
        self.values.resize_with(new_len as usize, T::Blob::default);

        set_dirty_bits_for_vector_growth(old_len, new_len, &mut self.summary, &mut self.dirty_bits);
    }

    /// Writes the data to the GPU, either via a sparse upload or a bulk data
    /// upload.
    pub fn write_buffers(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        if self.values.is_empty() {
            return;
        }

        // Round up the size to a good value to balance reallocation frequency
        // against memory waste.
        let good_size = calculate_allocation_size(self.values.len());
        self.reserve(good_size, render_device);

        if self.should_perform_full_reupload(render_device) {
            self.write_entire_buffer(render_queue);
        } else {
            self.prepare_sparse_upload(render_device, render_queue);
        }
    }

    /// Returns true if the sparse buffer should perform a full reupload, either
    /// because it was resized or because too much data changed for a sparse
    /// update to be worthwhile.
    fn should_perform_full_reupload(&self, render_device: &RenderDevice) -> bool {
        if self.needs_full_reupload
            || render_device.limits().max_storage_buffers_per_shader_stage < 3
        {
            return true;
        }

        let changed_element_count = count_dirty_elements(&self.summary, &self.dirty_bits);
        self.staging_buffers
            .should_perform_full_reupload(changed_element_count, self.values.len())
    }

    /// Writes the entire buffer in bulk.
    ///
    /// This is the method used when a sparse update is not used, either because
    /// the buffer resized or because too much data changed for a sparse update
    /// to be worthwhile.
    fn write_entire_buffer(&mut self, render_queue: &RenderQueue) {
        let Some(ref mut data_buffer) = self.data_buffer else {
            error!("Dirty sparse buffer should have created a data buffer by now");
            return;
        };

        // SAFETY: We're just writing atomic data to the GPU. The worst that
        // can happen is that we race with somebody, which is unfortunate
        // but not memory-unsafe.
        unsafe {
            render_queue.write_buffer(
                data_buffer,
                0,
                slice::from_raw_parts(
                    self.values.as_ptr().cast::<u8>(),
                    self.values.len() * size_of::<T::Blob>(),
                ),
            );
        }

        // Mark all pages as clean.
        for atomic_summary_word in self.summary.iter() {
            atomic_summary_word.store(0, Ordering::Relaxed);
        }
        for atomic_dirty_word in self.dirty_bits.iter() {
            atomic_dirty_word.store(0, Ordering::Relaxed);
        }
        self.sparse_update_scheduled = false;
    }

    /// Schedules a sparse upload of only the elements that changed.
    fn prepare_sparse_upload(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // Iterate over all dirty elements, using the summary to accelerate the
        // search.
        for (summary_word_index, atomic_summary_word) in self.summary.iter().enumerate() {
            let summary_word = atomic_summary_word.load(Ordering::Relaxed);
            for summary_bit_offset in BitIter::new(summary_word) {
                let dirty_word_index =
                    summary_word_index * BITS_PER_WORD as usize + summary_bit_offset as usize;

                // Iterate over all dirty elements in each dirty page.
                let atomic_dirty_word = &self.dirty_bits[dirty_word_index];
                let dirty_word = atomic_dirty_word.load(Ordering::Relaxed);
                for dirty_bit_offset in BitIter::new(dirty_word) {
                    let element_index =
                        dirty_word_index * BITS_PER_WORD as usize + dirty_bit_offset as usize;

                    let Some(blob) = self.values.get(element_index) else {
                        continue;
                    };

                    // Write the index of the element so the shader will know where to
                    // scatter the data to.
                    self.staging_buffers.indices.push(element_index as u32);

                    // Copy the element to the GPU staging buffer.
                    let value = T::read_from_blob(blob);
                    self.staging_buffers
                        .source_data
                        .extend(bytemuck::cast_slice(&[value]).iter().copied());

                    // Make sure we're aligned up to a full element.
                    debug_assert_eq!(
                        self.staging_buffers.source_data.len()
                            % self.staging_buffers.element_word_size as usize,
                        0
                    );
                }

                // Mark the element as clean.
                atomic_dirty_word.store(0, Ordering::Relaxed);
            }

            // Mark the block as clean.
            atomic_summary_word.store(0, Ordering::Relaxed);
        }

        // Schedule a sparse update if there was something to do.
        self.sparse_update_scheduled = !self.staging_buffers.source_data.is_empty();
        if self.sparse_update_scheduled {
            self.staging_buffers.write_buffers(
                &mut self.metadata_uniform,
                render_device,
                render_queue,
            );
        }
    }

    /// If a sparse update has been scheduled, prepares all GPU resources
    /// necessary to perform a sparse buffer update, other than updating the
    /// metadata uniform.
    pub fn prepare_to_populate_buffers(
        &mut self,
        render_device: &RenderDevice,
        pipeline_cache: &PipelineCache,
        sparse_buffer_update_jobs: &mut SparseBufferUpdateJobs,
        sparse_buffer_update_bind_groups: &mut SparseBufferUpdateBindGroups,
        sparse_buffer_update_pipelines: &SparseBufferUpdatePipelines,
    ) {
        if self.sparse_update_scheduled {
            match (&self.data_buffer, self.metadata_uniform.buffer()) {
                (Some(data_buffer), Some(metadata_buffer)) => {
                    prepare_to_populate_buffers(
                        self.handle.clone(),
                        &self.label,
                        data_buffer,
                        &mut self.staging_buffers,
                        metadata_buffer,
                        render_device,
                        pipeline_cache,
                        sparse_buffer_update_jobs,
                        sparse_buffer_update_bind_groups,
                        sparse_buffer_update_pipelines,
                    );
                }
                _ => {
                    error!("Buffers should have been created by now");
                }
            }
        }

        // Clear out the staging buffers, now that we know the data is already
        // on the GPU.
        self.staging_buffers.source_data.clear();
        self.staging_buffers.indices.clear();

        // Reset the `needs_full_reupload` and `needs_sparse_update` flags.
        self.needs_full_reupload = false;
        self.sparse_update_scheduled = false;
    }
}

impl FromWorld for SparseBufferUpdateBindGroups {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope::<SpecializedComputePipelines<SparseBufferUpdatePipelines>, _>(
            |world, mut specialized_sparse_buffer_update_pipelines| {
                let pipeline_cache = world.resource::<PipelineCache>();
                let sparse_buffer_update_pipelines =
                    world.resource::<SparseBufferUpdatePipelines>();
                let pipeline_id = specialized_sparse_buffer_update_pipelines.specialize(
                    pipeline_cache,
                    sparse_buffer_update_pipelines,
                    (),
                );

                SparseBufferUpdateBindGroups {
                    bind_groups: WeakKeyHashMap::default(),
                    pipeline_id,
                }
            },
        )
    }
}

/// Marks elements within the range `old_len..new_len` as dirty, under the
/// assumption that the vector is being resized from a length of `old_len` to a
/// length of `new_len`.
///
/// This is more efficient than individually marking elements one-by-one. It
/// also resizes the `summary` and `dirty_bits` bitfields as necessary.
///
/// `new_len` must be greater than or equal to `old_len`.
fn set_dirty_bits_for_vector_growth(
    old_len: u32,
    new_len: u32,
    summary: &mut Vec<AtomicU64>,
    dirty_bits: &mut Vec<AtomicU64>,
) {
    debug_assert!(new_len >= old_len);
    if new_len == old_len {
        return;
    }

    if old_len > 0 {
        // Compute the index of the bit corresponding to the final existing
        // element. We're going to set every bit *after* that bit.
        let old_final_dirty_word_index = (old_len - 1) / BITS_PER_WORD;
        let old_final_dirty_bit_offset = (old_len - 1) % BITS_PER_WORD;
        if old_final_dirty_bit_offset < BITS_PER_WORD - 1
            && let Some(ref mut old_final_atomic_dirty_word) =
                dirty_bits.get_mut(old_final_dirty_word_index as usize)
        {
            // We add one here because we want to set every bit *after*, but not
            // including, the index we computed above.
            *old_final_atomic_dirty_word.get_mut() |=
                !((1u64 << (old_final_dirty_bit_offset + 1)).wrapping_sub(1));
        }

        // Now set all the blocks from the block corresponding to `old_len - 1`
        // onward to dirty. Note that this is an inclusive range, because we
        // want to include the page that `old_len - 1` is on.
        let old_final_summary_word_index = old_final_dirty_word_index / BITS_PER_WORD;
        let mut old_final_summary_bit_offset = old_final_dirty_word_index % BITS_PER_WORD;
        // This is a tricky exception. If `old_len` was precisely aligned on a
        // block boundary, then we *don't* include the block that `old_len - 1`
        // is on.
        if old_final_dirty_bit_offset == BITS_PER_WORD - 1 {
            old_final_summary_bit_offset += 1;
        }
        if old_final_summary_bit_offset < BITS_PER_WORD
            && let Some(ref mut old_final_atomic_summary_word) =
                summary.get_mut(old_final_summary_word_index as usize)
        {
            // We don't add one to `old_final_summary_bit_offset` here because
            // we want to include the block that `old_len - 1` is on.
            *old_final_atomic_summary_word.get_mut() |=
                !((1u64 << old_final_summary_bit_offset).wrapping_sub(1));
        }
    }

    // Add any new summary and dirty words, with all bits set.
    let new_dirty_word_count = (new_len as usize).div_ceil(BITS_PER_WORD as usize);
    let new_summary_word_count = new_dirty_word_count.div_ceil(BITS_PER_WORD as usize);
    summary.resize_with(new_summary_word_count, || AtomicU64::new(u64::MAX));
    dirty_bits.resize_with(new_dirty_word_count, || AtomicU64::new(u64::MAX));

    // Clear all bits past the last valid element index in `dirty_bits`.
    let last_dirty_bit_offset = new_len % BITS_PER_WORD;
    if last_dirty_bit_offset != 0 {
        let mut final_dirty_word = dirty_bits[new_dirty_word_count - 1].load(Ordering::Relaxed);
        final_dirty_word &= (1u64 << last_dirty_bit_offset) - 1;
        dirty_bits[new_dirty_word_count - 1].store(final_dirty_word, Ordering::Relaxed);
    }

    // Clear all bits past the last valid summary bit in `summary`.
    let last_summary_bit_offset = new_dirty_word_count % BITS_PER_WORD as usize;
    if last_summary_bit_offset != 0 {
        let mut final_summary_word = summary[new_summary_word_count - 1].load(Ordering::Relaxed);
        final_summary_word &= (1u64 << last_summary_bit_offset) - 1;
        summary[new_summary_word_count - 1].store(final_summary_word, Ordering::Relaxed);
    }
}

/// Marks the given element index as dirty so that we know that we need to
/// upload it.
///
/// This is a separate function so we can unit test it easily (i.e. without the
/// need of a `RenderDevice`).
fn note_changed_index(index: u32, summary: &[AtomicU64], dirty_bits: &[AtomicU64]) {
    let dirty_word_index = index / BITS_PER_WORD;
    let (summary_word_index, summary_bit_offset) = (
        dirty_word_index / BITS_PER_WORD,
        dirty_word_index % BITS_PER_WORD,
    );
    summary[summary_word_index as usize].fetch_or(1 << summary_bit_offset, Ordering::Relaxed);
    let (element_word, element_in_word) = (index / BITS_PER_WORD, index % BITS_PER_WORD);
    dirty_bits[element_word as usize].fetch_or(1 << element_in_word, Ordering::Relaxed);
}

/// Returns the total number of bits set in `dirty_bits`, using the given
/// `summary` to accelerate the count.
fn count_dirty_elements(summary: &[AtomicU64], dirty_bits: &[AtomicU64]) -> u32 {
    let mut changed_element_count = 0u32;
    for (summary_word_index, summary_word) in summary.iter().enumerate() {
        for summary_bit_offset in BitIter::new(summary_word.load(Ordering::Relaxed)) {
            let dirty_word_index =
                summary_word_index * BITS_PER_WORD as usize + summary_bit_offset as usize;
            let dirty_word = dirty_bits[dirty_word_index].load(Ordering::Relaxed);
            changed_element_count += dirty_word.count_ones();
        }
    }

    changed_element_count
}

/// Prepares all GPU resources necessary to perform a sparse buffer update,
/// other than updating the metadata uniform.
///
/// This function creates the [`SparseBufferUpdateJob`] and ensures the bind
/// group and pipeline are up to date.
fn prepare_to_populate_buffers(
    sparse_buffer_handle: SparseBufferHandle,
    label: &Arc<str>,
    data_buffer: &Buffer,
    staging_buffers: &mut SparseBufferStagingBuffers,
    metadata_buffer: &Buffer,
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    sparse_buffer_update_jobs: &mut SparseBufferUpdateJobs,
    sparse_buffer_update_bind_groups: &mut SparseBufferUpdateBindGroups,
    sparse_buffer_update_pipelines: &SparseBufferUpdatePipelines,
) {
    let (Some(source_data_staging_buffer), Some(indices_staging_buffer)) = (
        staging_buffers.source_data.buffer(),
        staging_buffers.indices.buffer(),
    ) else {
        error!("Staging buffers should have been created by now");
        return;
    };

    let Some(bind_group_layout) = &sparse_buffer_update_pipelines.bind_group_layout else {
        return;
    };

    // Record the update job.
    sparse_buffer_update_jobs.push(SparseBufferUpdateJob {
        sparse_buffer_handle: sparse_buffer_handle.clone(),
        updated_element_count: staging_buffers.updated_element_count(),
        element_word_size: staging_buffers.element_word_size,
    });

    // Create the bind group.
    let bind_group = render_device.create_bind_group(
        Some(&*format!("{} bind group", label)),
        &pipeline_cache.get_bind_group_layout(bind_group_layout),
        &BindGroupEntries::sequential((
            // @group(0) @binding(0) var<storage, read_write> dest_buffer: array<u32>;
            data_buffer.as_entire_binding(),
            // @group(0) @binding(1) var<storage> src_buffer: array<u32>;
            source_data_staging_buffer.as_entire_binding(),
            // @group(0) @binding(2) var<storage> indices: array<u32>;
            indices_staging_buffer.as_entire_binding(),
            // @group(0) @binding(3) var<uniform> metadata:
            // SparseBufferUpdateMetadata;
            metadata_buffer.as_entire_binding(),
        )),
    );
    sparse_buffer_update_bind_groups.bind_groups.insert(
        sparse_buffer_handle,
        SparseBufferUpdateBindGroup { bind_group },
    );
}

/// Ensures that the backing buffer for an [`AtomicSparseBufferVec`] is present
/// on the GPU.
///
/// The `capacity`, `data_buffer`, and `needs_full_reupload` fields are updated
/// to reflect the new buffer.
fn reserve(
    new_capacity: usize,
    capacity: &mut usize,
    label: &str,
    data_buffer: &mut Option<Buffer>,
    buffer_usages: BufferUsages,
    needs_full_reupload: &mut bool,
    element_size: usize,
    render_device: &RenderDevice,
) {
    // If the buffer is already big enough, do nothing.
    if new_capacity == 0 || new_capacity <= *capacity {
        return;
    }

    *capacity = new_capacity;
    *data_buffer = Some(render_device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: element_size as u64 * new_capacity as u64,
        usage: buffer_usages,
        mapped_at_creation: false,
    }));

    // Since we resized the buffer, we need to reupload it.
    *needs_full_reupload = true;
}

impl GpuSparseBufferUpdateMetadata {
    /// Returns a new [`GpuSparseBufferUpdateMetadata`] for the given type.
    fn new<T>() -> GpuSparseBufferUpdateMetadata {
        assert_eq!(size_of::<T>() % 4, 0);
        GpuSparseBufferUpdateMetadata {
            element_size: (size_of::<T>() / 4) as u32,
            updated_element_count: 0,
        }
    }
}

/// Iterates over the bits in a single `u64`, from the least significant bit to
/// the most significant bit.
struct BitIter(u64);

impl BitIter {
    fn new(bits: u64) -> BitIter {
        BitIter(bits)
    }
}

impl Iterator for BitIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let trailing_zeros = self.0.trailing_zeros();
        if trailing_zeros == 64 {
            return None;
        }
        self.0 &= !(1 << trailing_zeros);
        Some(trailing_zeros)
    }
}

/// Calculates the size that a buffer should be in order to balance reallocation
/// frequency against memory waste.
fn calculate_allocation_size(length: usize) -> usize {
    let exponent = (length as f64).log(REALLOCATION_FACTOR).ceil();
    let size = REALLOCATION_FACTOR.powf(exponent) as usize;
    size.next_multiple_of(REALLOCATION_SIZE_MULTIPLE)
}

#[cfg(test)]
mod tests {
    use super::{BitIter, BITS_PER_WORD};
    use core::{
        iter,
        sync::atomic::{AtomicU64, Ordering},
    };
    use proptest::prelude::proptest;

    proptest! {
        // Ensures that the `BitIter` is correct.
        #[test]
        fn bit_iter(bits: u64) {
            let bits_reference: Vec<_> = (0u32..64u32).filter(|bit_pos| {
                (bits & (1 << bit_pos)) != 0
            }).collect();
            let bits_iter_results: Vec<_> = BitIter::new(bits).collect();
            assert_eq!(bits_iter_results, bits_reference);
        }

        // Ensures that `set_dirty_bits_for_vector_growth` is correct.
        #[test]
        fn set_dirty_bits_for_vector_growth(
            old_len in 0u32..16384u32,
            new_element_count in 0u32..16384u32,
            start_dirty: bool
        ) {
            // Initialize the dirty bits.
            let new_len = old_len + new_element_count;
            let mut dirty_bits: Vec<_> = iter::repeat_with(|| {
                AtomicU64::new(0)
            }).take(old_len.div_ceil(BITS_PER_WORD) as usize).collect();
            if start_dirty {
                for bit_index in 0..old_len {
                    let word_index = bit_index as usize / 64;
                    dirty_bits[word_index].fetch_or(1 << (bit_index % 64), Ordering::Relaxed);
                }
            }

            // Initialize the summary.
            let mut summary: Vec<_> = iter::repeat_with(|| {
                AtomicU64::new(0)
            }).take(dirty_bits.len().div_ceil(BITS_PER_WORD as usize)).collect();
            for (word_index, word) in dirty_bits.iter().enumerate() {
                if word.load(Ordering::Relaxed) != 0 {
                    summary[word_index / 64].fetch_or(1 << (word_index % 64), Ordering::Relaxed);
                }
            }

            super::set_dirty_bits_for_vector_growth(
                old_len,
                new_len,
                &mut summary,
                &mut dirty_bits
            );

            // Check dirty flags for elements.
            // Bits in the range [0, old_len) should be unchanged.
            for element_index in 0..old_len {
                check_element_dirty(element_index, &dirty_bits, start_dirty);
            }
            // Bits in the range [old_len, new_len) should be dirty.
            for element_index in old_len..new_len {
                check_element_dirty(element_index, &dirty_bits, true);
            }
            // Bits in the range [new_len, end) should be clean.
            for element_index in (new_len..).take_while(|element_index| {
                element_index % BITS_PER_WORD != 0
            }) {
                check_element_dirty(element_index, &dirty_bits, false);
            }

            // Check the dirty flag for each block to ensure that it precisely
            // corresponds to the logical *or* of the dirty flags for all
            // elements in that block.
            for (dirty_word_index, atomic_dirty_word) in dirty_bits.iter().enumerate() {
                // Determine the range of elements that this block encompasses.
                let element_start = dirty_word_index * BITS_PER_WORD as usize;
                let element_end =
                    ((dirty_word_index + 1) * BITS_PER_WORD as usize).min(new_len as usize);
                assert!(element_start <= element_end);

                // Determine whether the block should be dirty.
                let dirty_word = atomic_dirty_word.load(Ordering::Relaxed);
                let block_is_dirty = (element_start..element_end).any(|element_index| {
                    (dirty_word & (1 << (element_index % (BITS_PER_WORD as usize)))) != 0
                });

                // Check to make sure that the block has the correct dirty state.
                check_block_dirty(dirty_word_index as u32, &summary, block_is_dirty);
            }

            // Make sure that all dirty block bits past the last valid dirty
            // block bit are clear.
            if !summary.is_empty() {
                let last_summary_word_index = summary.len();
                let last_padding_block_index = last_summary_word_index * BITS_PER_WORD as usize;
                let last_dirty_word_index = (new_len as usize - 1) / BITS_PER_WORD as usize;
                for padding_block_index in (last_dirty_word_index + 1)..last_padding_block_index {
                    check_block_dirty(padding_block_index as u32, &summary, false);
                }
            }

            // Asserts that the dirty status of the element at `element_index`
            // matches the expected dirty status.
            fn check_element_dirty(
                element_index: u32,
                dirty_bits: &[AtomicU64],
                expect_dirty: bool
            ) {
                let expected = if expect_dirty { 1 } else { 0 };

                let dirty_word_index = element_index / BITS_PER_WORD;
                let dirty_bit_offset = element_index % BITS_PER_WORD;
                let dirty_word = dirty_bits[dirty_word_index as usize].load(Ordering::Relaxed);
                assert_eq!((dirty_word >> dirty_bit_offset) & 1, expected);
            }

            // Asserts that the dirty status of the block at `block_index`
            // matches the expected dirty status in the summary.
            //
            // This is actually the same code as `ensure_elements_dirty`, but is
            // duplicated for clarity.
            fn check_block_dirty(block_index: u32, summary: &[AtomicU64], expect_dirty: bool) {
                let expected = if expect_dirty { 1 } else { 0 };

                let summary_word_index = block_index / BITS_PER_WORD;
                let summary_bit_offset = block_index % BITS_PER_WORD;
                let summary_word = summary[summary_word_index as usize].load(Ordering::Relaxed);
                assert_eq!((summary_word >> summary_bit_offset) & 1, expected);
            }
        }

        // Ensures that the population-count-based `count_dirty_elements` code
        // correctly calculates the number of changed elements.
        //
        // The input `dirty_flags` is an array of booleans, one for each
        // element, in which `false` represents "not changed" and `true`
        // represents "changed".
        #[test]
        fn dirty_element_count(dirty_flags: Vec<bool>) {
            let dirty_word_count = dirty_flags.len().div_ceil(BITS_PER_WORD as usize);
            let summary_word_count = dirty_word_count.div_ceil(BITS_PER_WORD as usize);

            let dirty_bits: Vec<_> = (0..dirty_word_count).map(|_| AtomicU64::new(0)).collect();
            let summary: Vec<_> = (0..summary_word_count).map(|_| AtomicU64::new(0)).collect();

            let mut true_dirty_element_count = 0;
            for (element_index, _) in dirty_flags.iter().enumerate().filter(|(_, element)| **element) {
                super::note_changed_index(element_index as u32, &summary, &dirty_bits);
                true_dirty_element_count += 1;
            }

            let calculated_dirty_element_count = super::count_dirty_elements(
                &summary,
                &dirty_bits
            );
            assert_eq!(calculated_dirty_element_count, true_dirty_element_count);
        }
    }
}
