//! Batching functionality when GPU preprocessing is in use.

use alloc::sync::Arc;
use core::{
    any::TypeId,
    marker::PhantomData,
    mem,
    ops::Range,
    sync::atomic::{AtomicU32, Ordering},
};

use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::Entity,
    query::{Has, With},
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Query, Res, ResMut, StaticSystemParam},
    world::{FromWorld, World},
};
use bevy_encase_derive::ShaderType;
use bevy_log::{error, info_once};
use bevy_math::UVec4;
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_tasks::ComputeTaskPool;
use bevy_utils::{default, TypeIdMap};
use bytemuck::{Pod, Zeroable};
use encase::{internal::WriteInto, ShaderSize};
use nonmax::NonMaxU32;
use wgpu::{BindingResource, BufferUsages, DownlevelFlags, Features};

use crate::{
    occlusion_culling::OcclusionCulling,
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhaseBatch, BinnedRenderPhaseBatchSet,
        BinnedRenderPhaseBatchSets, CachedRenderPipelinePhaseItem, PhaseItem,
        PhaseItemBatchSetKey as _, PhaseItemExtraIndex, RenderMultidrawableBatchSet,
        SortedPhaseItem, SortedRenderPhase, UnbatchableBinnedEntityIndices, ViewBinnedRenderPhases,
        ViewSortedRenderPhases,
    },
    render_resource::{
        AtomicPod, AtomicRawBufferVec, AtomicSparseBufferVec, Buffer, GpuArrayBufferable,
        PartialBufferVec, PipelineCache, RawBufferVec, SparseBufferUpdateBindGroups,
        SparseBufferUpdateJobs, SparseBufferUpdatePipelines, UninitBufferVec,
    },
    renderer::{RenderAdapter, RenderAdapterInfo, RenderDevice, RenderQueue, WgpuWrapper},
    sync_world::{MainEntity, MainEntityHashMap},
    view::{ExtractedView, NoIndirectDrawing, RetainedViewEntity},
    GpuResourceAppExt, Render, RenderApp, RenderDebugFlags, RenderSystems,
};

use super::{BatchSetMeta, GetBatchData, GetFullBatchData};

#[derive(Default)]
pub struct BatchingPlugin {
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl Plugin for BatchingPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(IndirectParametersBuffersSettings {
                allow_copies_from_indirect_parameter_buffers: self
                    .debug_flags
                    .contains(RenderDebugFlags::ALLOW_COPIES_FROM_INDIRECT_PARAMETERS),
            })
            .init_gpu_resource::<IndirectParametersBuffers>()
            .allow_ambiguous_resource::<IndirectParametersBuffers>()
            .init_gpu_resource::<BinUnpackingBuffers>()
            .add_systems(
                Render,
                write_indirect_parameters_buffers.in_set(RenderSystems::PrepareResourcesFlush),
            )
            .add_systems(
                Render,
                clear_indirect_parameters_buffers.in_set(RenderSystems::PrepareViews),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_gpu_resource::<GpuPreprocessingSupport>();
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

    /// Returns true if GPU culling is supported on this platform.
    pub fn is_culling_supported(&self) -> bool {
        self.max_supported_mode == GpuPreprocessingMode::Culling
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
    BDI: AtomicPod,
{
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
    pub previous_input_buffer: PreviousInstanceInputUniformBuffer<BDI>,

    /// The data needed to render buffers for each phase.
    ///
    /// The keys of this map are the type IDs of each phase: e.g. `Opaque3d`,
    /// `AlphaMask3d`, etc.
    pub phase_instance_buffers: TypeIdMap<UntypedPhaseBatchedInstanceBuffers<BD>>,
}

impl<BD, BDI> Default for BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: AtomicPod,
{
    fn default() -> Self {
        BatchedInstanceBuffers {
            current_input_buffer: InstanceInputUniformBuffer::new(),
            previous_input_buffer: PreviousInstanceInputUniformBuffer::new(),
            phase_instance_buffers: TypeIdMap::default(),
        }
    }
}

/// The GPU buffers holding the data needed to render batches for a single
/// phase.
///
/// These are split out per phase so that we can run the phases in parallel.
/// This is the version of the structure that has a type parameter, which
/// enables Bevy's scheduler to run the batching operations for the different
/// phases in parallel.
///
/// See the documentation for [`BatchedInstanceBuffers`] for more information.
#[derive(Resource)]
pub struct PhaseBatchedInstanceBuffers<PI, BD>
where
    PI: PhaseItem,
    BD: GpuArrayBufferable + Sync + Send + 'static,
{
    /// The buffers for this phase.
    pub buffers: UntypedPhaseBatchedInstanceBuffers<BD>,
    phantom: PhantomData<PI>,
}

impl<PI, BD> Default for PhaseBatchedInstanceBuffers<PI, BD>
where
    PI: PhaseItem,
    BD: GpuArrayBufferable + Sync + Send + 'static,
{
    fn default() -> Self {
        PhaseBatchedInstanceBuffers {
            buffers: UntypedPhaseBatchedInstanceBuffers::default(),
            phantom: PhantomData,
        }
    }
}

/// The GPU buffers holding the data needed to render batches for a single
/// phase, without a type parameter for that phase.
///
/// Since this structure doesn't have a type parameter, it can be placed in
/// [`BatchedInstanceBuffers::phase_instance_buffers`].
pub struct UntypedPhaseBatchedInstanceBuffers<BD>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
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
    pub work_item_buffers: HashMap<RetainedViewEntity, PreprocessWorkItemBuffers>,

    /// A buffer that holds the number of indexed meshes that weren't visible in
    /// the previous frame, when GPU occlusion culling is in use.
    ///
    /// There's one set of [`LatePreprocessWorkItemIndirectParameters`] per
    /// view. Bevy uses this value to determine how many threads to dispatch to
    /// check meshes that weren't visible next frame to see if they became newly
    /// visible this frame.
    pub late_indexed_indirect_parameters_buffer:
        RawBufferVec<LatePreprocessWorkItemIndirectParameters>,

    /// A buffer that holds the number of non-indexed meshes that weren't
    /// visible in the previous frame, when GPU occlusion culling is in use.
    ///
    /// There's one set of [`LatePreprocessWorkItemIndirectParameters`] per
    /// view. Bevy uses this value to determine how many threads to dispatch to
    /// check meshes that weren't visible next frame to see if they became newly
    /// visible this frame.
    pub late_non_indexed_indirect_parameters_buffer:
        RawBufferVec<LatePreprocessWorkItemIndirectParameters>,
}

/// Holds the GPU buffer of instance input data, which is the data about each
/// mesh instance that the CPU provides.
///
/// `BDI` is the *buffer data input* type, which the GPU mesh preprocessing
/// shader is expected to expand to the full *buffer data* type.
pub struct InstanceInputUniformBuffer<BDI>
where
    BDI: AtomicPod,
{
    /// The buffer containing the data that will be uploaded to the GPU.
    buffer: AtomicSparseBufferVec<BDI>,

    /// Indices of slots that are free within the buffer.
    ///
    /// When adding data, we preferentially overwrite these slots first before
    /// growing the buffer itself.
    free_uniform_indices: Vec<u32>,
}

impl<BDI> InstanceInputUniformBuffer<BDI>
where
    BDI: AtomicPod,
{
    /// Creates a new, empty buffer.
    pub fn new() -> InstanceInputUniformBuffer<BDI> {
        InstanceInputUniformBuffer {
            buffer: AtomicSparseBufferVec::new(
                BufferUsages::STORAGE,
                8,
                Arc::from("instance input uniform buffer"),
            ),
            free_uniform_indices: vec![],
        }
    }

    /// Clears the buffer and entity list out.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.free_uniform_indices.clear();
    }

    /// Returns the [`AtomicSparseBufferVec`] corresponding to this input
    /// uniform buffer.
    #[inline]
    pub fn buffer(&self) -> &AtomicSparseBufferVec<BDI> {
        &self.buffer
    }

    /// Adds a new piece of buffered data to the uniform buffer and returns its
    /// index.
    pub fn add(&mut self, element: BDI) -> u32 {
        match self.free_uniform_indices.pop() {
            Some(uniform_index) => {
                self.buffer.set(uniform_index, element);
                uniform_index
            }
            None => self.buffer.push(element),
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
        if uniform_index >= self.buffer.len() || self.free_uniform_indices.contains(&uniform_index)
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
        self.buffer.get(uniform_index)
    }

    /// Stores a piece of buffered data at the given index.
    ///
    /// # Panics
    /// if `uniform_index` is not in bounds of [`Self::buffer`].
    pub fn set(&self, uniform_index: u32, element: BDI) {
        self.buffer.set(uniform_index, element);
    }

    // Ensures that the buffers are nonempty, which the GPU requires before an
    // upload can take place.
    pub fn ensure_nonempty(&mut self) {
        if self.buffer.is_empty() {
            self.buffer.push(default());
        }
    }

    /// Returns the number of instances in this buffer.
    pub fn len(&self) -> usize {
        self.buffer.len() as usize
    }

    /// Returns true if this buffer has no instances or false if it contains any
    /// instances.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Consumes this [`InstanceInputUniformBuffer`] and returns the raw buffer
    /// ready to be uploaded to the GPU.
    pub fn into_buffer(self) -> AtomicSparseBufferVec<BDI> {
        self.buffer
    }
}

impl<BDI> Default for InstanceInputUniformBuffer<BDI>
where
    BDI: AtomicPod,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Stores the input uniforms for the previous frame.
///
/// This doesn't use a sparse buffer because it's cleared out every frame and
/// only ever pushed onto. The length is stored in an atomic field, so multiple
/// threads can push simultaneously.
///
/// The [`AtomicRawBufferVec`] serves as a backing store only. We reserve a
/// large size, enough to hold all push operations that could possibly occur on
/// the worker threads, and only synchronize the changed portion of the buffer
/// to the GPU on each frame.
pub struct PreviousInstanceInputUniformBuffer<BDI>
where
    BDI: AtomicPod,
{
    /// The buffer containing the data that will be uploaded to the GPU.
    buffer: AtomicRawBufferVec<BDI>,

    /// The number of elements pushed since the last [`Self::reserve`].
    atomic_len: AtomicU32,
}

impl<BDI> PreviousInstanceInputUniformBuffer<BDI>
where
    BDI: AtomicPod,
{
    /// Creates a new, empty buffer.
    pub fn new() -> PreviousInstanceInputUniformBuffer<BDI> {
        PreviousInstanceInputUniformBuffer {
            buffer: AtomicRawBufferVec::with_label(
                BufferUsages::STORAGE,
                "previous instance input uniform buffer",
            ),
            atomic_len: AtomicU32::new(0),
        }
    }

    /// Writes the buffer to the GPU.
    fn write_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // `Self::ensure_nonempty` must have been called first.
        debug_assert!(!self.buffer.is_empty());
        // Only write the modified portion of this buffer. Typically, that
        // portion will be much smaller than the full size of the buffer.
        self.buffer.write_buffer_range(
            0..(self.atomic_len.load(Ordering::Relaxed) as usize).max(1),
            render_device,
            render_queue,
        );
    }

    /// Clears out the buffer in preparation for a new frame.
    pub fn clear(&mut self) {
        // Don't actually clear the underlying buffer out, as then we'd have to
        // grow it again and that would be slow.
        self.atomic_len.store(0, Ordering::Relaxed);
    }

    /// Pre-allocates capacity for concurrent [`Self::push`] calls.
    pub fn reserve(&mut self, capacity: u32) {
        self.buffer.grow(capacity);
        *self.atomic_len.get_mut() = 0;
    }

    /// Appends a value and returns its index. Thread-safe.
    ///
    /// [`Self::reserve`] must have been called first with sufficient capacity.
    pub fn push(&self, value: BDI) -> u32 {
        let index = self.atomic_len.fetch_add(1, Ordering::Relaxed);
        debug_assert!(
            (index as usize) < self.buffer.len() as usize,
            "push exceeded pre-allocated capacity"
        );
        self.buffer.set(index, value);
        index
    }

    /// Pushes a dummy element onto the backing store of this buffer, if this
    /// buffer is empty.
    pub fn ensure_nonempty(&mut self) {
        if self.buffer.is_empty() {
            self.buffer.push(default());
        }
    }

    /// Returns the GPU buffer, if allocated.
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.buffer()
    }
}

