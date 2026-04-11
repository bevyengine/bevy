//! GPU buffers that support sparse updates if only a small number of elements
//! have changed.

use alloc::sync::{Arc, Weak};
use core::{
    iter, slice,
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
use bevy_log::{error, info};
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
    diagnostic::{DiagnosticsRecorder, RecordDiagnostics as _},
    render_resource::{
        AtomicPod, BindGroup, BindGroupEntries, Buffer, PipelineCache, RawBufferVec,
        SpecializedComputePipeline, SpecializedComputePipelines, UniformBuffer,
    },
    renderer::{RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue},
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

/// The number of dirty-page bits packed into each [`AtomicU64`] word.
const PAGES_PER_DIRTY_WORD: u32 = 64;

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
    /// The number of pages to update.
    updated_page_count: u32,
    /// The base-2 logarithm of the size of a page for the buffer.
    ///
    /// The actual page size can be computed as `1 << page_size_log2`.
    page_size_log2: u32,
    /// The size of each element in 32-bit words.
    element_word_size: u32,
    /// A debugging label for the buffer.
    label: Arc<str>,
}

impl SparseBufferUpdateJob {
    /// The number of elements per page.
    fn page_size(&self) -> u32 {
        1 << self.page_size_log2
    }

    /// Calculates the number of words that need to be updated.
    fn words_to_update(&self) -> u32 {
        self.updated_page_count * self.page_size() * self.element_word_size
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
    /// The number of pages that need to be updated.
    updated_page_count: u32,
    /// The base-2 logarithm of the page size.
    ///
    /// That is, the page size is `1 << page_size_log2`.
    page_size_log2: u32,
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
    mut diagnostics: Option<ResMut<DiagnosticsRecorder>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Bail if we have nothing to do.
    if sparse_buffer_update_jobs.is_empty() {
        return;
    }

    // We need to create a command encoder since this pass isn't associated with
    // a view.
    let mut command_encoder =
        render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sparse buffer update"),
        });

    let time_span = diagnostics
        .as_mut()
        .map(|diagnostics| diagnostics.time_span(&mut command_encoder, "sparse buffer update"));

    command_encoder.push_debug_group("sparse buffer update");

    let Some(compute_pipeline) =
        pipeline_cache.get_compute_pipeline(sparse_buffer_update_bind_groups.pipeline_id)
    else {
        return;
    };

    // Process each sparse buffer update job.
    for sparse_buffer_update_job in sparse_buffer_update_jobs.iter() {
        let Some(sparse_buffer_update_bind_group) = sparse_buffer_update_bind_groups
            .bind_groups
            .get(&sparse_buffer_update_job.sparse_buffer_handle)
        else {
            continue;
        };

        let mut sparse_buffer_update_pass =
            command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&*format!(
                    "sparse buffer update ({})",
                    &sparse_buffer_update_job.label
                )),
                timestamp_writes: None,
            });
        sparse_buffer_update_pass.set_pipeline(compute_pipeline);
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

    command_encoder.pop_debug_group();
    if let Some(time_span) = time_span {
        time_span.end(&mut command_encoder);
    }

    render_queue.submit([command_encoder.finish()]);
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
    /// All pages that have changed and need to be updated.
    source_data: RawBufferVec<u32>,

    /// The index at which we write each page in [`Self::source_data`].
    ///
    /// The length of this buffer is equal to [`Self::source_data`] divided by
    /// 2^[`Self::page_size_log2`].
    indices: RawBufferVec<u32>,

    /// The size of each element in 32-bit words.
    element_word_size: u32,

    /// The base-2 logarithm of the page size in elements.
    ///
    /// That is, the page size in elements is `1 << page_size_log2`.
    page_size_log2: u32,
}

impl SparseBufferStagingBuffers {
    /// The number of elements per page.
    fn page_size(&self) -> usize {
        1 << self.page_size_log2
    }

    /// Creates a new set of staging buffers for a sparse buffer vector.
    fn new(label: &str, element_word_size: u32, page_size_log2: u32) -> SparseBufferStagingBuffers {
        let mut source_data_buffer =
            RawBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
        source_data_buffer.set_label(Some(&*format!("{} staging buffer", label)));

        let mut indices_buffer = RawBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
        indices_buffer.set_label(Some(&*format!("{} index buffer", label)));

        SparseBufferStagingBuffers {
            source_data: source_data_buffer,
            indices: indices_buffer,
            element_word_size,
            page_size_log2,
        }
    }

