//! Batching functionality when GPU preprocessing is in use.

use core::any::TypeId;

use bevy_app::{App, Plugin};
use bevy_ecs::{
    entity::{hash_map::EntityHashMap, Entity},
    query::{Has, With},
    resource::Resource,
    schedule::IntoSystemConfigs as _,
    system::{Query, Res, ResMut, StaticSystemParam},
    world::{FromWorld, World},
};
use bevy_encase_derive::ShaderType;
use bevy_math::UVec4;
use bevy_platform_support::collections::hash_map::Entry;
use bevy_utils::{default, TypeIdMap};
use bytemuck::{Pod, Zeroable};
use nonmax::NonMaxU32;
use tracing::error;
use wgpu::{BindingResource, BufferUsages, DownlevelFlags, Features};

use crate::{
    experimental::occlusion_culling::OcclusionCulling,
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhaseBatch, BinnedRenderPhaseBatchSet,
        BinnedRenderPhaseBatchSets, CachedRenderPipelinePhaseItem, PhaseItemBatchSetKey as _,
        PhaseItemExtraIndex, SortedPhaseItem, SortedRenderPhase, UnbatchableBinnedEntityIndices,
        ViewBinnedRenderPhases, ViewSortedRenderPhases,
    },
    render_resource::{Buffer, BufferVec, GpuArrayBufferable, RawBufferVec, UninitBufferVec},
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    view::{ExtractedView, NoIndirectDrawing},
    Render, RenderApp, RenderSet,
};

use super::{BatchMeta, GetBatchData, GetFullBatchData};

#[derive(Default)]
pub struct BatchingPlugin {
    /// If true, this sets the `COPY_SRC` flag on indirect draw parameters so
    /// that they can be read back to CPU.
    ///
    /// This is a debugging feature that may reduce performance. It primarily
    /// exists for the `occlusion_culling` example.
    pub allow_copies_from_indirect_parameters: bool,
}