impl<BDI> Default for PreviousInstanceInputUniformBuffer<BDI>
where
    BDI: AtomicPod,
{
    fn default() -> Self {
        Self::new()
    }
}

/// The buffer of GPU preprocessing work items for a single view.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::large_enum_variant,
        reason = "See https://github.com/bevyengine/bevy/issues/19220"
    )
)]
pub enum PreprocessWorkItemBuffers {
    /// The work items we use if we aren't using indirect drawing.
    ///
    /// Because we don't have to separate indexed from non-indexed meshes in
    /// direct mode, we only have a single buffer here.
    Direct(RawBufferVec<PreprocessWorkItem>),

    /// The buffer of work items we use if we are using indirect drawing.
    ///
    /// We need to separate out indexed meshes from non-indexed meshes in this
    /// case because the indirect parameters for these two types of meshes have
    /// different sizes.
    Indirect {
        /// The buffer of work items corresponding to indexed meshes.
        indexed: PartialBufferVec<PreprocessWorkItem>,
        /// The buffer of work items corresponding to non-indexed meshes.
        non_indexed: PartialBufferVec<PreprocessWorkItem>,
        /// The work item buffers we use when GPU occlusion culling is in use.
        gpu_occlusion_culling: Option<GpuOcclusionCullingWorkItemBuffers>,
    },
}

/// The work item buffers we use when GPU occlusion culling is in use.
pub struct GpuOcclusionCullingWorkItemBuffers {
    /// The buffer of work items corresponding to indexed meshes.
    pub late_indexed: UninitBufferVec<PreprocessWorkItem>,
    /// The buffer of work items corresponding to non-indexed meshes.
    pub late_non_indexed: UninitBufferVec<PreprocessWorkItem>,
    /// The offset into the
    /// [`UntypedPhaseBatchedInstanceBuffers::late_indexed_indirect_parameters_buffer`]
    /// where this view's indirect dispatch counts for indexed meshes live.
    pub late_indirect_parameters_indexed_offset: u32,
    /// The offset into the
    /// [`UntypedPhaseBatchedInstanceBuffers::late_non_indexed_indirect_parameters_buffer`]
    /// where this view's indirect dispatch counts for non-indexed meshes live.
    pub late_indirect_parameters_non_indexed_offset: u32,
}

/// A GPU-side data structure that stores the number of workgroups to dispatch
/// for the second phase of GPU occlusion culling.
///
/// The late mesh preprocessing phase checks meshes that weren't visible frame
/// to determine if they're potentially visible this frame.
#[derive(Clone, Copy, ShaderType, Pod, Zeroable)]
#[repr(C)]
pub struct LatePreprocessWorkItemIndirectParameters {
    /// The number of workgroups to dispatch.
    ///
    /// This will be equal to `work_item_count / 64`, rounded *up*.
    dispatch_x: u32,
    /// The number of workgroups along the abstract Y axis to dispatch: always
    /// 1.
    dispatch_y: u32,
    /// The number of workgroups along the abstract Z axis to dispatch: always
    /// 1.
    dispatch_z: u32,
    /// The actual number of work items.
    ///
    /// The GPU indirect dispatch doesn't read this, but it's used internally to
    /// determine the actual number of work items that exist in the late
    /// preprocessing work item buffer.
    work_item_count: u32,
    /// Padding to 64-byte boundaries for some hardware.
    pad: UVec4,
}

impl Default for LatePreprocessWorkItemIndirectParameters {
    fn default() -> LatePreprocessWorkItemIndirectParameters {
        LatePreprocessWorkItemIndirectParameters {
            dispatch_x: 0,
            dispatch_y: 1,
            dispatch_z: 1,
            work_item_count: 0,
            pad: default(),
        }
    }
}

/// Returns the set of work item buffers for the given view, first creating it
/// if necessary.
///
/// Bevy uses work item buffers to tell the mesh preprocessing compute shader
/// which meshes are to be drawn.
///
/// You may need to call this function if you're implementing your own custom
/// render phases. See the `specialized_mesh_pipeline` example.
pub fn get_or_create_work_item_buffer<'a, I>(
    work_item_buffers: &'a mut HashMap<RetainedViewEntity, PreprocessWorkItemBuffers>,
    view: RetainedViewEntity,
    no_indirect_drawing: bool,
    enable_gpu_occlusion_culling: bool,
) -> &'a mut PreprocessWorkItemBuffers
where
    I: 'static,
{
    let preprocess_work_item_buffers = match work_item_buffers.entry(view) {
        Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
        Entry::Vacant(vacant_entry) => {
            if no_indirect_drawing {
                vacant_entry.insert(PreprocessWorkItemBuffers::Direct(RawBufferVec::new(
                    BufferUsages::STORAGE,
                )))
            } else {
                vacant_entry.insert(PreprocessWorkItemBuffers::Indirect {
                    indexed: PartialBufferVec::new(
                        BufferUsages::STORAGE,
                        "indexed preprocess work item buffer".to_owned(),
                    ),
                    non_indexed: PartialBufferVec::new(
                        BufferUsages::STORAGE,
                        "non-indexed preprocess work item buffer".to_owned(),
                    ),
                    // We fill this in below if `enable_gpu_occlusion_culling`
                    // is set.
                    gpu_occlusion_culling: None,
                })
            }
        }
    };

    // Initialize the GPU occlusion culling buffers if necessary.
    if let PreprocessWorkItemBuffers::Indirect {
        ref mut gpu_occlusion_culling,
        ..
    } = *preprocess_work_item_buffers
    {
        match (
            enable_gpu_occlusion_culling,
            gpu_occlusion_culling.is_some(),
        ) {
            (false, false) | (true, true) => {}
            (false, true) => {
                *gpu_occlusion_culling = None;
            }
            (true, false) => {
                *gpu_occlusion_culling = Some(GpuOcclusionCullingWorkItemBuffers {
                    late_indexed: UninitBufferVec::new(BufferUsages::STORAGE),
                    late_non_indexed: UninitBufferVec::new(BufferUsages::STORAGE),
                    late_indirect_parameters_indexed_offset: 0,
                    late_indirect_parameters_non_indexed_offset: 0,
                });
            }
        }
    }

    preprocess_work_item_buffers
}

/// Initializes work item buffers for a phase in preparation for a new frame.
pub fn init_work_item_buffers(
    work_item_buffers: &mut PreprocessWorkItemBuffers,
    late_indexed_indirect_parameters_buffer: &'_ mut RawBufferVec<
        LatePreprocessWorkItemIndirectParameters,
    >,
    late_non_indexed_indirect_parameters_buffer: &'_ mut RawBufferVec<
        LatePreprocessWorkItemIndirectParameters,
    >,
) {
    // Add the offsets for indirect parameters that the late phase of mesh
    // preprocessing writes to.
    if let PreprocessWorkItemBuffers::Indirect {
        gpu_occlusion_culling:
            Some(GpuOcclusionCullingWorkItemBuffers {
                ref mut late_indirect_parameters_indexed_offset,
                ref mut late_indirect_parameters_non_indexed_offset,
                ..
            }),
        ..
    } = *work_item_buffers
    {
        *late_indirect_parameters_indexed_offset = late_indexed_indirect_parameters_buffer
            .push(LatePreprocessWorkItemIndirectParameters::default())
            as u32;
        *late_indirect_parameters_non_indexed_offset = late_non_indexed_indirect_parameters_buffer
            .push(LatePreprocessWorkItemIndirectParameters::default())
            as u32;
    }
}

impl PreprocessWorkItemBuffers {
    /// Adds a new work item to the appropriate buffer.
    ///
    /// `indexed` specifies whether the work item corresponds to an indexed
    /// mesh.
    pub fn push(&mut self, indexed: bool, preprocess_work_item: PreprocessWorkItem) {
        match *self {
            PreprocessWorkItemBuffers::Direct(ref mut buffer) => {
                buffer.push(preprocess_work_item);
            }
            PreprocessWorkItemBuffers::Indirect {
                indexed: ref mut indexed_buffer,
                non_indexed: ref mut non_indexed_buffer,
                ref mut gpu_occlusion_culling,
            } => {
                if indexed {
                    indexed_buffer.push_init(preprocess_work_item);
                } else {
                    non_indexed_buffer.push_init(preprocess_work_item);
                }

                if let Some(ref mut gpu_occlusion_culling) = *gpu_occlusion_culling {
                    if indexed {
                        gpu_occlusion_culling.late_indexed.add();
                    } else {
                        gpu_occlusion_culling.late_non_indexed.add();
                    }
                }
            }
        }
    }

    /// Clears out the GPU work item buffers in preparation for a new frame.
    pub fn clear(&mut self) {
        match *self {
            PreprocessWorkItemBuffers::Direct(ref mut buffer) => {
                buffer.clear();
            }
            PreprocessWorkItemBuffers::Indirect {
                indexed: ref mut indexed_buffer,
                non_indexed: ref mut non_indexed_buffer,
                ref mut gpu_occlusion_culling,
            } => {
                indexed_buffer.clear();
                non_indexed_buffer.clear();

                if let Some(ref mut gpu_occlusion_culling) = *gpu_occlusion_culling {
                    gpu_occlusion_culling.late_indexed.clear();
                    gpu_occlusion_culling.late_non_indexed.clear();
                    gpu_occlusion_culling.late_indirect_parameters_indexed_offset = 0;
                    gpu_occlusion_culling.late_indirect_parameters_non_indexed_offset = 0;
                }
            }
        }
    }
}

/// One invocation of the preprocessing shader: i.e. one mesh instance in a
/// view.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct PreprocessWorkItem {
    /// The index of the batch input data in the input buffer that the shader
    /// reads from.
    pub input_index: u32,

    /// In direct mode, the index of the mesh uniform; in indirect mode, the
    /// index of the [`IndirectParametersGpuMetadata`].
    ///
    /// In indirect mode, this is the index of the
    /// [`IndirectParametersGpuMetadata`] in the
    /// `IndirectParametersBuffers::indexed_metadata` or
    /// `IndirectParametersBuffers::non_indexed_metadata`.
    pub output_or_indirect_parameters_index: u32,
}