    /// Returns the number of updated pages.
    fn updated_page_count(&self) -> u32 {
        // Note that we don't have to round up here because data is always
        // uploaded in increments of a whole page.
        let element_count = self.source_data.len() / self.element_word_size as usize;
        (element_count / self.page_size()) as u32
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
        metadata_uniform.get_mut().updated_page_count = self.updated_page_count();
        metadata_uniform.write_buffer(render_device, render_queue);

        self.source_data.write_buffer(render_device, render_queue);
        self.indices.write_buffer(render_device, render_queue);
    }

    /// Returns true if a sparse buffer update should *not* be performed because
    /// too many words changed.
    fn should_perform_full_reupload(&self, changed_page_count: u32, buffer_length: usize) -> bool {
        // Calculate the number of changed words. If it's greater than the
        // maximum number of workgroups as defined by `wgpu`, we must perform a
        // full reupload.
        let total_changed_word_count =
            changed_page_count * self.page_size() as u32 * self.element_word_size;
        if total_changed_word_count > MAX_WORKGROUPS * SPARSE_BUFFER_UPDATE_WORKGROUP_SIZE {
            return true;
        }

        // Don't perform a sparse upload if too many words changed, as it'll end
        // up being slower than just uploading the whole buffer afresh.
        let sparse_upload_fraction =
            changed_page_count as f64 / buffer_length.div_ceil(self.page_size()) as f64;
        sparse_upload_fraction > SPARSE_UPLOAD_THRESHOLD
    }
}

/// A GPU buffer that can grow, can be updated atomically from multiple threads
/// on the CPU, and is sparsely updated on the GPU if only a small number of
/// elements have changed.
///
/// This type is similar to
/// [`crate::render_resource::buffer_vec::AtomicRawBufferVec`], but instead of
/// reuploading the entire buffer to the GPU when it's changed, it tracks
/// changes on a per-page level and uploads only the pages that changed if the
/// number of such pages is small. It uses a compute shader to scatter the
/// changed pages.
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
    /// A bit set of dirty pages.
    ///
    /// The size of this vector in bits is the number of elements divided by the
    /// page size, rounded up. A 1 in a bit indicates that the page has changed
    /// since the last upload, while a 0 indicates that the page hasn't changed.
    dirty_pages: Vec<AtomicU64>,
    /// True if the entire buffer needs to be reuploaded because it resized.
    needs_full_reupload: bool,
    /// True if a sparse update is to be performed.
    sparse_update_scheduled: bool,
}