impl Plugin for BatchingPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(IndirectParametersBuffers::new(
                self.allow_copies_from_indirect_parameters,
            ))
            .add_systems(
                Render,
                write_indirect_parameters_buffers.in_set(RenderSet::PrepareResourcesFlush),
            )
            .add_systems(
                Render,
                clear_indirect_parameters_buffers.in_set(RenderSet::ManageViews),
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
    pub work_item_buffers: EntityHashMap<TypeIdMap<PreprocessWorkItemBuffers>>,

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

    /// Returns the number of instances in this buffer.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if this buffer has no instances or false if it contains any
    /// instances.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Consumes this [`InstanceInputUniformBuffer`] and returns the raw buffer
    /// ready to be uploaded to the GPU.
    pub fn into_buffer(self) -> RawBufferVec<BDI> {
        self.buffer
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
pub enum PreprocessWorkItemBuffers {
    /// The work items we use if we aren't using indirect drawing.
    ///
    /// Because we don't have to separate indexed from non-indexed meshes in
    /// direct mode, we only have a single buffer here.
    Direct(BufferVec<PreprocessWorkItem>),

    /// The buffer of work items we use if we are using indirect drawing.
    ///
    /// We need to separate out indexed meshes from non-indexed meshes in this
    /// case because the indirect parameters for these two types of meshes have
    /// different sizes.
    Indirect {
        /// The buffer of work items corresponding to indexed meshes.
        indexed: BufferVec<PreprocessWorkItem>,
        /// The buffer of work items corresponding to non-indexed meshes.
        non_indexed: BufferVec<PreprocessWorkItem>,
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
    /// [`BatchedInstanceBuffers::late_indexed_indirect_parameters_buffer`]
    /// where this view's indirect dispatch counts for indexed meshes live.
    pub late_indirect_parameters_indexed_offset: u32,
    /// The offset into the
    /// [`BatchedInstanceBuffers::late_non_indexed_indirect_parameters_buffer`]
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
    work_item_buffers: &'a mut EntityHashMap<TypeIdMap<PreprocessWorkItemBuffers>>,
    view: Entity,
    no_indirect_drawing: bool,
    gpu_occlusion_culling: bool,
    late_indexed_indirect_parameters_buffer: &'_ mut RawBufferVec<
        LatePreprocessWorkItemIndirectParameters,
    >,
    late_non_indexed_indirect_parameters_buffer: &'_ mut RawBufferVec<
        LatePreprocessWorkItemIndirectParameters,
    >,
) -> &'a mut PreprocessWorkItemBuffers
where
    I: 'static,
{
    match work_item_buffers
        .entry(view)
        .or_default()
        .entry(TypeId::of::<I>())
    {
        Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
        Entry::Vacant(vacant_entry) => {
            if no_indirect_drawing {
                vacant_entry.insert(PreprocessWorkItemBuffers::Direct(BufferVec::new(
                    BufferUsages::STORAGE,
                )))
            } else {
                vacant_entry.insert(PreprocessWorkItemBuffers::Indirect {
                    indexed: BufferVec::new(BufferUsages::STORAGE),
                    non_indexed: BufferVec::new(BufferUsages::STORAGE),
                    gpu_occlusion_culling: if gpu_occlusion_culling {
                        let late_indirect_parameters_indexed_offset =
                            late_indexed_indirect_parameters_buffer
                                .push(LatePreprocessWorkItemIndirectParameters::default());
                        let late_indirect_parameters_non_indexed_offset =
                            late_non_indexed_indirect_parameters_buffer
                                .push(LatePreprocessWorkItemIndirectParameters::default());
                        Some(GpuOcclusionCullingWorkItemBuffers {
                            late_indexed: UninitBufferVec::new(BufferUsages::STORAGE),
                            late_non_indexed: UninitBufferVec::new(BufferUsages::STORAGE),
                            late_indirect_parameters_indexed_offset:
                                late_indirect_parameters_indexed_offset as u32,
                            late_indirect_parameters_non_indexed_offset:
                                late_indirect_parameters_non_indexed_offset as u32,
                        })
                    } else {
                        None
                    },
                })
            }
        }
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
                    indexed_buffer.push(preprocess_work_item);
                } else {
                    non_indexed_buffer.push(preprocess_work_item);
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
}

/// One invocation of the preprocessing shader: i.e. one mesh instance in a
/// view.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct PreprocessWorkItem {
    /// The index of the batch input data in the input buffer that the shader
    /// reads from.
    pub input_index: u32,
    /// The index of the `MeshUniform` in the output buffer that we write to.
    /// In direct mode, this is the index of the uniform. In indirect mode, this
    /// is the first index uniform in the batch set.
    pub output_index: u32,
    /// The index of the [`IndirectParametersMetadata`] in the
    /// `IndirectParametersBuffers::indexed_metadata` or
    /// `IndirectParametersBuffers::non_indexed_metadata`.
    pub indirect_parameters_index: u32,
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

/// A structure, shared between CPU and GPU, that records how many instances of
/// each mesh are actually to be drawn.
///
/// The CPU writes to this structure in order to initialize the fields other
/// than [`Self::early_instance_count`] and [`Self::late_instance_count`]. The
/// GPU mesh preprocessing shader increments the [`Self::early_instance_count`]
/// and [`Self::late_instance_count`] as it determines that meshes are visible.
/// The indirect parameter building shader reads this metadata in order to
/// construct the indirect draw parameters.
///
/// Each batch will have one instance of this structure.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct IndirectParametersMetadata {
    /// The index of the mesh in the array of `MeshInputUniform`s.
    pub mesh_index: u32,

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
    /// [`IndirectParametersMetadata`] structures) can be part of the same batch
    /// set.
    pub batch_set_index: u32,

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
/// the buffers that store [`IndirectParametersMetadata`], which are the
/// structures that culling writes to so that the indirect parameter building
/// pass can determine how many meshes are actually to be drawn.
///
/// These buffers will remain empty if indirect drawing isn't in use.
#[derive(Resource)]
pub struct IndirectParametersBuffers {
    /// The GPU buffer that stores the indirect draw parameters for non-indexed
    /// meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, while the
    /// `multi_draw_indirect` or `multi_draw_indirect_count` commands read from
    /// it to perform the draws.
    non_indexed_data: UninitBufferVec<IndirectParametersNonIndexed>,

    /// The GPU buffer that holds the data used to construct indirect draw
    /// parameters for non-indexed meshes.
    ///
    /// The GPU mesh preprocessing shader writes to this buffer, and the
    /// indirect parameters building shader reads this buffer to construct the
    /// indirect draw parameters.
    non_indexed_metadata: RawBufferVec<IndirectParametersMetadata>,

    /// The GPU buffer that holds the number of indirect draw commands for each
    /// phase of each view, for non-indexed meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, and the
    /// `multi_draw_indirect_count` command reads from it in order to know how
    /// many indirect draw commands to process.
    non_indexed_batch_sets: RawBufferVec<IndirectBatchSet>,

    /// The GPU buffer that stores the indirect draw parameters for indexed
    /// meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, while the
    /// `multi_draw_indirect` or `multi_draw_indirect_count` commands read from
    /// it to perform the draws.
    indexed_data: UninitBufferVec<IndirectParametersIndexed>,

    /// The GPU buffer that holds the data used to construct indirect draw
    /// parameters for indexed meshes.
    ///
    /// The GPU mesh preprocessing shader writes to this buffer, and the
    /// indirect parameters building shader reads this buffer to construct the
    /// indirect draw parameters.
    indexed_metadata: RawBufferVec<IndirectParametersMetadata>,

    /// The GPU buffer that holds the number of indirect draw commands for each
    /// phase of each view, for indexed meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, and the
    /// `multi_draw_indirect_count` command reads from it in order to know how
    /// many indirect draw commands to process.
    indexed_batch_sets: RawBufferVec<IndirectBatchSet>,
}

impl IndirectParametersBuffers {
    /// Creates the indirect parameters buffers.
    pub fn new(allow_copies_from_indirect_parameter_buffers: bool) -> IndirectParametersBuffers {
        let mut indirect_parameter_buffer_usages = BufferUsages::STORAGE | BufferUsages::INDIRECT;
        if allow_copies_from_indirect_parameter_buffers {
            indirect_parameter_buffer_usages |= BufferUsages::COPY_SRC;
        }

        IndirectParametersBuffers {
            non_indexed_data: UninitBufferVec::new(indirect_parameter_buffer_usages),
            non_indexed_metadata: RawBufferVec::new(BufferUsages::STORAGE),
            non_indexed_batch_sets: RawBufferVec::new(indirect_parameter_buffer_usages),
            indexed_data: UninitBufferVec::new(indirect_parameter_buffer_usages),
            indexed_metadata: RawBufferVec::new(BufferUsages::STORAGE),
            indexed_batch_sets: RawBufferVec::new(indirect_parameter_buffer_usages),
        }
    }

    /// Returns the GPU buffer that stores the indirect draw parameters for
    /// indexed meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, while the
    /// `multi_draw_indirect` or `multi_draw_indirect_count` commands read from
    /// it to perform the draws.
    #[inline]
    pub fn indexed_data_buffer(&self) -> Option<&Buffer> {
        self.indexed_data.buffer()
    }

    /// Returns the GPU buffer that holds the data used to construct indirect
    /// draw parameters for indexed meshes.
    ///
    /// The GPU mesh preprocessing shader writes to this buffer, and the
    /// indirect parameters building shader reads this buffer to construct the
    /// indirect draw parameters.
    #[inline]
    pub fn indexed_metadata_buffer(&self) -> Option<&Buffer> {
        self.indexed_metadata.buffer()
    }

    /// Returns the GPU buffer that holds the number of indirect draw commands
    /// for each phase of each view, for indexed meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, and the
    /// `multi_draw_indirect_count` command reads from it in order to know how
    /// many indirect draw commands to process.
    #[inline]
    pub fn indexed_batch_sets_buffer(&self) -> Option<&Buffer> {
        self.indexed_batch_sets.buffer()
    }

    /// Returns the GPU buffer that stores the indirect draw parameters for
    /// non-indexed meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, while the
    /// `multi_draw_indirect` or `multi_draw_indirect_count` commands read from
    /// it to perform the draws.
    #[inline]
    pub fn non_indexed_data_buffer(&self) -> Option<&Buffer> {
        self.non_indexed_data.buffer()
    }

    /// Returns the GPU buffer that holds the data used to construct indirect
    /// draw parameters for non-indexed meshes.
    ///
    /// The GPU mesh preprocessing shader writes to this buffer, and the
    /// indirect parameters building shader reads this buffer to construct the
    /// indirect draw parameters.
    #[inline]
    pub fn non_indexed_metadata_buffer(&self) -> Option<&Buffer> {
        self.non_indexed_metadata.buffer()
    }

    /// Returns the GPU buffer that holds the number of indirect draw commands
    /// for each phase of each view, for non-indexed meshes.
    ///
    /// The indirect parameters building shader writes to this buffer, and the
    /// `multi_draw_indirect_count` command reads from it in order to know how
    /// many indirect draw commands to process.
    #[inline]
    pub fn non_indexed_batch_sets_buffer(&self) -> Option<&Buffer> {
        self.non_indexed_batch_sets.buffer()
    }

    /// Reserves space for `count` new batches corresponding to indexed meshes.
    ///
    /// This allocates in both the [`Self::indexed_metadata`] and
    /// [`Self::indexed_data`] buffers.
    fn allocate_indexed(&mut self, count: u32) -> u32 {
        let length = self.indexed_data.len();
        self.indexed_metadata.reserve_internal(count as usize);
        for _ in 0..count {
            self.indexed_data.add();
            self.indexed_metadata
                .push(IndirectParametersMetadata::default());
        }
        length as u32
    }

    /// Reserves space for `count` new batches corresponding to non-indexed
    /// meshes.
    ///
    /// This allocates in both the `non_indexed_metadata` and `non_indexed_data`
    /// buffers.
    pub fn allocate_non_indexed(&mut self, count: u32) -> u32 {
        let length = self.non_indexed_data.len();
        self.non_indexed_metadata.reserve_internal(count as usize);
        for _ in 0..count {
            self.non_indexed_data.add();
            self.non_indexed_metadata
                .push(IndirectParametersMetadata::default());
        }
        length as u32
    }

    /// Reserves space for `count` new batches.
    ///
    /// The `indexed` parameter specifies whether the meshes that these batches
    /// correspond to are indexed or not.
    pub fn allocate(&mut self, indexed: bool, count: u32) -> u32 {
        if indexed {
            self.allocate_indexed(count)
        } else {
            self.allocate_non_indexed(count)
        }
    }

    /// Initializes the batch corresponding to an indexed mesh at the given
    /// index with the given [`IndirectParametersMetadata`].
    pub fn set_indexed(&mut self, index: u32, value: IndirectParametersMetadata) {
        self.indexed_metadata.set(index, value);
    }

    /// Initializes the batch corresponding to a non-indexed mesh at the given
    /// index with the given [`IndirectParametersMetadata`].
    pub fn set_non_indexed(&mut self, index: u32, value: IndirectParametersMetadata) {
        self.non_indexed_metadata.set(index, value);
    }

    /// Returns the number of batches currently allocated.
    ///
    /// The `indexed` parameter specifies whether the meshes that these batches
    /// correspond to are indexed or not.
    fn batch_count(&self, indexed: bool) -> usize {
        if indexed {
            self.indexed_batch_count()
        } else {
            self.non_indexed_batch_count()
        }
    }

    /// Returns the number of batches corresponding to indexed meshes that are
    /// currently allocated.
    #[inline]
    pub fn indexed_batch_count(&self) -> usize {
        self.indexed_data.len()
    }

    /// Returns the number of batches corresponding to non-indexed meshes that
    /// are currently allocated.
    #[inline]
    pub fn non_indexed_batch_count(&self) -> usize {
        self.non_indexed_data.len()
    }

    /// Returns the number of batch sets currently allocated.
    ///
    /// The `indexed` parameter specifies whether the meshes that these batch
    /// sets correspond to are indexed or not.
    pub fn batch_set_count(&self, indexed: bool) -> usize {
        if indexed {
            self.indexed_batch_sets.len()
        } else {
            self.non_indexed_batch_sets.len()
        }
    }

    /// Adds a new batch set to `Self::indexed_batch_sets` or
    /// `Self::non_indexed_batch_sets` as appropriate.
    ///
    /// `indexed` specifies whether the meshes that these batch sets correspond
    /// to are indexed or not. `indirect_parameters_base` specifies the offset
    /// within `Self::indexed_data` or `Self::non_indexed_data` of the first
    /// batch in this batch set.
    pub fn add_batch_set(&mut self, indexed: bool, indirect_parameters_base: u32) {
        if indexed {
            self.indexed_batch_sets.push(IndirectBatchSet {
                indirect_parameters_base,
                indirect_parameters_count: 0,
            });
        } else {
            self.non_indexed_batch_sets.push(IndirectBatchSet {
                indirect_parameters_base,
                indirect_parameters_count: 0,
            });
        }
    }

    pub fn get_next_batch_set_index(&self, indexed: bool) -> Option<NonMaxU32> {
        NonMaxU32::new(self.batch_set_count(indexed) as u32)
    }
}

impl Default for IndirectParametersBuffers {
    fn default() -> Self {
        // By default, we don't allow GPU indirect parameter mapping, since
        // that's a debugging option.
        Self::new(false)
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
    BDI: Pod + Sync + Send + Default + 'static,
{
    /// Creates new buffers.
    pub fn new() -> Self {
        BatchedInstanceBuffers {
            data_buffer: UninitBufferVec::new(BufferUsages::STORAGE),
            work_item_buffers: EntityHashMap::default(),
            current_input_buffer: InstanceInputUniformBuffer::new(),
            previous_input_buffer: InstanceInputUniformBuffer::new(),
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
    pub fn instance_data_binding(&self) -> Option<BindingResource> {
        self.data_buffer
            .buffer()
            .map(|buffer| buffer.as_entire_binding())
    }

    /// Clears out the buffers in preparation for a new frame.
    pub fn clear(&mut self) {
        self.data_buffer.clear();
        self.late_indexed_indirect_parameters_buffer.clear();
        self.late_non_indexed_indirect_parameters_buffer.clear();
        self.work_item_buffers.clear();
    }
}

impl<BD, BDI> Default for BatchedInstanceBuffers<BD, BDI>
where
    BD: GpuArrayBufferable + Sync + Send + 'static,
    BDI: Pod + Default + Sync + Send + 'static,
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

    /// True if the mesh in question has an index buffer; false otherwise.
    indexed: bool,

    /// The index of the indirect parameters for this batch in the
    /// [`IndirectParametersBuffers`].
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
    fn flush<I>(
        self,
        instance_end_index: u32,
        phase: &mut SortedRenderPhase<I>,
        indirect_parameters_buffers: &mut IndirectParametersBuffers,
    ) where
        I: CachedRenderPipelinePhaseItem + SortedPhaseItem,
    {
        let (batch_range, batch_extra_index) =
            phase.items[self.phase_item_start_index as usize].batch_range_and_extra_index_mut();
        *batch_range = self.instance_start_index..instance_end_index;
        *batch_extra_index = match self.indirect_parameters_index {
            Some(indirect_parameters_index) => PhaseItemExtraIndex::IndirectParametersIndex {
                range: u32::from(indirect_parameters_index)
                    ..(u32::from(indirect_parameters_index) + 1),
                batch_set_index: None,
            },
            None => PhaseItemExtraIndex::None,
        };
        if let Some(indirect_parameters_index) = self.indirect_parameters_index {
            indirect_parameters_buffers
                .add_batch_set(self.indexed, indirect_parameters_index.into());
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
    mut indirect_parameters_buffers: ResMut<IndirectParametersBuffers>,
    mut sorted_render_phases: ResMut<ViewSortedRenderPhases<I>>,
    mut views: Query<(
        Entity,
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
    let BatchedInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ref mut late_indexed_indirect_parameters_buffer,
        ref mut late_non_indexed_indirect_parameters_buffer,
        ..
    } = gpu_array_buffer.into_inner();

    for (view, extracted_view, no_indirect_drawing, gpu_occlusion_culling) in &mut views {
        let Some(phase) = sorted_render_phases.get_mut(&extracted_view.retained_view_entity) else {
            continue;
        };

        // Create the work item buffer if necessary.
        let work_item_buffer = get_or_create_work_item_buffer::<I>(
            work_item_buffers,
            view,
            no_indirect_drawing,
            gpu_occlusion_culling,
            late_indexed_indirect_parameters_buffer,
            late_non_indexed_indirect_parameters_buffer,
        );

        // Walk through the list of phase items, building up batches as we go.
        let mut batch: Option<SortedRenderBatch<GFBD>> = None;

        let mut first_output_index = data_buffer.len() as u32;

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
                if let Some(batch) = batch.take() {
                    batch.flush(
                        data_buffer.len() as u32,
                        phase,
                        &mut indirect_parameters_buffers,
                    );
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
            let output_index = data_buffer.add() as u32;

            // If we can't batch, break the existing batch and make a new one.
            if !can_batch {
                // Break a batch if we need to.
                if let Some(batch) = batch.take() {
                    batch.flush(output_index, phase, &mut indirect_parameters_buffers);
                }

                let indirect_parameters_index = if no_indirect_drawing {
                    None
                } else if item_is_indexed {
                    Some(indirect_parameters_buffers.allocate_indexed(1))
                } else {
                    Some(indirect_parameters_buffers.allocate_non_indexed(1))
                };

                // Start a new batch.
                if let Some(indirect_parameters_index) = indirect_parameters_index {
                    GFBD::write_batch_indirect_parameters_metadata(
                        current_input_index.into(),
                        item_is_indexed,
                        output_index,
                        None,
                        &mut indirect_parameters_buffers,
                        indirect_parameters_index,
                    );
                };

                batch = Some(SortedRenderBatch {
                    phase_item_start_index: current_index as u32,
                    instance_start_index: output_index,
                    indexed: item_is_indexed,
                    indirect_parameters_index: indirect_parameters_index.and_then(NonMaxU32::new),
                    meta: current_meta,
                });

                first_output_index = output_index;
            }

            // Add a new preprocessing work item so that the preprocessing
            // shader will copy the per-instance data over.
            if let Some(batch) = batch.as_ref() {
                work_item_buffer.push(
                    item_is_indexed,
                    PreprocessWorkItem {
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
                    },
                );
            }
        }

        // Flush the final batch if necessary.
        if let Some(batch) = batch.take() {
            batch.flush(
                data_buffer.len() as u32,
                phase,
                &mut indirect_parameters_buffers,
            );
        }
    }
}

/// Creates batches for a render phase that uses bins.
pub fn batch_and_prepare_binned_render_phase<BPI, GFBD>(
    gpu_array_buffer: ResMut<BatchedInstanceBuffers<GFBD::BufferData, GFBD::BufferInputData>>,
    mut indirect_parameters_buffers: ResMut<IndirectParametersBuffers>,
    mut binned_render_phases: ResMut<ViewBinnedRenderPhases<BPI>>,
    mut views: Query<
        (
            Entity,
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

    let BatchedInstanceBuffers {
        ref mut data_buffer,
        ref mut work_item_buffers,
        ref mut late_indexed_indirect_parameters_buffer,
        ref mut late_non_indexed_indirect_parameters_buffer,
        ..
    } = gpu_array_buffer.into_inner();

    for (view, extracted_view, no_indirect_drawing, gpu_occlusion_culling) in &mut views {
        let Some(phase) = binned_render_phases.get_mut(&extracted_view.retained_view_entity) else {
            continue;
        };

        // Create the work item buffer if necessary; otherwise, just mark it as
        // used this frame.
        let work_item_buffer = get_or_create_work_item_buffer::<BPI>(
            work_item_buffers,
            view,
            no_indirect_drawing,
            gpu_occlusion_culling,
            late_indexed_indirect_parameters_buffer,
            late_non_indexed_indirect_parameters_buffer,
        );

        // Prepare multidrawables.

        for batch_set_key in &phase.multidrawable_mesh_keys {
            let mut batch_set = None;
            let indirect_parameters_base =
                indirect_parameters_buffers.batch_count(batch_set_key.indexed()) as u32;
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
                            work_item_buffer.push(
                                batch_set_key.indexed(),
                                PreprocessWorkItem {
                                    input_index: input_index.into(),
                                    output_index: first_output_index,
                                    indirect_parameters_index: match batch.extra_index {
                                        PhaseItemExtraIndex::IndirectParametersIndex {
                                            ref range,
                                            ..
                                        } => range.start,
                                        PhaseItemExtraIndex::DynamicOffset(_)
                                        | PhaseItemExtraIndex::None => 0,
                                    },
                                },
                            );
                        }

                        None => {
                            // Start a new batch, in indirect mode.
                            let indirect_parameters_index =
                                indirect_parameters_buffers.allocate(batch_set_key.indexed(), 1);
                            let batch_set_index = indirect_parameters_buffers
                                .get_next_batch_set_index(batch_set_key.indexed());

                            GFBD::write_batch_indirect_parameters_metadata(
                                input_index.into(),
                                batch_set_key.indexed(),
                                output_index,
                                batch_set_index,
                                &mut indirect_parameters_buffers,
                                indirect_parameters_index,
                            );
                            work_item_buffer.push(
                                batch_set_key.indexed(),
                                PreprocessWorkItem {
                                    input_index: input_index.into(),
                                    output_index: first_output_index,
                                    indirect_parameters_index,
                                },
                            );
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
                                index: indirect_parameters_buffers
                                    .batch_set_count(batch_set_key.indexed())
                                    as u32,
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
                    indirect_parameters_buffers
                        .add_batch_set(batch_set_key.indexed(), indirect_parameters_base);
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
                        work_item_buffer.push(
                            key.0.indexed(),
                            PreprocessWorkItem {
                                input_index: input_index.into(),
                                output_index: if no_indirect_drawing {
                                    output_index
                                } else {
                                    first_output_index
                                },
                                indirect_parameters_index: match batch.extra_index {
                                    PhaseItemExtraIndex::IndirectParametersIndex {
                                        range: ref indirect_parameters_range,
                                        ..
                                    } => indirect_parameters_range.start,
                                    PhaseItemExtraIndex::DynamicOffset(_)
                                    | PhaseItemExtraIndex::None => 0,
                                },
                            },
                        );
                    }

                    None if !no_indirect_drawing => {
                        // Start a new batch, in indirect mode.
                        let indirect_parameters_index =
                            indirect_parameters_buffers.allocate(key.0.indexed(), 1);
                        let batch_set_index =
                            indirect_parameters_buffers.get_next_batch_set_index(key.0.indexed());

                        GFBD::write_batch_indirect_parameters_metadata(
                            input_index.into(),
                            key.0.indexed(),
                            output_index,
                            batch_set_index,
                            &mut indirect_parameters_buffers,
                            indirect_parameters_index,
                        );
                        work_item_buffer.push(
                            key.0.indexed(),
                            PreprocessWorkItem {
                                input_index: input_index.into(),
                                output_index: first_output_index,
                                indirect_parameters_index,
                            },
                        );
                        batch = Some(BinnedRenderPhaseBatch {
                            representative_entity: (entity, main_entity),
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
                                input_index: input_index.into(),
                                output_index,
                                indirect_parameters_index: 0,
                            },
                        );
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
                            index: indirect_parameters_buffers.batch_set_count(key.0.indexed())
                                as u32,
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
            } else if key.0.indexed() {
                Some(
                    indirect_parameters_buffers
                        .allocate_indexed(unbatchables.entities.len() as u32),
                )
            } else {
                Some(
                    indirect_parameters_buffers
                        .allocate_non_indexed(unbatchables.entities.len() as u32),
                )
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
                    GFBD::write_batch_indirect_parameters_metadata(
                        input_index.into(),
                        key.0.indexed(),
                        output_index,
                        None,
                        &mut indirect_parameters_buffers,
                        *indirect_parameters_index,
                    );
                    work_item_buffer.push(
                        key.0.indexed(),
                        PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index,
                            indirect_parameters_index: *indirect_parameters_index,
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
                    indirect_parameters_buffers
                        .add_batch_set(key.0.indexed(), *indirect_parameters_index);
                    *indirect_parameters_index += 1;
                } else {
                    work_item_buffer.push(
                        key.0.indexed(),
                        PreprocessWorkItem {
                            input_index: input_index.into(),
                            output_index,
                            indirect_parameters_index: 0,
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
        ref mut work_item_buffers,
        ref mut current_input_buffer,
        ref mut previous_input_buffer,
        ref mut late_indexed_indirect_parameters_buffer,
        ref mut late_non_indexed_indirect_parameters_buffer,
    } = gpu_array_buffer.into_inner();

    data_buffer.write_buffer(&render_device);
    current_input_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
    previous_input_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
    late_indexed_indirect_parameters_buffer.write_buffer(&render_device, &render_queue);
    late_non_indexed_indirect_parameters_buffer.write_buffer(&render_device, &render_queue);

    for view_work_item_buffers in work_item_buffers.values_mut() {
        for phase_work_item_buffers in view_work_item_buffers.values_mut() {
            match *phase_work_item_buffers {
                PreprocessWorkItemBuffers::Direct(ref mut buffer_vec) => {
                    buffer_vec.write_buffer(&render_device, &render_queue);
                }
                PreprocessWorkItemBuffers::Indirect {
                    ref mut indexed,
                    ref mut non_indexed,
                    ref mut gpu_occlusion_culling,
                } => {
                    indexed.write_buffer(&render_device, &render_queue);
                    non_indexed.write_buffer(&render_device, &render_queue);

                    if let Some(GpuOcclusionCullingWorkItemBuffers {
                        ref mut late_indexed,
                        ref mut late_non_indexed,
                        late_indirect_parameters_indexed_offset: _,
                        late_indirect_parameters_non_indexed_offset: _,
                    }) = *gpu_occlusion_culling
                    {
                        if !late_indexed.is_empty() {
                            late_indexed.write_buffer(&render_device);
                        }
                        if !late_non_indexed.is_empty() {
                            late_non_indexed.write_buffer(&render_device);
                        }
                    }
                }
            }
        }
    }
}

pub fn clear_indirect_parameters_buffers(
    mut indirect_parameters_buffers: ResMut<IndirectParametersBuffers>,
) {
    indirect_parameters_buffers.indexed_data.clear();
    indirect_parameters_buffers.indexed_metadata.clear();
    indirect_parameters_buffers.indexed_batch_sets.clear();
    indirect_parameters_buffers.non_indexed_data.clear();
    indirect_parameters_buffers.non_indexed_metadata.clear();
    indirect_parameters_buffers.non_indexed_batch_sets.clear();
}

pub fn write_indirect_parameters_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut indirect_parameters_buffers: ResMut<IndirectParametersBuffers>,
) {
    indirect_parameters_buffers
        .indexed_data
        .write_buffer(&render_device);
    indirect_parameters_buffers
        .non_indexed_data
        .write_buffer(&render_device);

    indirect_parameters_buffers
        .indexed_metadata
        .write_buffer(&render_device, &render_queue);
    indirect_parameters_buffers
        .non_indexed_metadata
        .write_buffer(&render_device, &render_queue);

    indirect_parameters_buffers
        .indexed_batch_sets
        .write_buffer(&render_device, &render_queue);
    indirect_parameters_buffers
        .non_indexed_batch_sets
        .write_buffer(&render_device, &render_queue);
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