/// The `wgpu` indirect parameters structure that specifies a GPU draw command.
///
/// This is the variant for indexed meshes. We generate the instances of this
/// structure in the `build_indirect_params.wgsl` compute shader.
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct IndirectParametersIndexed {
    /// The number of indices that this mesh has.
    pub index_count: u32,
    /// The number of instances we are to draw.
    pub instance_count: u32,
    /// The offset of the first index for this mesh in the index buffer slab.
    pub first_index: u32,
    /// The offset of the first vertex for this mesh in the vertex buffer slab.
    pub base_vertex: u32,
    /// The index of the first mesh instance in the `MeshUniform` buffer.
    pub first_instance: u32,
}

/// The `wgpu` indirect parameters structure that specifies a GPU draw command.
///
/// This is the variant for non-indexed meshes. We generate the instances of
/// this structure in the `build_indirect_params.wgsl` compute shader.
#[derive(Clone, Copy, Debug, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct IndirectParametersNonIndexed {
    /// The number of vertices that this mesh has.
    pub vertex_count: u32,
    /// The number of instances we are to draw.
    pub instance_count: u32,
    /// The offset of the first vertex for this mesh in the vertex buffer slab.
    pub base_vertex: u32,
    /// The index of the first mesh instance in the `Mesh` buffer.
    pub first_instance: u32,
}

/// A structure, initialized on CPU and read on GPU, that contains metadata
/// about each batch.
///
/// Each batch will have one instance of this structure.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct IndirectParametersCpuMetadata {
    /// The index of the first instance of this mesh in the array of
    /// `MeshUniform`s.
    ///
    /// Note that this is the *first* output index in this batch. Since each
    /// instance of this structure refers to arbitrarily many instances, the
    /// `MeshUniform`s corresponding to this batch span the indices
    /// `base_output_index..(base_output_index + instance_count)`.
    pub base_output_index: u32,

    /// The index of the batch set that this batch belongs to in the
    /// [`IndirectBatchSet`] buffer.
    ///
    /// A *batch set* is a set of meshes that may be multi-drawn together.
    /// Multiple batches (and therefore multiple instances of
    /// [`IndirectParametersGpuMetadata`] structures) can be part of the same
    /// batch set.
    pub batch_set_index: u32,
}

/// A structure, written and read on GPU, that records how many instances of
/// each mesh are actually to be drawn.
///
/// The GPU mesh preprocessing shader increments the
/// [`Self::early_instance_count`] and [`Self::late_instance_count`] as it
/// determines that meshes are visible.  The indirect parameter building shader
/// reads this metadata in order to construct the indirect draw parameters.
///
/// Each batch will have one instance of this structure.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct IndirectParametersGpuMetadata {
    /// The index of the first mesh in this batch in the array of
    /// `MeshInputUniform`s.
    pub mesh_index: u32,

    /// The number of instances that were judged visible last frame.
    ///
    /// The CPU sets this value to 0, and the GPU mesh preprocessing shader
    /// increments it as it culls mesh instances.
    pub early_instance_count: u32,

    /// The number of instances that have been judged potentially visible this
    /// frame that weren't in the last frame's potentially visible set.
    ///
    /// The CPU sets this value to 0, and the GPU mesh preprocessing shader
    /// increments it as it culls mesh instances.
    pub late_instance_count: u32,
}

/// A structure, shared between CPU and GPU, that holds the number of on-GPU
/// indirect draw commands for each *batch set*.
///
/// A *batch set* is a set of meshes that may be multi-drawn together.
///
/// If the current hardware and driver support `multi_draw_indirect_count`, the
/// indirect parameters building shader increments
/// [`Self::indirect_parameters_count`] as it generates indirect parameters. The
/// `multi_draw_indirect_count` command reads
/// [`Self::indirect_parameters_count`] in order to determine how many commands
/// belong to each batch set.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct IndirectBatchSet {
    /// The number of indirect parameter commands (i.e. batches) in this batch
    /// set.
    ///
    /// The CPU sets this value to 0 before uploading this structure to GPU. The
    /// indirect parameters building shader increments this value as it creates
    /// indirect parameters. Then the `multi_draw_indirect_count` command reads
    /// this value in order to determine how many indirect draw commands to
    /// process.
    pub indirect_parameters_count: u32,

    /// The offset within the `IndirectParametersBuffers::indexed_data` or
    /// `IndirectParametersBuffers::non_indexed_data` of the first indirect draw
    /// command for this batch set.
    ///
    /// The CPU fills out this value.
    pub indirect_parameters_base: u32,
}

/// The buffers containing all the information that indirect draw commands
/// (`multi_draw_indirect`, `multi_draw_indirect_count`) use to draw the scene.
///
/// In addition to the indirect draw buffers themselves, this structure contains
/// the buffers that store [`IndirectParametersGpuMetadata`], which are the
/// structures that culling writes to so that the indirect parameter building
/// pass can determine how many meshes are actually to be drawn.
///
/// These buffers will remain empty if indirect drawing isn't in use.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct IndirectParametersBuffers {
    /// A mapping from a phase type ID to the indirect parameters buffers for
    /// that phase.
    ///
    /// Examples of phase type IDs are `Opaque3d` and `AlphaMask3d`.
    #[deref]
    pub buffers: TypeIdMap<UntypedPhaseIndirectParametersBuffers>,
}

/// Configuration for [`IndirectParametersBuffers`].
#[derive(Resource)]
pub struct IndirectParametersBuffersSettings {
    /// If true, this sets the `COPY_SRC` flag on indirect draw parameters so
    /// that they can be read back to CPU.
    ///
    /// This is a debugging feature that may reduce performance. It primarily
    /// exists for the `occlusion_culling` example.
    pub allow_copies_from_indirect_parameter_buffers: bool,
}

/// GPU-side information needed to unpack bins belonging to a single batch set.
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct GpuBinUnpackingMetadata {
    /// The index of the first `PreprocessWorkItem` that the compute shader
    /// dispatch is to write to.
    base_output_work_item_index: u32,
    /// The index of the first GPU indirect parameters command for this batch
    /// set.
    base_indirect_parameters_index: u32,
    /// The number of binned mesh instances in the `binned_mesh_instances`
    /// array.
    binned_mesh_instance_count: u32,
    /// Padding.
    pad: [u32; 61],
}

impl Default for GpuBinUnpackingMetadata {
    fn default() -> GpuBinUnpackingMetadata {
        GpuBinUnpackingMetadata {
            base_output_work_item_index: 0,
            base_indirect_parameters_index: 0,
            binned_mesh_instance_count: 0,
            pad: [0; _],
        }
    }
}

/// CPU-side information needed to construct the bind groups and issue the
/// dispatch for the `unpack_bins` shader, for a single batch set.
pub struct BinUnpackingJob {
    /// The GPU buffer of `GpuRenderBinnedMeshInstance`s corresponding to the
    /// mesh instances that this batch set contains.
    pub render_binned_mesh_instance_buffer: Buffer,
    /// The GPU buffer that maps each bin index to the index of the indirect
    /// drawing parameters for that bin, relative to the first such indirect
    /// drawing parameters for this batch set.
    pub bin_index_to_indirect_parameters_offset_buffer: Buffer,
    /// The index of this batch set's [`GpuBinUnpackingMetadata`] in the
    /// [`BinUnpackingBuffers::bin_unpacking_metadata`] buffer.
    pub bin_unpacking_metadata_index: BinUnpackingMetadataIndex,
    /// The total number of mesh instances in this batch set.
    pub mesh_instance_count: u32,
}

/// The buffers containing all the information that indirect draw commands use
/// to draw the scene, for a single phase.
///
/// This is the version of the structure that has a type parameter, so that the
/// batching for different phases can run in parallel.
///
/// See the [`IndirectParametersBuffers`] documentation for more information.
#[derive(Resource)]
pub struct PhaseIndirectParametersBuffers<PI>
where
    PI: PhaseItem,
{
    /// The indirect draw buffers for the phase.
    pub buffers: UntypedPhaseIndirectParametersBuffers,
    phantom: PhantomData<PI>,
}

impl<PI> FromWorld for PhaseIndirectParametersBuffers<PI>
where
    PI: PhaseItem,
{
    fn from_world(world: &mut World) -> Self {
        let settings = world.resource::<IndirectParametersBuffersSettings>();
        PhaseIndirectParametersBuffers {
            buffers: UntypedPhaseIndirectParametersBuffers::new(
                settings.allow_copies_from_indirect_parameter_buffers,
            ),
            phantom: PhantomData,
        }
    }
}

impl<PI> PhaseIndirectParametersBuffers<PI>
where
    PI: PhaseItem,
{
    /// Allocates a single set of indirect parameters in the appropriate buffer.
    fn allocate(&mut self, no_indirect_drawing: bool, item_is_indexed: bool) -> Option<u32> {
        if no_indirect_drawing {
            None
        } else if item_is_indexed {
            Some(self.buffers.indexed.allocate(1))
        } else {
            Some(self.buffers.non_indexed.allocate(1))
        }
    }
}

/// The buffers containing all the information that indirect draw commands use
/// to draw the scene, for a single phase.
///
/// This is the version of the structure that doesn't have a type parameter, so
/// that it can be inserted into [`IndirectParametersBuffers::buffers`]
///
/// See the [`IndirectParametersBuffers`] documentation for more information.
pub struct UntypedPhaseIndirectParametersBuffers {
    /// Information that indirect draw commands use to draw indexed meshes in
    /// the scene.
    pub indexed: MeshClassIndirectParametersBuffers<IndirectParametersIndexed>,
    /// Information that indirect draw commands use to draw non-indexed meshes
    /// in the scene.
    pub non_indexed: MeshClassIndirectParametersBuffers<IndirectParametersNonIndexed>,
}

impl UntypedPhaseIndirectParametersBuffers {
    /// Creates the indirect parameters buffers.
    pub fn new(
        allow_copies_from_indirect_parameter_buffers: bool,
    ) -> UntypedPhaseIndirectParametersBuffers {
        UntypedPhaseIndirectParametersBuffers {
            non_indexed: MeshClassIndirectParametersBuffers::new(
                allow_copies_from_indirect_parameter_buffers,
            ),
            indexed: MeshClassIndirectParametersBuffers::new(
                allow_copies_from_indirect_parameter_buffers,
            ),
        }
    }

    /// Reserves space for `count` new batches.
    ///
    /// The `indexed` parameter specifies whether the meshes that these batches
    /// correspond to are indexed or not.
    pub fn allocate(&mut self, indexed: bool, count: u32) -> u32 {
        if indexed {
            self.indexed.allocate(count)
        } else {
            self.non_indexed.allocate(count)
        }
    }

    /// Returns the number of batches currently allocated.
    ///
    /// The `indexed` parameter specifies whether the meshes that these batches
    /// correspond to are indexed or not.
    fn batch_count(&self, indexed: bool) -> usize {
        if indexed {
            self.indexed.batch_count()
        } else {
            self.non_indexed.batch_count()
        }
    }

    /// Returns the number of batch sets currently allocated.
    ///
    /// The `indexed` parameter specifies whether the meshes that these batch
    /// sets correspond to are indexed or not.
    pub fn batch_set_count(&self, indexed: bool) -> usize {
        if indexed {
            self.indexed.batch_sets.len()
        } else {
            self.non_indexed.batch_sets.len()
        }
    }