impl<T> AtomicSparseBufferVec<T>
where
    T: AtomicPod,
{
    /// The number of elements per page.
    fn page_size(&self) -> u32 {
        1 << self.staging_buffers.page_size_log2
    }

    /// Creates a new [`AtomicSparseBufferVec`] with the given set of buffer
    /// usages, page size, and label.
    ///
    /// `buffer_usages` specifies the set of allowed `wgpu` buffer usages for
    /// the buffer that [`AtomicSparseBufferVec`] manages.
    /// `BufferUsages::COPY_DST` is automatically added to this set.
    ///
    /// The `page_size_log2` parameter is the base-2 logarithm of the page size.
    /// That is, the page size is `1 << page_size_log2`.
    pub fn new(buffer_usages: BufferUsages, page_size_log2: u32, label: Arc<str>) -> Self {
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
            staging_buffers: SparseBufferStagingBuffers::new(
                &label,
                element_word_size as u32,
                page_size_log2,
            ),
            metadata_uniform: UniformBuffer::from(GpuSparseBufferUpdateMetadata::new::<T>(
                page_size_log2,
            )),
            capacity: 0,
            buffer_usages: buffer_usages | BufferUsages::COPY_DST,
            label,
            dirty_pages: vec![],
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
        self.truncate(0);
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

        let page_word = (self.index_to_page(index) / PAGES_PER_DIRTY_WORD) as usize;
        while self.dirty_pages.len() < page_word + 1 {
            self.dirty_pages.push(AtomicU64::default());
        }
        self.note_changed_index(index);

        index
    }

    /// Marks the page corresponding to the given element index as dirty so that
    /// we know that we need to upload it.
    fn note_changed_index(&self, index: u32) {
        let page = self.index_to_page(index);
        let (page_word, page_in_word) = (page / PAGES_PER_DIRTY_WORD, page % PAGES_PER_DIRTY_WORD);
        self.dirty_pages[page_word as usize].fetch_or(1 << page_in_word, Ordering::Relaxed);
    }

    /// Returns the page corresponding to the given element index.
    fn index_to_page(&self, index: u32) -> u32 {
        index / self.page_size()
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
    /// If the buffer is already large enough, this method does nothing.
    pub fn grow(&mut self, new_len: u32) {
        let old_len = self.values.len() as u32;
        if old_len >= new_len {
            return;
        }

        self.values.reserve(new_len as usize - old_len as usize);
        self.values.resize_with(new_len as usize, T::Blob::default);

        // This is a bit tricky. We want to set the dirty bits corresponding to
        // all pages that we added, if any. First, we compute the index of the
        // last page word before the append operation.
        let old_final_page = self.index_to_page(old_len);
        let old_final_page_word_index = old_final_page / PAGES_PER_DIRTY_WORD;
        let old_final_page_in_word = old_final_page % PAGES_PER_DIRTY_WORD;

        // Next, we set the bits corresponding to every page that we added to
        // that final page word. Note that this might set bits corresponding to
        // pages past the end of our buffer; that's OK as we ignore them.
        if old_final_page_in_word != 0
            && let Some(ref mut old_final_atomic_page_word) =
                self.dirty_pages.get_mut(old_final_page_word_index as usize)
        {
            *old_final_atomic_page_word.get_mut() |= !((1u64 << old_final_page_in_word) - 1);
        }

        // Finally, we add any new page words, with all bits set.
        let new_page_count = self.index_to_page(new_len);
        self.dirty_pages.resize_with(
            (new_page_count as usize).div_ceil(PAGES_PER_DIRTY_WORD as usize),
            || AtomicU64::new(u64::MAX),
        );
    }

    /// Truncates the buffer to the given length.
    ///
    /// If the buffer is already that length or shorter, this method does
    /// nothing.
    pub fn truncate(&mut self, len: u32) {
        self.values.truncate(len as usize);

        let page = self.index_to_page(len);
        self.dirty_pages
            .truncate(page.div_ceil(PAGES_PER_DIRTY_WORD) as usize);
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
        if self.needs_full_reupload {
            return true;
        }

        if render_device.limits().max_storage_buffers_per_shader_stage < 3 {
            return true;
        }

        // Calculate the number of changed pages via population count.
        let changed_page_count: u32 = self
            .dirty_pages
            .iter()
            .map(|atomic_page_word| atomic_page_word.load(Ordering::Relaxed).count_ones())
            .sum();

        self.staging_buffers
            .should_perform_full_reupload(changed_page_count, self.values.len())
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
        for atomic_page_word in self.dirty_pages.iter() {
            atomic_page_word.store(0, Ordering::Relaxed);
        }
        self.sparse_update_scheduled = false;
    }

    /// Schedules a sparse upload of only the pages that changed.
    fn prepare_sparse_upload(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // Iterate over all dirty pages.
        for (page_word_index, atomic_page_word) in self.dirty_pages.iter().enumerate() {
            let page_word = atomic_page_word.load(Ordering::Relaxed);
            for page_index_in_word in BitIter::new(page_word) {
                let page = page_word_index as u32 * PAGES_PER_DIRTY_WORD + page_index_in_word;

                // Write the index of the page so the shader will know where to
                // scatter the data to.
                self.staging_buffers.indices.push(page);

                // Copy the page to the GPU staging buffer.
                let page_size = self.staging_buffers.page_size();
                let page_start = page as usize * page_size;
                let page_end = page_start + page_size;
                for value_index in page_start..page_end {
                    match self.values.get(value_index) {
                        Some(blob) => {
                            let value = T::read_from_blob(blob);
                            self.staging_buffers
                                .source_data
                                .extend(bytemuck::cast_slice(&[value]).iter().copied());
                        }
                        None => {
                            self.staging_buffers.source_data.extend(iter::repeat_n(
                                0,
                                self.staging_buffers.element_word_size as usize,
                            ));
                        }
                    }
                }

                // Make sure we're aligned up to a full page.
                debug_assert_eq!(
                    self.staging_buffers.source_data.len()
                        % (self.staging_buffers.element_word_size as usize
                            * self.staging_buffers.page_size()),
                    0
                );
            }

            // Mark the page as clean.
            atomic_page_word.store(0, Ordering::Relaxed);
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
        page_size_log2: staging_buffers.page_size_log2,
        updated_page_count: staging_buffers.updated_page_count(),
        element_word_size: staging_buffers.element_word_size,
        label: (*label).clone(),
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
    /// Returns a new [`GpuSparseBufferUpdateMetadata`] for the given type and
    /// page size.
    fn new<T>(page_size_log2: u32) -> GpuSparseBufferUpdateMetadata {
        assert_eq!(size_of::<T>() % 4, 0);
        GpuSparseBufferUpdateMetadata {
            element_size: (size_of::<T>() / 4) as u32,
            updated_page_count: 0,
            page_size_log2,
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