    /// Adds a new batch set to `Self::indexed_batch_sets` or
    /// `Self::non_indexed_batch_sets` as appropriate.
    ///
    /// `indexed` specifies whether the meshes that these batch sets correspond
    /// to are indexed or not. `indirect_parameters_base` specifies the offset
    /// within `Self::indexed_data` or `Self::non_indexed_data` of the first
    /// batch in this batch set.
    #[inline]
    pub fn add_batch_set(&mut self, indexed: bool, indirect_parameters_base: u32) {
        if indexed {
            self.indexed.batch_sets.push(IndirectBatchSet {
                indirect_parameters_base,
                indirect_parameters_count: 0,
            });
        } else {
            self.non_indexed.batch_sets.push(IndirectBatchSet {
                indirect_parameters_base,
                indirect_parameters_count: 0,
            });
        }
    }

    /// Returns the index that a newly-added batch set will have.
    ///
    /// The `indexed` parameter specifies whether the meshes in such a batch set
    /// are indexed or not.
    pub fn get_next_batch_set_index(&self, indexed: bool) -> Option<NonMaxU32> {
        NonMaxU32::new(self.batch_set_count(indexed) as u32)
    }

    /// Clears out the buffers in preparation for a new frame.
    pub fn clear(&mut self) {
        self.indexed.clear();
        self.non_indexed.clear();
    }
}

/// A resource, part of the render world, that holds all GPU buffers used for
/// the bin unpacking shader.
#[derive(Resource)]
pub struct BinUnpackingBuffers {
    /// A buffer containing all the uniforms needed to run the bin unpacking
    /// compute shader for each batch set.
    pub bin_unpacking_metadata: RawBufferVec<GpuBinUnpackingMetadata>,
    /// Per-view-phase buffers for the bin unpacking shader.
    pub view_phase_buffers: HashMap<BinUnpackingBuffersKey, ViewPhaseBinUnpackingBuffers>,
}

impl Default for BinUnpackingBuffers {
    fn default() -> Self {
        let mut bin_unpacking_metadata = RawBufferVec::new(BufferUsages::UNIFORM);
        bin_unpacking_metadata.set_label(Some("bin unpacking metadata buffer"));
        BinUnpackingBuffers {
            bin_unpacking_metadata,
            view_phase_buffers: HashMap::default(),
        }
    }
}

/// GPU buffers for the bin unpacking shader that are specific to each phase of
/// each view.
#[derive(Default)]
pub struct ViewPhaseBinUnpackingBuffers {
    /// Metadata that describes each unpacking job, specific to indexed meshes.
    pub indexed_unpacking_jobs: Vec<BinUnpackingJob>,
    /// Metadata that describes each unpacking job, specific to non-indexed
    /// meshes.
    pub non_indexed_unpacking_jobs: Vec<BinUnpackingJob>,
}

/// A key used to look up the bin unpacking buffers for a specific phase of a
/// specific view.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BinUnpackingBuffersKey {
    /// The ID of the phase.
    pub phase: TypeId,
    /// The entity ID of the view.
    pub view: RetainedViewEntity,
}

/// The index of the metadata corresponding to one bin unpacking job in the
/// [`BinUnpackingBuffers::bin_unpacking_metadata`] buffer.
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct BinUnpackingMetadataIndex(pub NonMaxU32);

impl BinUnpackingMetadataIndex {
    /// Returns the byte offset within the
    /// [`BinUnpackingBuffers::bin_unpacking_metadata`] buffer corresponding to
    /// this index.
    pub fn uniform_offset(&self) -> u32 {
        self.get() * size_of::<GpuBinUnpackingMetadata>() as u32
    }
}

/// The buffers containing all the information that indirect draw commands use
/// to draw the scene, for a single mesh class (indexed or non-indexed), for a
/// single phase.
pub struct MeshClassIndirectParametersBuffers<IP>
where
    IP: Clone + ShaderSize + WriteInto,
{
    /// The GPU buffer that stores the indirect draw parameters for the meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, while the
    /// `multi_draw_indirect` or `multi_draw_indirect_count` commands read from
    /// it to perform the draws.
    indirect_draw_parameters: UninitBufferVec<IP>,

    /// The GPU buffer that holds the data used to construct indirect draw
    /// parameters for meshes.
    ///
    /// The GPU mesh preprocessing shader writes to this buffer, and the
    /// indirect parameters building shader reads this buffer to construct the
    /// indirect draw parameters.
    cpu_metadata: RawBufferVec<IndirectParametersCpuMetadata>,

    /// The GPU buffer that holds data built by the GPU used to construct
    /// indirect draw parameters for meshes.
    ///
    /// The GPU mesh preprocessing shader writes to this buffer, and the
    /// indirect parameters building shader reads this buffer to construct the
    /// indirect draw parameters.
    gpu_metadata: UninitBufferVec<IndirectParametersGpuMetadata>,

    /// The GPU buffer that holds the number of indirect draw commands for each
    /// phase of each view, for meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, and the
    /// `multi_draw_indirect_count` command reads from it in order to know how
    /// many indirect draw commands to process.
    batch_sets: RawBufferVec<IndirectBatchSet>,
}

impl<IP> MeshClassIndirectParametersBuffers<IP>
where
    IP: Clone + ShaderSize + WriteInto,
{
    fn new(
        allow_copies_from_indirect_parameter_buffers: bool,
    ) -> MeshClassIndirectParametersBuffers<IP> {
        let mut indirect_parameter_buffer_usages = BufferUsages::STORAGE | BufferUsages::INDIRECT;
        if allow_copies_from_indirect_parameter_buffers {
            indirect_parameter_buffer_usages |= BufferUsages::COPY_SRC;
        }

        MeshClassIndirectParametersBuffers {
            indirect_draw_parameters: UninitBufferVec::new(indirect_parameter_buffer_usages),
            cpu_metadata: RawBufferVec::new(BufferUsages::STORAGE),
            gpu_metadata: UninitBufferVec::new(BufferUsages::STORAGE),
            batch_sets: RawBufferVec::new(indirect_parameter_buffer_usages),
        }
    }

    /// Returns the GPU buffer that stores the indirect draw parameters for
    /// indexed meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, while the
    /// `multi_draw_indirect` or `multi_draw_indirect_count` commands read from
    /// it to perform the draws.
    #[inline]
    pub fn data_buffer(&self) -> Option<&Buffer> {
        self.indirect_draw_parameters.buffer()
    }

    /// Returns the GPU buffer that holds the CPU-constructed data used to
    /// construct indirect draw parameters for meshes.
    ///
    /// The CPU writes to this buffer, and the indirect parameters building
    /// shader reads this buffer to construct the indirect draw parameters.
    #[inline]
    pub fn cpu_metadata_buffer(&self) -> Option<&Buffer> {
        self.cpu_metadata.buffer()
    }

    /// Returns the GPU buffer that holds the GPU-constructed data used to
    /// construct indirect draw parameters for meshes.
    ///
    /// The GPU mesh preprocessing shader writes to this buffer, and the
    /// indirect parameters building shader reads this buffer to construct the
    /// indirect draw parameters.
    #[inline]
    pub fn gpu_metadata_buffer(&self) -> Option<&Buffer> {
        self.gpu_metadata.buffer()
    }

    /// Returns the GPU buffer that holds the number of indirect draw commands
    /// for each phase of each view.
    ///
    /// The indirect parameters building shader writes to this buffer, and the
    /// `multi_draw_indirect_count` command reads from it in order to know how
    /// many indirect draw commands to process.
    #[inline]
    pub fn batch_sets_buffer(&self) -> Option<&Buffer> {
        self.batch_sets.buffer()
    }

    /// Reserves space for `count` new batches.
    ///
    /// This allocates in the [`Self::cpu_metadata`], [`Self::gpu_metadata`],
    /// and [`Self::indirect_draw_parameters`] buffers.
    fn allocate(&mut self, count: u32) -> u32 {
        let length = self.indirect_draw_parameters.len();
        self.cpu_metadata.reserve_internal(count as usize);
        self.gpu_metadata.add_multiple(count as usize);
        for _ in 0..count {
            self.indirect_draw_parameters.add();
            self.cpu_metadata
                .push(IndirectParametersCpuMetadata::default());
        }
        length as u32
    }

    /// Sets the [`IndirectParametersCpuMetadata`] for the mesh at the given
    /// index.
    pub fn set(&mut self, index: u32, value: IndirectParametersCpuMetadata) {
        self.cpu_metadata.set(index, value);
    }

    /// Returns the number of batches corresponding to meshes that are currently
    /// allocated.
    #[inline]
    pub fn batch_count(&self) -> usize {
        self.indirect_draw_parameters.len()
    }

    /// Clears out all the buffers in preparation for a new frame.
    pub fn clear(&mut self) {
        self.indirect_draw_parameters.clear();
        self.cpu_metadata.clear();
        self.gpu_metadata.clear();
        self.batch_sets.clear();
    }
}

impl FromWorld for GpuPreprocessingSupport {
    fn from_world(world: &mut World) -> Self {
        let adapter = world.resource::<RenderAdapter>();
        let device = world.resource::<RenderDevice>();

        // Filter Android drivers that are incompatible with GPU preprocessing:
        // - We filter out Adreno 730 and earlier GPUs (except 720, as it's newer
        //   than 730).
        // - We filter out Mali GPUs with driver versions lower than 48.
        // - We limit Pixel 10 GPUs (all versions for now) to preprocessing only (no culling)
        fn is_non_supported_android_device(adapter_info: &RenderAdapterInfo) -> bool {
            crate::get_adreno_model(adapter_info).is_some_and(|model| model != 720 && model <= 730)
                || crate::get_mali_driver_version(adapter_info).is_some_and(|version| version < 48)
        }
        fn is_preprocessing_only_android_device(adapter_info: &RenderAdapterInfo) -> bool {
            crate::get_pixel10_driver_version(adapter_info).is_some()
        }

        let culling_feature_support = device
            .features()
            .contains(Features::INDIRECT_FIRST_INSTANCE | Features::IMMEDIATES);
        // Depth downsampling for occlusion culling requires 12 textures
        // and the early occlusion culling pass requires 10 storage buffers
        let limit_support = device.limits().max_storage_textures_per_shader_stage >= 12 &&
            device.limits().max_storage_buffers_per_shader_stage >= 10 &&
            // Even if the adapter supports compute, we might be simulating a lack of
            // compute via device limits (see `WgpuSettingsPriority::WebGL2` and
            // `wgpu::Limits::downlevel_webgl2_defaults()`). This will have set all the
            // `max_compute_*` limits to zero, so we arbitrarily pick one as a canary.
            device.limits().max_compute_workgroup_storage_size != 0;

        let downlevel_support = adapter
            .get_downlevel_capabilities()
            .flags
            .contains(DownlevelFlags::COMPUTE_SHADERS);

        let adapter_info = RenderAdapterInfo(WgpuWrapper::new(adapter.get_info()));

        let max_supported_mode = if device.limits().max_compute_workgroup_size_x == 0
            || is_non_supported_android_device(&adapter_info)
            || adapter_info.backend == wgpu::Backend::Gl
        {
            info_once!(
                "GPU preprocessing is not supported on this device. \
                Falling back to CPU preprocessing.",
            );
            GpuPreprocessingMode::None
        } else if !(culling_feature_support && limit_support && downlevel_support)
            || is_preprocessing_only_android_device(&adapter_info)
        {
            info_once!("Some GPU preprocessing are limited on this device.");
            GpuPreprocessingMode::PreprocessingOnly
        } else {
            info_once!("GPU preprocessing is fully supported on this device.");
            GpuPreprocessingMode::Culling
        };

        GpuPreprocessingSupport { max_supported_mode }
    }
}

impl<BD, BDI> BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: AtomicPod,
{
    /// Creates new buffers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears out the buffers in preparation for a new frame.
    pub fn clear(&mut self) {
        for phase_instance_buffer in self.phase_instance_buffers.values_mut() {
            phase_instance_buffer.clear();
        }
    }
}

impl<BD> UntypedPhaseBatchedInstanceBuffers<BD>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
{
    pub fn new() -> Self {
        UntypedPhaseBatchedInstanceBuffers {
            data_buffer: UninitBufferVec::new(BufferUsages::STORAGE),
            work_item_buffers: HashMap::default(),
            late_indexed_indirect_parameters_buffer: RawBufferVec::new(
                BufferUsages::STORAGE | BufferUsages::INDIRECT,
            ),
            late_non_indexed_indirect_parameters_buffer: RawBufferVec::new(
                BufferUsages::STORAGE | BufferUsages::INDIRECT,
            ),
        }
    }

    /// Returns the binding of the buffer that contains the per-instance data.
    ///
    /// This buffer needs to be filled in via a compute shader.
    pub fn instance_data_binding(&self) -> Option<BindingResource<'_>> {
        self.data_buffer
            .buffer()
            .map(|buffer| buffer.as_entire_binding())
    }

    /// Clears out the buffers in preparation for a new frame.
    pub fn clear(&mut self) {
        self.data_buffer.clear();
        self.late_indexed_indirect_parameters_buffer.clear();
        self.late_non_indexed_indirect_parameters_buffer.clear();

        // Clear each individual set of buffers, but don't depopulate the hash
        // table. We want to avoid reallocating these vectors every frame.
        for view_work_item_buffers in self.work_item_buffers.values_mut() {
            view_work_item_buffers.clear();
        }
    }
}

impl<BD> Default for UntypedPhaseBatchedInstanceBuffers<BD>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a single render batch set that we're building up during a
/// sorted render phase.
struct SortedRenderBatchSet<F>
where
    F: GetBatchData,
{
    /// The index of the first phase item in this batch in the list of phase
    /// items.
    phase_item_start_index: u32,

    /// The index of the first instance in this batch in the instance buffer.
    instance_start_index: u32,

    /// True if the mesh in question has an index buffer; false otherwise.
    indexed: bool,

    /// The index of the indirect parameters for this batch in the
    /// [`IndirectParametersBuffers`].
    ///
    /// If CPU culling is being used, then this will be `None`.
    indirect_parameters_index_range: Option<Range<u32>>,

    /// Metadata that can be used to determine whether an instance can be placed
    /// into this batch.
    ///
    /// If `None`, the item inside is unbatchable.
    meta: Option<(BatchSetMeta<F::BatchSetCompareData>, F::BatchCompareData)>,
}

impl<F> SortedRenderBatchSet<F>
where
    F: GetBatchData,
{
    /// Finalizes this batch and updates the [`SortedRenderPhase`] with the
    /// appropriate indices.
    ///
    /// `instance_end_index` is the index of the last instance in this batch
    /// plus one.
    fn flush<I>(
        self,
        instance_end_index: u32,
        phase: &mut SortedRenderPhase<I>,
        phase_indirect_parameters_buffers: &mut UntypedPhaseIndirectParametersBuffers,
    ) where
        I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    {
        let (batch_range, batch_extra_index) =
            phase.items[self.phase_item_start_index as usize].batch_range_and_extra_index_mut();
        *batch_range = self.instance_start_index..instance_end_index;
        *batch_extra_index = match self.indirect_parameters_index_range {
            Some(ref indirect_parameters_index_range) => {
                PhaseItemExtraIndex::IndirectParametersIndex {
                    range: (*indirect_parameters_index_range).clone(),
                    batch_set_index: None,
                }
            }
            None => PhaseItemExtraIndex::None,
        };
        if let Some(ref indirect_parameters_index_range) = self.indirect_parameters_index_range {
            phase_indirect_parameters_buffers
                .add_batch_set(self.indexed, indirect_parameters_index_range.start);
        }
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
    // Don't clear the entire table, because that would delete the buffers, and
    // we want to reuse those allocations.
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
    extracted_views: Query<&ExtractedView>,
) where
    GFBD: GetFullBatchData,
{
    let retained_view_entities: HashSet<_> = extracted_views
        .iter()
        .map(|extracted_view| extracted_view.retained_view_entity)
        .collect();
    for phase_instance_buffers in gpu_batched_instance_buffers
        .phase_instance_buffers
        .values_mut()
    {
        phase_instance_buffers
            .work_item_buffers
            .retain(|retained_view_entity, _| {
                retained_view_entities.contains(retained_view_entity)
            });
    }
}

/// Batch the items in a sorted render phase, when GPU instance buffer building
/// is in use. This means comparing metadata needed to draw each phase item and
/// trying to combine the draws into a batch.
pub fn batch_and_prepare_sorted_render_phase<I, GFBD>(
    mut phase_batched_instance_buffers: ResMut<PhaseBatchedInstanceBuffers<I, GFBD::BufferData>>,
    mut phase_indirect_parameters_buffers: ResMut<PhaseIndirectParametersBuffers<I>>,
    mut sorted_render_phases: ResMut<ViewSortedRenderPhases<I>>,
    mut views: Query<(
        &ExtractedView,
        Has<NoIndirectDrawing>,
        Has<OcclusionCulling>,
    )>,
    system_param_item: StaticSystemParam<GFBD::Param>,
) where
    I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    GFBD: GetFullBatchData,
{
    // We only process GPU-built batch data in this function.
    let UntypedPhaseBatchedInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ref mut late_indexed_indirect_parameters_buffer,
        ref mut late_non_indexed_indirect_parameters_buffer,
    } = phase_batched_instance_buffers.buffers;

    for (extracted_view, no_indirect_drawing, gpu_occlusion_culling) in &mut views {
        let Some(phase) = sorted_render_phases.get_mut(&extracted_view.retained_view_entity) else {
            continue;
        };

        // Create the work item buffer if necessary.
        let work_item_buffer = get_or_create_work_item_buffer::<I>(
            work_item_buffers,
            extracted_view.retained_view_entity,
            no_indirect_drawing,
            gpu_occlusion_culling,
        );

        // Initialize those work item buffers in preparation for this new frame.
        init_work_item_buffers(
            work_item_buffer,
            late_indexed_indirect_parameters_buffer,
            late_non_indexed_indirect_parameters_buffer,
        );

        // Walk through the list of phase items, building up batches as we go.
        let mut batch_set: Option<SortedRenderBatchSet<GFBD>> = None;

        for current_index in 0..phase.items.len() {
            // Get the index of the input data, and comparison metadata, for
            // this entity.
            let item = &phase.items[current_index];
            let entity = item.main_entity();
            let item_is_indexed = item.indexed();
            let current_batch_input_index =
                GFBD::get_index_and_compare_data(&system_param_item, entity);

            // Unpack that index and metadata. Note that it's possible for index
            // and/or metadata to not be present, which signifies that this
            // entity is unbatchable. In that case, we break the batch here.
            // If the index isn't present the item is not part of this pipeline and so will be skipped.
            let Some((current_input_index, current_meta)) = current_batch_input_index else {
                // Break a batch if we need to.
                if let Some(batch_set) = batch_set.take() {
                    batch_set.flush(
                        data_buffer.len() as u32,
                        phase,
                        &mut phase_indirect_parameters_buffers.buffers,
                    );
                }

                continue;
            };
            let current_meta = current_meta.map(|(batch_set_meta, batch_meta)| {
                (
                    BatchSetMeta::new(&phase.items[current_index], batch_set_meta),
                    batch_meta,
                )
            });

            // Determine if this entity can be included in the batch we're
            // building up.
            let can_batch = match batch_set.as_ref() {
                None => SortedPhaseItemBatchability::BreakBatchSet,
                Some(batch_set) => match (&current_meta, &batch_set.meta) {
                    (
                        &Some((ref current_batch_set_key, ref current_bin_key)),
                        &Some((ref batch_set_key, ref bin_key)),
                    ) => {
                        if *current_batch_set_key == *batch_set_key {
                            if *current_bin_key == *bin_key {
                                SortedPhaseItemBatchability::BatchOk
                            } else {
                                SortedPhaseItemBatchability::BreakBatch
                            }
                        } else {
                            SortedPhaseItemBatchability::BreakBatchSet
                        }
                    }
                    _ => SortedPhaseItemBatchability::BreakBatchSet,
                },
            };

            // Make space in the data buffer for this instance.
            let output_index = data_buffer.add() as u32;

            // If we can't batch, break the existing batch or batch set and make
            // a new one.
            match can_batch {
                SortedPhaseItemBatchability::BreakBatchSet => {
                    // Flush the existing batch set.
                    if let Some(batch_set) = batch_set.take() {
                        batch_set.flush(
                            output_index,
                            phase,
                            &mut phase_indirect_parameters_buffers.buffers,
                        );
                    }

                    let indirect_parameters_index = phase_indirect_parameters_buffers
                        .allocate(no_indirect_drawing, item_is_indexed);

                    // Start a new batch.
                    if let Some(indirect_parameters_index) = indirect_parameters_index {
                        GFBD::write_batch_indirect_parameters_metadata(
                            item_is_indexed,
                            output_index,
                            None,
                            &mut phase_indirect_parameters_buffers.buffers,
                            indirect_parameters_index,
                        );
                    }

                    batch_set = Some(SortedRenderBatchSet {
                        phase_item_start_index: current_index as u32,
                        instance_start_index: output_index,
                        indexed: item_is_indexed,
                        indirect_parameters_index_range: indirect_parameters_index
                            .map(|i| i..(i + 1)),
                        meta: current_meta,
                    });
                }

                SortedPhaseItemBatchability::BreakBatch => {
                    // Allocate the indirect parameters.
                    let maybe_indirect_parameters_index = phase_indirect_parameters_buffers
                        .allocate(no_indirect_drawing, item_is_indexed);

                    if let (&mut Some(ref mut batch_set), Some(indirect_parameters_index)) =
                        (&mut batch_set, maybe_indirect_parameters_index)
                    {
                        GFBD::write_batch_indirect_parameters_metadata(
                            item_is_indexed,
                            output_index,
                            None,
                            &mut phase_indirect_parameters_buffers.buffers,
                            indirect_parameters_index,
                        );

                        batch_set.meta = current_meta;

                        let indirect_parameters_index_range = batch_set
                            .indirect_parameters_index_range
                            .as_mut()
                            .expect("Can't allocate in a multidraw set if we aren't multidrawing");
                        debug_assert_eq!(
                            indirect_parameters_index,
                            indirect_parameters_index_range.end
                        );
                        indirect_parameters_index_range.end += 1;
                    }
                }

                SortedPhaseItemBatchability::BatchOk => {}
            };

            // Add a new preprocessing work item so that the preprocessing
            // shader will copy the per-instance data over.
            if let Some(batch_set) = batch_set.as_ref() {
                work_item_buffer.push(
                    item_is_indexed,
                    PreprocessWorkItem {
                        input_index: current_input_index.into(),
                        output_or_indirect_parameters_index: match (
                            no_indirect_drawing,
                            &batch_set.indirect_parameters_index_range,
                        ) {
                            (true, _) => output_index,
                            (false, Some(indirect_parameters_index_range)) => {
                                indirect_parameters_index_range.end - 1
                            }
                            (false, None) => 0,
                        },
                    },
                );
            }
        }

        // Flush the final batch set if necessary.
        if let Some(batch_set) = batch_set.take() {
            batch_set.flush(
                data_buffer.len() as u32,
                phase,
                &mut phase_indirect_parameters_buffers.buffers,
            );
        }
    }
}

/// How a single sorted phase item can be batched with the previous phase item.
#[derive(Clone, Copy, PartialEq)]
enum SortedPhaseItemBatchability {
    /// The item can be batched with the previous item.
    BatchOk,
    /// The item can't be batched with the previous item, but can still go in
    /// the same batch set.
    ///
    /// That is, the item can be multi-drawn with the previous item.
    BreakBatch,
    /// The item needs to create a new batch set.
    BreakBatchSet,
}

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase<BPI, GFBD>(
    mut phase_batched_instance_buffers: ResMut<PhaseBatchedInstanceBuffers<BPI, GFBD::BufferData>>,
    phase_indirect_parameters_buffers: ResMut<PhaseIndirectParametersBuffers<BPI>>,
    mut binned_render_phases: ResMut<ViewBinnedRenderPhases<BPI>>,
    mut views: Query<
        (
            &ExtractedView,
            Has<NoIndirectDrawing>,
            Has<OcclusionCulling>,
        ),
        With<ExtractedView>,
    >,
    param: StaticSystemParam<GFBD::Param>,
) where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    let system_param_item = param.into_inner();

    let phase_indirect_parameters_buffers = phase_indirect_parameters_buffers.into_inner();

    let UntypedPhaseBatchedInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ref mut late_indexed_indirect_parameters_buffer,
        ref mut late_non_indexed_indirect_parameters_buffer,
    } = phase_batched_instance_buffers.buffers;

    for (extracted_view, no_indirect_drawing, gpu_occlusion_culling) in &mut views {
        let Some(phase) = binned_render_phases.get_mut(&extracted_view.retained_view_entity) else {
            continue;
        };

        // Create the work item buffer if necessary; otherwise, just mark it as
        // used this frame.
        let work_item_buffer = get_or_create_work_item_buffer::<BPI>(
            work_item_buffers,
            extracted_view.retained_view_entity,
            no_indirect_drawing,
            gpu_occlusion_culling,
        );

        // Initialize those work item buffers in preparation for this new frame.
        init_work_item_buffers(
            work_item_buffer,
            late_indexed_indirect_parameters_buffer,
            late_non_indexed_indirect_parameters_buffer,
        );

        // We prepare unbatchables, batchables, and multidrawables in that
        // order. This is because:
        //
        // 1. The `PreprocessWorkItem`s are stored in a `PartialBufferVec`.
        // 2. `PreprocessWorkItem`s corresponding to multidrawable mesh
        // instances are built on GPU via the `unpack_bins` shader.
        // 3. `PreprocessWorkItem`s corresponding to unbatchable and
        // batchable-but-not-multidrawable mesh instances are currently built on
        // the CPU.
        // 4. The `PartialBufferVec`s type enforces that CPU-initialized values
        // precede the uninitialized (i.e. GPU-initialized) ones.
        //
        // Thus, we have to make sure the preprocessing work items that the GPU
        // will build follow the preprocessing work items that the CPU built. We
        // do so by preparing the items in the order listed above.

        // Prepare unbatchables.

        for (key, unbatchables) in &mut phase.unbatchable_meshes {
            // Allocate the indirect parameters if necessary.
            let mut indirect_parameters_offset = if no_indirect_drawing {
                None
            } else if key.0.indexed() {
                Some(
                    phase_indirect_parameters_buffers
                        .buffers
                        .indexed
                        .allocate(unbatchables.entities.len() as u32),
                )
            } else {
                Some(
                    phase_indirect_parameters_buffers
                        .buffers
                        .non_indexed
                        .allocate(unbatchables.entities.len() as u32),
                )
            };

            for main_entity in unbatchables.entities.keys() {
                let Some(input_index) = GFBD::get_binned_index(&system_param_item, *main_entity)
                else {
                    continue;
                };
                let output_index = data_buffer.add() as u32;

                if let Some(ref mut indirect_parameters_index) = indirect_parameters_offset {
                    // We're in indirect mode, so add an indirect parameters
                    // index.
                    GFBD::write_batch_indirect_parameters_metadata(
                        key.0.indexed(),
                        output_index,
                        None,
                        &mut phase_indirect_parameters_buffers.buffers,
                        *indirect_parameters_index,
                    );
                    work_item_buffer.push(
                        key.0.indexed(),
                        PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_or_indirect_parameters_index: *indirect_parameters_index,
                        },
                    );
                    unbatchables
                        .buffer_indices
                        .add(UnbatchableBinnedEntityIndices {
                            instance_index: *indirect_parameters_index,
                            extra_index: PhaseItemExtraIndex::IndirectParametersIndex {
                                range: *indirect_parameters_index..(*indirect_parameters_index + 1),
                                batch_set_index: None,
                            },
                        });
                    phase_indirect_parameters_buffers
                        .buffers
                        .add_batch_set(key.0.indexed(), *indirect_parameters_index);
                    *indirect_parameters_index += 1;
                } else {
                    work_item_buffer.push(
                        key.0.indexed(),
                        PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_or_indirect_parameters_index: output_index,
                        },
                    );
                    unbatchables
                        .buffer_indices
                        .add(UnbatchableBinnedEntityIndices {
                            instance_index: output_index,
                            extra_index: PhaseItemExtraIndex::None,
                        });
                }
            }
        }

        // Prepare batchables.

        for (key, bin) in &phase.batchable_meshes {
            let mut batch: Option<BinnedRenderPhaseBatch> = None;
            for (&main_entity, &input_index) in bin.entities() {
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
                        work_item_buffer.push(
                            key.0.indexed(),
                            PreprocessWorkItem {
                                input_index: *input_index,
                                output_or_indirect_parameters_index: match (
                                    no_indirect_drawing,
                                    &batch.extra_index,
                                ) {
                                    (true, _) => output_index,
                                    (
                                        false,
                                        PhaseItemExtraIndex::IndirectParametersIndex {
                                            range: indirect_parameters_range,
                                            ..
                                        },
                                    ) => indirect_parameters_range.start,
                                    (false, &PhaseItemExtraIndex::DynamicOffset(_))
                                    | (false, &PhaseItemExtraIndex::None) => 0,
                                },
                            },
                        );
                    }

                    None if !no_indirect_drawing => {
                        // Start a new batch, in indirect mode.
                        let indirect_parameters_index = phase_indirect_parameters_buffers
                            .buffers
                            .allocate(key.0.indexed(), 1);
                        let batch_set_index = phase_indirect_parameters_buffers
                            .buffers
                            .get_next_batch_set_index(key.0.indexed());

                        GFBD::write_batch_indirect_parameters_metadata(
                            key.0.indexed(),
                            output_index,
                            batch_set_index,
                            &mut phase_indirect_parameters_buffers.buffers,
                            indirect_parameters_index,
                        );
                        work_item_buffer.push(
                            key.0.indexed(),
                            PreprocessWorkItem {
                                input_index: *input_index,
                                output_or_indirect_parameters_index: indirect_parameters_index,
                            },
                        );
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: (Entity::PLACEHOLDER, main_entity),
                            instance_range: output_index..output_index + 1,
                            extra_index: PhaseItemExtraIndex::IndirectParametersIndex {
                                range: indirect_parameters_index..(indirect_parameters_index + 1),
                                batch_set_index: None,
                            },
                        });
                    }

                    None => {
                        // Start a new batch, in direct mode.
                        work_item_buffer.push(
                            key.0.indexed(),
                            PreprocessWorkItem {
                                input_index: *input_index,
                                output_or_indirect_parameters_index: output_index,
                            },
                        );
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: (Entity::PLACEHOLDER, main_entity),
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
                            first_batch: batch,
                            batch_count: 1,
                            bin_key: key.1.clone(),
                            index: phase_indirect_parameters_buffers
                                .buffers
                                .batch_set_count(key.0.indexed())
                                as u32,
                            // Unused.
                            first_work_item_index: 0,
                        });
                    }
                }
            }
        }

        // Prepare multidrawables.

        if let (
            &mut BinnedRenderPhaseBatchSets::MultidrawIndirect(ref mut batch_sets),
            &mut PreprocessWorkItemBuffers::Indirect {
                indexed: ref mut indexed_work_item_buffer,
                non_indexed: ref mut non_indexed_work_item_buffer,
                gpu_occlusion_culling: ref mut gpu_occlusion_culling_buffers,
            },
        ) = (&mut phase.batch_sets, &mut *work_item_buffer)
        {
            // Initialize the state for both indexed and non-indexed meshes.
            let mut indexed_preparer: MultidrawableBatchSetPreparer<BPI, GFBD> =
                MultidrawableBatchSetPreparer::new(
                    phase_indirect_parameters_buffers.buffers.batch_count(true) as u32,
                    phase_indirect_parameters_buffers
                        .buffers
                        .indexed
                        .batch_sets
                        .len() as u32,
                );
            let mut non_indexed_preparer: MultidrawableBatchSetPreparer<BPI, GFBD> =
                MultidrawableBatchSetPreparer::new(
                    phase_indirect_parameters_buffers.buffers.batch_count(false) as u32,
                    phase_indirect_parameters_buffers
                        .buffers
                        .non_indexed
                        .batch_sets
                        .len() as u32,
                );

            // Prepare each batch set.
            for (batch_set_key, bins) in &phase.multidrawable_meshes {
                if batch_set_key.indexed() {
                    indexed_preparer.prepare_multidrawable_binned_batch_set(
                        bins,
                        data_buffer,
                        indexed_work_item_buffer,
                        &mut phase_indirect_parameters_buffers.buffers.indexed,
                        batch_sets,
                    );
                } else {
                    non_indexed_preparer.prepare_multidrawable_binned_batch_set(
                        bins,
                        data_buffer,
                        non_indexed_work_item_buffer,
                        &mut phase_indirect_parameters_buffers.buffers.non_indexed,
                        batch_sets,
                    );
                }
            }

            // Reserve space in the occlusion culling buffers, if necessary.
            if let Some(gpu_occlusion_culling_buffers) = gpu_occlusion_culling_buffers {
                gpu_occlusion_culling_buffers
                    .late_indexed
                    .add_multiple(indexed_preparer.work_item_count);
                gpu_occlusion_culling_buffers
                    .late_non_indexed
                    .add_multiple(non_indexed_preparer.work_item_count);
            }
        }
    }
}

/// The state that [`batch_and_prepare_binned_render_phase`] uses to construct
/// multidrawable batch sets.
///
/// The [`batch_and_prepare_binned_render_phase`] system maintains two of these:
/// one for indexed meshes and one for non-indexed meshes.
struct MultidrawableBatchSetPreparer<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    /// The offset in the indirect parameters buffer at which the next indirect
    /// parameters will be written.
    indirect_parameters_index: u32,
    /// The number of batch sets we've built so far for this mesh class.
    batch_set_index: u32,
    /// The number of work items we've emitted so far for this mesh class.
    work_item_count: usize,
    phantom: PhantomData<(BPI, GFBD)>,
}

impl<BPI, GFBD> MultidrawableBatchSetPreparer<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    /// Creates a new [`MultidrawableBatchSetPreparer`] that will start writing
    /// indirect parameters and batch sets at the given indices.
    #[inline]
    fn new(initial_indirect_parameters_index: u32, initial_batch_set_index: u32) -> Self {
        MultidrawableBatchSetPreparer {
            indirect_parameters_index: initial_indirect_parameters_index,
            batch_set_index: initial_batch_set_index,
            work_item_count: 0,
            phantom: PhantomData,
        }
    }

    /// Creates batch sets and writes the GPU data needed to draw all visible
    /// entities of one mesh class in the given batch set.
    ///
    /// The *mesh class* represents whether the mesh has indices or not.
    #[inline]
    fn prepare_multidrawable_binned_batch_set<IP>(
        &mut self,
        batch_set: &RenderMultidrawableBatchSet<BPI>,
        data_buffer: &mut UninitBufferVec<GFBD::BufferData>,
        work_item_buffer: &mut PartialBufferVec<PreprocessWorkItem>,
        mesh_class_buffers: &mut MeshClassIndirectParametersBuffers<IP>,
        batch_sets: &mut Vec<BinnedRenderPhaseBatchSet<BPI::BinKey>>,
    ) where
        IP: Clone + ShaderSize + WriteInto,
    {
        let current_indexed_batch_set_index = self.batch_set_index;
        let current_output_index = data_buffer.len() as u32;
        let first_work_item_index = work_item_buffer.len() as u32;

        let indirect_parameters_base = self.indirect_parameters_index;

        // We're going to write the first entity into the batch set. Do this
        // here so that we can preload the bin into cache as a side effect.
        let Some((first_bin_key, first_bin_index)) = batch_set.bin_key_to_bin_index.iter().next()
        else {
            return;
        };
        let first_bin = batch_set
            .bin(*first_bin_index)
            .expect("At least one bin must be present in each batch set");
        let first_bin_len = first_bin.entity_to_binned_mesh_instance_index.len();
        let first_bin_entity = batch_set
            .representative_entity()
            .unwrap_or(MainEntity::from(Entity::PLACEHOLDER));

        // Calculate where the mesh uniform (not the mesh input uniform) should
        // go for each mesh instance in our bins. This entails performing a
        // prefix sum on the number of elements in each bin. First, initialize
        // each base output index to zero.
        //
        // TODO: Eventually, this should be done on GPU with a prefix sum. We
        // don't want any per-bin work to be done on CPU for bins that didn't
        // change since the last frame.
        let cpu_metadata_offset = mesh_class_buffers.cpu_metadata.len() as u32;
        for _ in 0..batch_set.bin_count() {
            mesh_class_buffers
                .cpu_metadata
                .push(IndirectParametersCpuMetadata {
                    // We fill this in later.
                    base_output_index: 0,
                    batch_set_index: self.batch_set_index,
                });
        }

        // Next, traverse each bin and allocate the position of each mesh
        // uniform in it. Additionally, reserve space for the mesh instances in
        // the buffers.
        for bin_index in batch_set.bin_key_to_bin_index.values() {
            let bin = batch_set.bin(*bin_index).expect("Bin not present");

            // Allocate the indirect parameters.
            let indirect_parameters_offset = *batch_set
                .gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .get(bin_index.0)
                .unwrap();
            mesh_class_buffers.cpu_metadata.values_mut()
                [cpu_metadata_offset as usize + indirect_parameters_offset as usize]
                .base_output_index = data_buffer.len() as u32;

            // Reserve space for the appropriate number of entities in the work
            // item buffer and data buffer. Also, advance the output index and
            // work item count.
            let bin_entity_count = bin.entity_to_binned_mesh_instance_index.len();
            work_item_buffer.push_multiple_uninit(bin_entity_count);
            data_buffer.add_multiple(bin_entity_count);
            self.work_item_count += bin_entity_count;
        }

        // Reserve space for the bins in this batch set in the GPU buffers.
        let bin_count = batch_set.bin_count();
        mesh_class_buffers.gpu_metadata.add_multiple(bin_count);
        mesh_class_buffers
            .indirect_draw_parameters
            .add_multiple(bin_count);

        // Write the information the GPU will need about this batch set.
        mesh_class_buffers.batch_sets.push(IndirectBatchSet {
            indirect_parameters_base,
            indirect_parameters_count: 0,
        });

        self.indirect_parameters_index += bin_count as u32;
        self.batch_set_index += 1;

        // Record the batch set. The render node later processes this record to
        // render the batches.
        batch_sets.push(BinnedRenderPhaseBatchSet {
            first_batch: BinnedRenderPhaseBatch {
                representative_entity: (Entity::PLACEHOLDER, first_bin_entity),
                instance_range: current_output_index..(current_output_index + first_bin_len as u32),
                extra_index: PhaseItemExtraIndex::maybe_indirect_parameters_index(NonMaxU32::new(
                    indirect_parameters_base,
                )),
            },
            bin_key: (*first_bin_key).clone(),
            batch_count: self.indirect_parameters_index - indirect_parameters_base,
            index: current_indexed_batch_set_index,
            first_work_item_index,
        });
    }
}

/// A system that gathers up the per-phase GPU buffers and inserts them into the
/// [`BatchedInstanceBuffers`] and [`IndirectParametersBuffers`] tables.
///
/// This runs after the [`batch_and_prepare_binned_render_phase`] or
/// [`batch_and_prepare_sorted_render_phase`] systems. It takes the per-phase
/// [`PhaseBatchedInstanceBuffers`] and [`PhaseIndirectParametersBuffers`]
/// resources and inserts them into the global [`BatchedInstanceBuffers`] and
/// [`IndirectParametersBuffers`] tables.
///
/// This system exists so that the [`batch_and_prepare_binned_render_phase`] and
/// [`batch_and_prepare_sorted_render_phase`] can run in parallel with one
/// another. If those two systems manipulated [`BatchedInstanceBuffers`] and
/// [`IndirectParametersBuffers`] directly, then they wouldn't be able to run in
/// parallel.
pub fn collect_buffers_for_phase<PI, GFBD>(
    mut phase_batched_instance_buffers: ResMut<PhaseBatchedInstanceBuffers<PI, GFBD::BufferData>>,
    mut phase_indirect_parameters_buffers: ResMut<PhaseIndirectParametersBuffers<PI>>,
    mut batched_instance_buffers: ResMut<
        BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>,
    >,
    mut indirect_parameters_buffers: ResMut<IndirectParametersBuffers>,
    indirect_parameters_buffers_settings: Res<IndirectParametersBuffersSettings>,
) where
    PI: PhaseItem,
    GFBD: GetFullBatchData + Send + Sync + 'static,
{
    // Insert the `PhaseBatchedInstanceBuffers` into the global table. Replace
    // the contents of the per-phase resource with the old batched instance
    // buffers in order to reuse allocations.
    let untyped_phase_batched_instance_buffers =
        mem::take(&mut phase_batched_instance_buffers.buffers);
    if let Some(mut old_untyped_phase_batched_instance_buffers) = batched_instance_buffers
        .phase_instance_buffers
        .insert(TypeId::of::<PI>(), untyped_phase_batched_instance_buffers)
    {
        old_untyped_phase_batched_instance_buffers.clear();
        phase_batched_instance_buffers.buffers = old_untyped_phase_batched_instance_buffers;
    }

    // Insert the `PhaseIndirectParametersBuffers` into the global table.
    // Replace the contents of the per-phase resource with the old indirect
    // parameters buffers in order to reuse allocations.
    let untyped_phase_indirect_parameters_buffers = mem::replace(
        &mut phase_indirect_parameters_buffers.buffers,
        UntypedPhaseIndirectParametersBuffers::new(
            indirect_parameters_buffers_settings.allow_copies_from_indirect_parameter_buffers,
        ),
    );
    if let Some(mut old_untyped_phase_indirect_parameters_buffers) = indirect_parameters_buffers
        .insert(
            TypeId::of::<PI>(),
            untyped_phase_indirect_parameters_buffers,
        )
    {
        old_untyped_phase_indirect_parameters_buffers.clear();
        phase_indirect_parameters_buffers.buffers = old_untyped_phase_indirect_parameters_buffers;
    }
}

/// A system that writes all instance buffers to the GPU.
pub fn write_batched_instance_buffers<GFBD>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    pipeline_cache: Res<PipelineCache>,
    mut bin_unpacking_buffers: ResMut<BinUnpackingBuffers>,
    mut sparse_buffer_update_jobs: ResMut<SparseBufferUpdateJobs>,
    mut sparse_buffer_update_bind_groups: ResMut<SparseBufferUpdateBindGroups>,
    sparse_buffer_update_pipelines: Res<SparseBufferUpdatePipelines>,
) where
    GFBD: GetFullBatchData,
{
    let BatchedInstanceBuffers {
        current_input_buffer,
        previous_input_buffer,
        phase_instance_buffers,
    } = gpu_array_buffer.into_inner();

    let render_device = &*render_device;
    let render_queue = &*render_queue;

    ComputeTaskPool::get().scope(|scope| {
        scope.spawn(async {
            #[cfg(feature = "trace")]
            let _span = bevy_log::info_span!("write_current_input_buffers").entered();
            current_input_buffer
                .buffer
                .write_buffers(render_device, render_queue);
        });
        scope.spawn(async {
            #[cfg(feature = "trace")]
            let _span = bevy_log::info_span!("write_previous_input_buffers").entered();
            previous_input_buffer.write_buffer(render_device, render_queue);
        });

        for phase_instance_buffers in phase_instance_buffers.values_mut() {
            let UntypedPhaseBatchedInstanceBuffers {
                ref mut data_buffer,
                ref mut work_item_buffers,
                ref mut late_indexed_indirect_parameters_buffer,
                ref mut late_non_indexed_indirect_parameters_buffer,
            } = *phase_instance_buffers;

            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("write_phase_instance_buffers").entered();
                data_buffer.write_buffer(render_device);
                late_indexed_indirect_parameters_buffer.write_buffer(render_device, render_queue);
                late_non_indexed_indirect_parameters_buffer
                    .write_buffer(render_device, render_queue);
            });

            for phase_work_item_buffers in work_item_buffers.values_mut() {
                scope.spawn(async {
                    #[cfg(feature = "trace")]
                    let _span = bevy_log::info_span!("write_work_item_buffers").entered();
                    match *phase_work_item_buffers {
                        PreprocessWorkItemBuffers::Direct(ref mut buffer_vec) => {
                            buffer_vec.write_buffer(render_device, render_queue);
                        }
                        PreprocessWorkItemBuffers::Indirect {
                            ref mut indexed,
                            ref mut non_indexed,
                            ref mut gpu_occlusion_culling,
                        } => {
                            indexed.write_buffer(render_device, render_queue);
                            non_indexed.write_buffer(render_device, render_queue);

                            if let Some(GpuOcclusionCullingWorkItemBuffers {
                                ref mut late_indexed,
                                ref mut late_non_indexed,
                                late_indirect_parameters_indexed_offset: _,
                                late_indirect_parameters_non_indexed_offset: _,
                            }) = *gpu_occlusion_culling
                            {
                                if !late_indexed.is_empty() {
                                    late_indexed.write_buffer(render_device);
                                }
                                if !late_non_indexed.is_empty() {
                                    late_non_indexed.write_buffer(render_device);
                                }
                            }
                        }
                    }
                });
            }
        }
    });

    // Create the resources necessary to perform sparse uploads of the current
    // input buffer if necessary.
    current_input_buffer.buffer.prepare_to_populate_buffers(
        render_device,
        &pipeline_cache,
        &mut sparse_buffer_update_jobs,
        &mut sparse_buffer_update_bind_groups,
        &sparse_buffer_update_pipelines,
    );

    bin_unpacking_buffers
        .bin_unpacking_metadata
        .write_buffer(render_device, render_queue);
}

/// Writes the bin data for each render phase to the GPU.
///
/// The bin data consists of the IDs of the mesh instances, as well as the
/// metadata needed for the `unpack_bins` shader to unpack them.
pub fn write_binned_instance_buffers<BPI, GFBD>(
    mut views: Query<&ExtractedView>,
    mut view_binned_render_phases: ResMut<ViewBinnedRenderPhases<BPI>>,
    bin_unpacking_buffers: ResMut<BinUnpackingBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    let bin_unpacking_buffers = bin_unpacking_buffers.into_inner();

    let phase_type_id = TypeId::of::<BPI>();

    // Record all the `RetainedViewEntity` keys that we saw so that we can
    // delete buffers corresponding to views that went away.
    let mut all_seen_view_entities = HashSet::new();

    for extracted_view in &mut views {
        all_seen_view_entities.insert(extracted_view.retained_view_entity);

        let Some(view_binned_render_phase) =
            view_binned_render_phases.get_mut(&extracted_view.retained_view_entity)
        else {
            continue;
        };

        // Since we currently only perform GPU-side bin unpacking for multidrawn
        // batch sets, we bail out for all other types of batch sets.
        let BinnedRenderPhaseBatchSets::MultidrawIndirect(ref batch_sets) =
            view_binned_render_phase.batch_sets
        else {
            continue;
        };

        // Get or create the bin unpacking buffers for this (view, phase)
        // combination.
        let view_phase_bin_unpacking_buffers = bin_unpacking_buffers
            .view_phase_buffers
            .entry(BinUnpackingBuffersKey {
                phase: phase_type_id,
                view: extracted_view.retained_view_entity,
            })
            .or_default();

        // Clear out the list of jobs.
        view_phase_bin_unpacking_buffers
            .indexed_unpacking_jobs
            .clear();
        view_phase_bin_unpacking_buffers
            .non_indexed_unpacking_jobs
            .clear();

        // Our goal is to extract the output work item location and indirect
        // parameters info from the flat `batch_sets` list and to use it to
        // build each batch set's `GpuBinUnpackingMetadata`. To do that, we
        // first loop over each batch set in the `batch_set` list and add the
        // extracted entry to the
        // `representative_entity_to_batch_set_bin_unpacking_metadata` table.

        // We use the *representative entity* as the key for the later loop to
        // find the `BatchSetBinUnpackingMetadata`, because it's a unique value
        // that can be fetched from the `BinnedRenderPhaseBatchSet`.
        let mut representative_entity_to_batch_set_bin_unpacking_metadata =
            MainEntityHashMap::default();

        for batch_set in batch_sets {
            let main_entity = batch_set.first_batch.representative_entity.1;
            if *main_entity != Entity::PLACEHOLDER
                && let PhaseItemExtraIndex::IndirectParametersIndex {
                    range: ref indirect_parameters_range,
                    ..
                } = batch_set.first_batch.extra_index
            {
                // Record the batch set bin unpacking metadata for later passes
                // to use.
                representative_entity_to_batch_set_bin_unpacking_metadata.insert(
                    main_entity,
                    BatchSetBinUnpackingMetadata {
                        base_output_work_item_index: batch_set.first_work_item_index,
                        base_indirect_parameters_index: indirect_parameters_range.start,
                    },
                );
            }
        }

        // Now loop over all the batch sets in the phase. Look up the
        // corresponding `BatchSetBinUnpackingMetadata`, and use it to prepare
        // the `GpuBinUnpackingMetadata` and the `BinUnpackingJob`s. Also, kick
        // off writes for all the associated GPU buffers that we'd been building
        // up in earlier phases.
        for (batch_set_key, batch_set) in view_binned_render_phase.multidrawable_meshes.iter_mut() {
            let Some(representative_entity) = batch_set.representative_entity() else {
                continue;
            };
            let Some(bin_unpacking_metadata) =
                representative_entity_to_batch_set_bin_unpacking_metadata
                    .get(&representative_entity)
            else {
                continue;
            };

            // Write the various buffers to the GPU.

            batch_set
                .gpu_buffers
                .render_binned_mesh_instance_buffer
                .write_buffer(&render_device, &render_queue);
            batch_set
                .gpu_buffers
                .bin_index_to_indirect_parameters_offset_buffer
                .write_buffer(&render_device, &render_queue);

            let (
                Some(render_bin_entry_buffer),
                Some(bin_index_to_indirect_parameters_offset_buffer),
            ) = (
                batch_set
                    .gpu_buffers
                    .render_binned_mesh_instance_buffer
                    .buffer(),
                batch_set
                    .gpu_buffers
                    .bin_index_to_indirect_parameters_offset_buffer
                    .buffer(),
            )
            else {
                continue;
            };

            let binned_mesh_instance_count = batch_set
                .gpu_buffers
                .render_binned_mesh_instance_buffer
                .len() as u32;

            // Build up the `GpuBinUnpackingMetadata` for this batch set.
            let gpu_bin_unpacking_metadata_index = bin_unpacking_buffers
                .bin_unpacking_metadata
                .push(GpuBinUnpackingMetadata {
                    base_output_work_item_index: bin_unpacking_metadata.base_output_work_item_index,
                    base_indirect_parameters_index: bin_unpacking_metadata
                        .base_indirect_parameters_index,
                    binned_mesh_instance_count,
                    pad: [0; _],
                });

            let Some(gpu_bin_unpacking_metadata_index) =
                NonMaxU32::new(gpu_bin_unpacking_metadata_index as u32)
            else {
                continue;
            };

            // Create the [`BinUnpackingJob`].
            let job = BinUnpackingJob {
                render_binned_mesh_instance_buffer: render_bin_entry_buffer.clone(),
                bin_index_to_indirect_parameters_offset_buffer:
                    bin_index_to_indirect_parameters_offset_buffer.clone(),
                bin_unpacking_metadata_index: BinUnpackingMetadataIndex(
                    gpu_bin_unpacking_metadata_index,
                ),
                mesh_instance_count: binned_mesh_instance_count,
            };

            if batch_set_key.indexed() {
                view_phase_bin_unpacking_buffers
                    .indexed_unpacking_jobs
                    .push(job);
            } else {
                view_phase_bin_unpacking_buffers
                    .non_indexed_unpacking_jobs
                    .push(job);
            }
        }
    }

    // Delete buffers corresponding to dead views.
    bin_unpacking_buffers
        .view_phase_buffers
        .retain(|bin_unpacking_buffers_key, _| {
            bin_unpacking_buffers_key.phase != phase_type_id
                || all_seen_view_entities.contains(&bin_unpacking_buffers_key.view)
        });
}

/// Clears out the [`BinUnpackingBuffers`] in preparation for a new frame.
pub fn clear_bin_unpacking_buffers(mut bin_unpacking_buffers: ResMut<BinUnpackingBuffers>) {
    bin_unpacking_buffers.bin_unpacking_metadata.clear();
}

/// CPU-side metadata needed to drive the bin unpacking compute shader for a
/// single batch set.
struct BatchSetBinUnpackingMetadata {
    /// The index of the first [`PreprocessWorkItem`] that the compute shader
    /// dispatch is to write to.
    base_output_work_item_index: u32,
    /// The index of the first GPU indirect parameters command for the batch
    /// set.
    base_indirect_parameters_index: u32,
}

pub fn clear_indirect_parameters_buffers(
    mut indirect_parameters_buffers: ResMut<IndirectParametersBuffers>,
) {
    for phase_indirect_parameters_buffers in indirect_parameters_buffers.values_mut() {
        phase_indirect_parameters_buffers.clear();
    }
}

pub fn write_indirect_parameters_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut indirect_parameters_buffers: ResMut<IndirectParametersBuffers>,
) {
    let render_device = &*render_device;
    let render_queue = &*render_queue;
    ComputeTaskPool::get().scope(|scope| {
        for phase_indirect_parameters_buffers in indirect_parameters_buffers.values_mut() {
            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("indexed_data").entered();
                phase_indirect_parameters_buffers
                    .indexed
                    .indirect_draw_parameters
                    .write_buffer(render_device);
            });
            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("non_indexed_data").entered();
                phase_indirect_parameters_buffers
                    .non_indexed
                    .indirect_draw_parameters
                    .write_buffer(render_device);
            });

            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("indexed_cpu_metadata").entered();
                phase_indirect_parameters_buffers
                    .indexed
                    .cpu_metadata
                    .write_buffer(render_device, render_queue);
            });
            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("non_indexed_cpu_metadata").entered();
                phase_indirect_parameters_buffers
                    .non_indexed
                    .cpu_metadata
                    .write_buffer(render_device, render_queue);
            });

            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("non_indexed_gpu_metadata").entered();
                phase_indirect_parameters_buffers
                    .non_indexed
                    .gpu_metadata
                    .write_buffer(render_device);
            });
            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("indexed_gpu_metadata").entered();
                phase_indirect_parameters_buffers
                    .indexed
                    .gpu_metadata
                    .write_buffer(render_device);
            });

            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("indexed_batch_sets").entered();
                phase_indirect_parameters_buffers
                    .indexed
                    .batch_sets
                    .write_buffer(render_device, render_queue);
            });
            scope.spawn(async {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("non_indexed_batch_sets").entered();
                phase_indirect_parameters_buffers
                    .non_indexed
                    .batch_sets
                    .write_buffer(render_device, render_queue);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use bytemuck::{Pod, Zeroable};

    use crate::impl_atomic_pod;

    use super::*;

    #[derive(Clone, Copy, Default, PartialEq, Debug, Pod, Zeroable)]
    #[repr(C)]
    struct TestData(u32);

    impl_atomic_pod!(TestData, TestDataBlob);

    #[test]
    fn instance_buffer_correct_behavior() {
        let mut instance_buffer = InstanceInputUniformBuffer::new();

        let index = instance_buffer.add(TestData(2));
        instance_buffer.remove(index);
        assert_eq!(instance_buffer.get_unchecked(index), TestData(2));
        assert_eq!(instance_buffer.get(index), None);

        instance_buffer.add(TestData(5));
        assert_eq!(instance_buffer.buffer().len(), 1);
    }
}
