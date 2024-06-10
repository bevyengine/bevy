use std::mem;

use bevy_asset::{load_internal_asset, AssetId};
use bevy_core_pipeline::{
    core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d, CORE_3D_DEPTH_FORMAT},
    deferred::{AlphaMask3dDeferred, Opaque3dDeferred},
    prepass::MotionVectorPrepass,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{
    prelude::*,
    query::ROQueryItem,
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_math::{Affine3, Rect, UVec2, Vec3, Vec4};
use bevy_render::{
    batching::{
        gpu_preprocessing::{
            self, GpuPreprocessingSupport, IndirectParameters, IndirectParametersBuffer,
        },
        no_gpu_preprocessing, GetBatchData, GetFullBatchData, NoAutomaticBatching,
    },
    camera::Camera,
    mesh::*,
    primitives::Aabb,
    render_asset::RenderAssets,
    render_phase::{
        BinnedRenderPhasePlugin, PhaseItem, RenderCommand, RenderCommandResult,
        SortedRenderPhasePlugin, TrackedRenderPass,
    },
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, DefaultImageSampler, ImageSampler, TextureFormatPixelInfo},
    view::{
        prepare_view_targets, GpuCulling, RenderVisibilityRanges, ViewTarget, ViewUniformOffset,
        ViewVisibility, VisibilityRange,
    },
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{tracing::error, tracing::warn, Entry, HashMap, Parallel};

use bytemuck::{Pod, Zeroable};
use nonmax::{NonMaxU16, NonMaxU32};
use static_assertions::const_assert_eq;

use crate::render::{
    morph::{
        extract_morphs, no_automatic_morph_batching, prepare_morphs, MorphIndices, MorphUniforms,
    },
    skin::no_automatic_skin_batching,
};
use crate::*;

use self::irradiance_volume::IRRADIANCE_VOLUMES_ARE_USABLE;

/// Provides support for rendering 3D meshes.
#[derive(Default)]
pub struct MeshRenderPlugin {
    /// Whether we're building [`MeshUniform`]s on GPU.
    ///
    /// This requires compute shader support and so will be forcibly disabled if
    /// the platform doesn't support those.
    pub use_gpu_instance_buffer_builder: bool,
}

pub const FORWARD_IO_HANDLE: Handle<Shader> = Handle::weak_from_u128(2645551199423808407);
pub const MESH_VIEW_TYPES_HANDLE: Handle<Shader> = Handle::weak_from_u128(8140454348013264787);
pub const MESH_VIEW_BINDINGS_HANDLE: Handle<Shader> = Handle::weak_from_u128(9076678235888822571);
pub const MESH_TYPES_HANDLE: Handle<Shader> = Handle::weak_from_u128(2506024101911992377);
pub const MESH_BINDINGS_HANDLE: Handle<Shader> = Handle::weak_from_u128(16831548636314682308);
pub const MESH_FUNCTIONS_HANDLE: Handle<Shader> = Handle::weak_from_u128(6300874327833745635);
pub const MESH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(3252377289100772450);
pub const SKINNING_HANDLE: Handle<Shader> = Handle::weak_from_u128(13215291596265391738);
pub const MORPH_HANDLE: Handle<Shader> = Handle::weak_from_u128(970982813587607345);

/// How many textures are allowed in the view bind group layout (`@group(0)`) before
/// broader compatibility with WebGL and WebGPU is at risk, due to the minimum guaranteed
/// values for `MAX_TEXTURE_IMAGE_UNITS` (in WebGL) and `maxSampledTexturesPerShaderStage` (in WebGPU),
/// currently both at 16.
///
/// We use 10 here because it still leaves us, in a worst case scenario, with 6 textures for the other bind groups.
///
/// See: <https://gpuweb.github.io/gpuweb/#limits>
#[cfg(debug_assertions)]
pub const MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES: usize = 10;

impl Plugin for MeshRenderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, FORWARD_IO_HANDLE, "forward_io.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            MESH_VIEW_TYPES_HANDLE,
            "mesh_view_types.wgsl",
            Shader::from_wgsl_with_defs,
            vec![
                ShaderDefVal::UInt(
                    "MAX_DIRECTIONAL_LIGHTS".into(),
                    MAX_DIRECTIONAL_LIGHTS as u32
                ),
                ShaderDefVal::UInt(
                    "MAX_CASCADES_PER_LIGHT".into(),
                    MAX_CASCADES_PER_LIGHT as u32,
                )
            ]
        );
        load_internal_asset!(
            app,
            MESH_VIEW_BINDINGS_HANDLE,
            "mesh_view_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, MESH_TYPES_HANDLE, "mesh_types.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            MESH_FUNCTIONS_HANDLE,
            "mesh_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, MESH_SHADER_HANDLE, "mesh.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, SKINNING_HANDLE, "skinning.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, MORPH_HANDLE, "morph.wgsl", Shader::from_wgsl);

        app.add_systems(
            PostUpdate,
            (no_automatic_skin_batching, no_automatic_morph_batching),
        )
        .add_plugins((
            BinnedRenderPhasePlugin::<Opaque3d, MeshPipeline>::default(),
            BinnedRenderPhasePlugin::<AlphaMask3d, MeshPipeline>::default(),
            BinnedRenderPhasePlugin::<Shadow, MeshPipeline>::default(),
            BinnedRenderPhasePlugin::<Opaque3dDeferred, MeshPipeline>::default(),
            BinnedRenderPhasePlugin::<AlphaMask3dDeferred, MeshPipeline>::default(),
            SortedRenderPhasePlugin::<Transmissive3d, MeshPipeline>::default(),
            SortedRenderPhasePlugin::<Transparent3d, MeshPipeline>::default(),
        ));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<MeshBindGroups>()
                .init_resource::<SkinUniforms>()
                .init_resource::<SkinIndices>()
                .init_resource::<MorphUniforms>()
                .init_resource::<MorphIndices>()
                .init_resource::<MeshCullingDataBuffer>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_skins,
                        extract_morphs,
                        gpu_preprocessing::clear_batched_gpu_instance_buffers::<MeshPipeline>
                            .before(ExtractMeshesSet),
                    ),
                )
                .add_systems(
                    Render,
                    (
                        set_mesh_motion_vector_flags.before(RenderSet::Queue),
                        prepare_skins.in_set(RenderSet::PrepareResources),
                        prepare_morphs.in_set(RenderSet::PrepareResources),
                        prepare_mesh_bind_group.in_set(RenderSet::PrepareBindGroups),
                        prepare_mesh_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                        no_gpu_preprocessing::clear_batched_cpu_instance_buffers::<MeshPipeline>
                            .in_set(RenderSet::Cleanup)
                            .after(RenderSet::Render),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        let mut mesh_bindings_shader_defs = Vec::with_capacity(1);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<GpuPreprocessingSupport>();

            let gpu_preprocessing_support =
                render_app.world().resource::<GpuPreprocessingSupport>();
            let use_gpu_instance_buffer_builder = self.use_gpu_instance_buffer_builder
                && *gpu_preprocessing_support != GpuPreprocessingSupport::None;

            let render_mesh_instances = RenderMeshInstances::new(use_gpu_instance_buffer_builder);
            render_app.insert_resource(render_mesh_instances);

            if use_gpu_instance_buffer_builder {
                render_app
                    .init_resource::<gpu_preprocessing::BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>(
                    )
                    .add_systems(
                        ExtractSchedule,
                        extract_meshes_for_gpu_building.in_set(ExtractMeshesSet),
                    )
                    .add_systems(
                        Render,
                        (
                            gpu_preprocessing::write_batched_instance_buffers::<MeshPipeline>
                                .in_set(RenderSet::PrepareResourcesFlush),
                            gpu_preprocessing::delete_old_work_item_buffers::<MeshPipeline>
                                .in_set(RenderSet::ManageViews)
                                .after(prepare_view_targets),
                        ),
                    );
            } else {
                let render_device = render_app.world().resource::<RenderDevice>();
                let cpu_batched_instance_buffer =
                    no_gpu_preprocessing::BatchedInstanceBuffer::<MeshUniform>::new(render_device);
                render_app
                    .insert_resource(cpu_batched_instance_buffer)
                    .add_systems(
                        ExtractSchedule,
                        extract_meshes_for_cpu_building.in_set(ExtractMeshesSet),
                    )
                    .add_systems(
                        Render,
                        no_gpu_preprocessing::write_batched_instance_buffer::<MeshPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                    );
            };

            let indirect_parameters_buffer = IndirectParametersBuffer::new();

            let render_device = render_app.world().resource::<RenderDevice>();
            if let Some(per_object_buffer_batch_size) =
                GpuArrayBuffer::<MeshUniform>::batch_size(render_device)
            {
                mesh_bindings_shader_defs.push(ShaderDefVal::UInt(
                    "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
                    per_object_buffer_batch_size,
                ));
            }

            render_app
                .insert_resource(indirect_parameters_buffer)
                .init_resource::<MeshPipelineViewLayouts>()
                .init_resource::<MeshPipeline>();
        }

        // Load the mesh_bindings shader module here as it depends on runtime information about
        // whether storage buffers are supported, or the maximum uniform buffer binding size.
        load_internal_asset!(
            app,
            MESH_BINDINGS_HANDLE,
            "mesh_bindings.wgsl",
            Shader::from_wgsl_with_defs,
            mesh_bindings_shader_defs
        );
    }
}

#[derive(Component)]
pub struct MeshTransforms {
    pub world_from_local: Affine3,
    pub previous_world_from_local: Affine3,
    pub flags: u32,
}

#[derive(ShaderType, Clone)]
pub struct MeshUniform {
    // Affine 4x3 matrices transposed to 3x4
    pub world_from_local: [Vec4; 3],
    pub previous_world_from_local: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub local_from_world_transpose_a: [Vec4; 2],
    pub local_from_world_transpose_b: f32,
    pub flags: u32,
    // Four 16-bit unsigned normalized UV values packed into a `UVec2`:
    //
    //                         <--- MSB                   LSB --->
    //                         +---- min v ----+ +---- min u ----+
    //     lightmap_uv_rect.x: vvvvvvvv vvvvvvvv uuuuuuuu uuuuuuuu,
    //                         +---- max v ----+ +---- max u ----+
    //     lightmap_uv_rect.y: VVVVVVVV VVVVVVVV UUUUUUUU UUUUUUUU,
    //
    // (MSB: most significant bit; LSB: least significant bit.)
    pub lightmap_uv_rect: UVec2,
}

/// Information that has to be transferred from CPU to GPU in order to produce
/// the full [`MeshUniform`].
///
/// This is essentially a subset of the fields in [`MeshUniform`] above.
#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct MeshInputUniform {
    /// Affine 4x3 matrix transposed to 3x4.
    pub world_from_local: [Vec4; 3],
    /// Four 16-bit unsigned normalized UV values packed into a `UVec2`:
    ///
    /// ```text
    ///                         <--- MSB                   LSB --->
    ///                         +---- min v ----+ +---- min u ----+
    ///     lightmap_uv_rect.x: vvvvvvvv vvvvvvvv uuuuuuuu uuuuuuuu,
    ///                         +---- max v ----+ +---- max u ----+
    ///     lightmap_uv_rect.y: VVVVVVVV VVVVVVVV UUUUUUUU UUUUUUUU,
    ///
    /// (MSB: most significant bit; LSB: least significant bit.)
    /// ```
    pub lightmap_uv_rect: UVec2,
    /// Various [`MeshFlags`].
    pub flags: u32,
    /// The index of this mesh's [`MeshInputUniform`] in the previous frame's
    /// buffer, if applicable.
    ///
    /// This is used for TAA. If not present, this will be `u32::MAX`.
    pub previous_input_index: u32,
}

/// Information about each mesh instance needed to cull it on GPU.
///
/// This consists of its axis-aligned bounding box (AABB).
#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct MeshCullingData {
    /// The 3D center of the AABB in model space, padded with an extra unused
    /// float value.
    pub aabb_center: Vec4,
    /// The 3D extents of the AABB in model space, divided by two, padded with
    /// an extra unused float value.
    pub aabb_half_extents: Vec4,
}

/// A GPU buffer that holds the information needed to cull meshes on GPU.
///
/// At the moment, this simply holds each mesh's AABB.
///
/// To avoid wasting CPU time in the CPU culling case, this buffer will be empty
/// if GPU culling isn't in use.
#[derive(Resource, Deref, DerefMut)]
pub struct MeshCullingDataBuffer(RawBufferVec<MeshCullingData>);

impl MeshUniform {
    pub fn new(mesh_transforms: &MeshTransforms, maybe_lightmap_uv_rect: Option<Rect>) -> Self {
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            mesh_transforms.world_from_local.inverse_transpose_3x3();
        Self {
            world_from_local: mesh_transforms.world_from_local.to_transpose(),
            previous_world_from_local: mesh_transforms.previous_world_from_local.to_transpose(),
            lightmap_uv_rect: pack_lightmap_uv_rect(maybe_lightmap_uv_rect),
            local_from_world_transpose_a,
            local_from_world_transpose_b,
            flags: mesh_transforms.flags,
        }
    }
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_types.wgsl!
bitflags::bitflags! {
    /// Various flags and tightly-packed values on a mesh.
    ///
    /// Flags grow from the top bit down; other values grow from the bottom bit
    /// up.
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        /// Bitmask for the 16-bit index into the LOD array.
        ///
        /// This will be `u16::MAX` if this mesh has no LOD.
        const LOD_INDEX_MASK              = (1 << 16) - 1;
        const SHADOW_RECEIVER             = 1 << 29;
        const TRANSMITTED_SHADOW_RECEIVER = 1 << 30;
        // Indicates the sign of the determinant of the 3x3 model matrix. If the sign is positive,
        // then the flag should be set, else it should not be set.
        const SIGN_DETERMINANT_MODEL_3X3  = 1 << 31;
        const NONE                        = 0;
        const UNINITIALIZED               = 0xFFFFFFFF;
    }
}

impl MeshFlags {
    fn from_components(
        transform: &GlobalTransform,
        lod_index: Option<NonMaxU16>,
        not_shadow_receiver: bool,
        transmitted_receiver: bool,
    ) -> MeshFlags {
        let mut mesh_flags = if not_shadow_receiver {
            MeshFlags::empty()
        } else {
            MeshFlags::SHADOW_RECEIVER
        };
        if transmitted_receiver {
            mesh_flags |= MeshFlags::TRANSMITTED_SHADOW_RECEIVER;
        }
        if transform.affine().matrix3.determinant().is_sign_positive() {
            mesh_flags |= MeshFlags::SIGN_DETERMINANT_MODEL_3X3;
        }

        let lod_index_bits = match lod_index {
            None => u16::MAX,
            Some(lod_index) => u16::from(lod_index),
        };
        mesh_flags |=
            MeshFlags::from_bits_retain((lod_index_bits as u32) << MeshFlags::LOD_INDEX_SHIFT);

        mesh_flags
    }

    /// The first bit of the LOD index.
    pub const LOD_INDEX_SHIFT: u32 = 0;
}

bitflags::bitflags! {
    /// Various useful flags for [`RenderMeshInstance`]s.
    #[derive(Clone, Copy)]
    pub struct RenderMeshInstanceFlags: u8 {
        /// The mesh casts shadows.
        const SHADOW_CASTER           = 1 << 0;
        /// The mesh can participate in automatic batching.
        const AUTOMATIC_BATCHING      = 1 << 1;
        /// The mesh had a transform last frame and so is eligible for motion
        /// vector computation.
        const HAS_PREVIOUS_TRANSFORM  = 1 << 2;
        /// The mesh had a skin last frame and so that skin should be taken into
        /// account for motion vector computation.
        const HAS_PREVIOUS_SKIN       = 1 << 3;
        /// The mesh had morph targets last frame and so they should be taken
        /// into account for motion vector computation.
        const HAS_PREVIOUS_MORPH      = 1 << 4;
    }
}

/// CPU data that the render world keeps for each entity, when *not* using GPU
/// mesh uniform building.
#[derive(Deref, DerefMut)]
pub struct RenderMeshInstanceCpu {
    /// Data shared between both the CPU mesh uniform building and the GPU mesh
    /// uniform building paths.
    #[deref]
    pub shared: RenderMeshInstanceShared,
    /// The transform of the mesh.
    ///
    /// This will be written into the [`MeshUniform`] at the appropriate time.
    pub transforms: MeshTransforms,
}

/// CPU data that the render world needs to keep for each entity that contains a
/// mesh when using GPU mesh uniform building.
#[derive(Deref, DerefMut)]
pub struct RenderMeshInstanceGpu {
    /// Data shared between both the CPU mesh uniform building and the GPU mesh
    /// uniform building paths.
    #[deref]
    pub shared: RenderMeshInstanceShared,
    /// The translation of the mesh.
    ///
    /// This is the only part of the transform that we have to keep on CPU (for
    /// distance sorting).
    pub translation: Vec3,
    /// The index of the [`MeshInputUniform`] in the buffer.
    pub current_uniform_index: NonMaxU32,
}

/// CPU data that the render world needs to keep about each entity that contains
/// a mesh.
pub struct RenderMeshInstanceShared {
    /// The [`AssetId`] of the mesh.
    pub mesh_asset_id: AssetId<Mesh>,
    /// A slot for the material bind group ID.
    ///
    /// This is filled in during [`crate::material::queue_material_meshes`].
    pub material_bind_group_id: AtomicMaterialBindGroupId,
    /// Various flags.
    pub flags: RenderMeshInstanceFlags,
}

/// Information that is gathered during the parallel portion of mesh extraction
/// when GPU mesh uniform building is enabled.
///
/// From this, the [`MeshInputUniform`] and [`RenderMeshInstanceGpu`] are
/// prepared.
pub struct RenderMeshInstanceGpuBuilder {
    /// Data that will be placed on the [`RenderMeshInstanceGpu`].
    pub shared: RenderMeshInstanceShared,
    /// The current transform.
    pub world_from_local: Affine3,
    /// Four 16-bit unsigned normalized UV values packed into a [`UVec2`]:
    ///
    /// ```text
    ///                         <--- MSB                   LSB --->
    ///                         +---- min v ----+ +---- min u ----+
    ///     lightmap_uv_rect.x: vvvvvvvv vvvvvvvv uuuuuuuu uuuuuuuu,
    ///                         +---- max v ----+ +---- max u ----+
    ///     lightmap_uv_rect.y: VVVVVVVV VVVVVVVV UUUUUUUU UUUUUUUU,
    ///
    /// (MSB: most significant bit; LSB: least significant bit.)
    /// ```
    pub lightmap_uv_rect: UVec2,
    /// The index of the previous mesh input.
    pub previous_input_index: Option<NonMaxU32>,
    /// Various flags.
    pub mesh_flags: MeshFlags,
}

/// The per-thread queues used during [`extract_meshes_for_gpu_building`].
///
/// There are two varieties of these: one for when culling happens on CPU and
/// one for when culling happens on GPU. Having the two varieties avoids wasting
/// space if GPU culling is disabled.
#[derive(Default)]
pub enum RenderMeshInstanceGpuQueue {
    /// The default value.
    ///
    /// This becomes [`RenderMeshInstanceGpuQueue::CpuCulling`] or
    /// [`RenderMeshInstanceGpuQueue::GpuCulling`] once extraction starts.
    #[default]
    None,
    /// The version of [`RenderMeshInstanceGpuQueue`] that omits the
    /// [`MeshCullingDataGpuBuilder`], so that we don't waste space when GPU
    /// culling is disabled.
    CpuCulling(Vec<(Entity, RenderMeshInstanceGpuBuilder)>),
    /// The version of [`RenderMeshInstanceGpuQueue`] that contains the
    /// [`MeshCullingDataGpuBuilder`], used when any view has GPU culling
    /// enabled.
    GpuCulling(Vec<(Entity, RenderMeshInstanceGpuBuilder, MeshCullingData)>),
}

impl RenderMeshInstanceShared {
    fn from_components(
        previous_transform: Option<&PreviousGlobalTransform>,
        handle: &Handle<Mesh>,
        not_shadow_caster: bool,
        no_automatic_batching: bool,
    ) -> Self {
        let mut mesh_instance_flags = RenderMeshInstanceFlags::empty();
        mesh_instance_flags.set(RenderMeshInstanceFlags::SHADOW_CASTER, !not_shadow_caster);
        mesh_instance_flags.set(
            RenderMeshInstanceFlags::AUTOMATIC_BATCHING,
            !no_automatic_batching,
        );
        mesh_instance_flags.set(
            RenderMeshInstanceFlags::HAS_PREVIOUS_TRANSFORM,
            previous_transform.is_some(),
        );

        RenderMeshInstanceShared {
            mesh_asset_id: handle.id(),

            flags: mesh_instance_flags,
            material_bind_group_id: AtomicMaterialBindGroupId::default(),
        }
    }

    /// Returns true if this entity is eligible to participate in automatic
    /// batching.
    #[inline]
    pub fn should_batch(&self) -> bool {
        self.flags
            .contains(RenderMeshInstanceFlags::AUTOMATIC_BATCHING)
            && self.material_bind_group_id.get().is_some()
    }
}

/// Information that the render world keeps about each entity that contains a
/// mesh.
///
/// The set of information needed is different depending on whether CPU or GPU
/// [`MeshUniform`] building is in use.
#[derive(Resource)]
pub enum RenderMeshInstances {
    /// Information needed when using CPU mesh instance data building.
    CpuBuilding(RenderMeshInstancesCpu),
    /// Information needed when using GPU mesh instance data building.
    GpuBuilding(RenderMeshInstancesGpu),
}

/// Information that the render world keeps about each entity that contains a
/// mesh, when using CPU mesh instance data building.
#[derive(Default, Deref, DerefMut)]
pub struct RenderMeshInstancesCpu(EntityHashMap<RenderMeshInstanceCpu>);

/// Information that the render world keeps about each entity that contains a
/// mesh, when using GPU mesh instance data building.
#[derive(Default, Deref, DerefMut)]
pub struct RenderMeshInstancesGpu(EntityHashMap<RenderMeshInstanceGpu>);

impl RenderMeshInstances {
    /// Creates a new [`RenderMeshInstances`] instance.
    fn new(use_gpu_instance_buffer_builder: bool) -> RenderMeshInstances {
        if use_gpu_instance_buffer_builder {
            RenderMeshInstances::GpuBuilding(RenderMeshInstancesGpu::default())
        } else {
            RenderMeshInstances::CpuBuilding(RenderMeshInstancesCpu::default())
        }
    }

    /// Returns the ID of the mesh asset attached to the given entity, if any.
    pub(crate) fn mesh_asset_id(&self, entity: Entity) -> Option<AssetId<Mesh>> {
        match *self {
            RenderMeshInstances::CpuBuilding(ref instances) => instances.mesh_asset_id(entity),
            RenderMeshInstances::GpuBuilding(ref instances) => instances.mesh_asset_id(entity),
        }
    }

    /// Constructs [`RenderMeshQueueData`] for the given entity, if it has a
    /// mesh attached.
    pub fn render_mesh_queue_data(&self, entity: Entity) -> Option<RenderMeshQueueData> {
        match *self {
            RenderMeshInstances::CpuBuilding(ref instances) => {
                instances.render_mesh_queue_data(entity)
            }
            RenderMeshInstances::GpuBuilding(ref instances) => {
                instances.render_mesh_queue_data(entity)
            }
        }
    }

    /// Inserts the given flags into the CPU or GPU render mesh instance data
    /// for the given mesh as appropriate.
    fn insert_mesh_instance_flags(&mut self, entity: Entity, flags: RenderMeshInstanceFlags) {
        match *self {
            RenderMeshInstances::CpuBuilding(ref mut instances) => {
                instances.insert_mesh_instance_flags(entity, flags);
            }
            RenderMeshInstances::GpuBuilding(ref mut instances) => {
                instances.insert_mesh_instance_flags(entity, flags);
            }
        }
    }
}

impl RenderMeshInstancesCpu {
    fn mesh_asset_id(&self, entity: Entity) -> Option<AssetId<Mesh>> {
        self.get(&entity)
            .map(|render_mesh_instance| render_mesh_instance.mesh_asset_id)
    }

    fn render_mesh_queue_data(&self, entity: Entity) -> Option<RenderMeshQueueData> {
        self.get(&entity)
            .map(|render_mesh_instance| RenderMeshQueueData {
                shared: &render_mesh_instance.shared,
                translation: render_mesh_instance.transforms.world_from_local.translation,
            })
    }

    /// Inserts the given flags into the render mesh instance data for the given
    /// mesh.
    fn insert_mesh_instance_flags(&mut self, entity: Entity, flags: RenderMeshInstanceFlags) {
        if let Some(instance) = self.get_mut(&entity) {
            instance.flags.insert(flags);
        }
    }
}

impl RenderMeshInstancesGpu {
    fn mesh_asset_id(&self, entity: Entity) -> Option<AssetId<Mesh>> {
        self.get(&entity)
            .map(|render_mesh_instance| render_mesh_instance.mesh_asset_id)
    }

    fn render_mesh_queue_data(&self, entity: Entity) -> Option<RenderMeshQueueData> {
        self.get(&entity)
            .map(|render_mesh_instance| RenderMeshQueueData {
                shared: &render_mesh_instance.shared,
                translation: render_mesh_instance.translation,
            })
    }

    /// Inserts the given flags into the render mesh instance data for the given
    /// mesh.
    fn insert_mesh_instance_flags(&mut self, entity: Entity, flags: RenderMeshInstanceFlags) {
        if let Some(instance) = self.get_mut(&entity) {
            instance.flags.insert(flags);
        }
    }
}

impl RenderMeshInstanceGpuQueue {
    /// Clears out a [`RenderMeshInstanceGpuQueue`], creating or recreating it
    /// as necessary.
    ///
    /// `any_gpu_culling` should be set to true if any view has GPU culling
    /// enabled.
    fn init(&mut self, any_gpu_culling: bool) {
        match (any_gpu_culling, &mut *self) {
            (true, RenderMeshInstanceGpuQueue::GpuCulling(queue)) => queue.clear(),
            (true, _) => *self = RenderMeshInstanceGpuQueue::GpuCulling(vec![]),
            (false, RenderMeshInstanceGpuQueue::CpuCulling(queue)) => queue.clear(),
            (false, _) => *self = RenderMeshInstanceGpuQueue::CpuCulling(vec![]),
        }
    }

    /// Adds a new mesh to this queue.
    fn push(
        &mut self,
        entity: Entity,
        instance_builder: RenderMeshInstanceGpuBuilder,
        culling_data_builder: Option<MeshCullingData>,
    ) {
        match (&mut *self, culling_data_builder) {
            (&mut RenderMeshInstanceGpuQueue::CpuCulling(ref mut queue), None) => {
                queue.push((entity, instance_builder));
            }
            (
                &mut RenderMeshInstanceGpuQueue::GpuCulling(ref mut queue),
                Some(culling_data_builder),
            ) => {
                queue.push((entity, instance_builder, culling_data_builder));
            }
            (_, None) => {
                *self = RenderMeshInstanceGpuQueue::CpuCulling(vec![(entity, instance_builder)]);
            }
            (_, Some(culling_data_builder)) => {
                *self = RenderMeshInstanceGpuQueue::GpuCulling(vec![(
                    entity,
                    instance_builder,
                    culling_data_builder,
                )]);
            }
        }
    }
}

impl RenderMeshInstanceGpuBuilder {
    /// Flushes this mesh instance to the [`RenderMeshInstanceGpu`] and
    /// [`MeshInputUniform`] tables.
    fn add_to(
        self,
        entity: Entity,
        render_mesh_instances: &mut EntityHashMap<RenderMeshInstanceGpu>,
        current_input_buffer: &mut RawBufferVec<MeshInputUniform>,
    ) -> usize {
        // Push the mesh input uniform.
        let current_uniform_index = current_input_buffer.push(MeshInputUniform {
            world_from_local: self.world_from_local.to_transpose(),
            lightmap_uv_rect: self.lightmap_uv_rect,
            flags: self.mesh_flags.bits(),
            previous_input_index: match self.previous_input_index {
                Some(previous_input_index) => previous_input_index.into(),
                None => u32::MAX,
            },
        });

        // Record the [`RenderMeshInstance`].
        render_mesh_instances.insert(
            entity,
            RenderMeshInstanceGpu {
                translation: self.world_from_local.translation,
                shared: self.shared,
                current_uniform_index: (current_uniform_index as u32)
                    .try_into()
                    .unwrap_or_default(),
            },
        );

        current_uniform_index
    }
}

impl MeshCullingData {
    /// Returns a new [`MeshCullingData`] initialized with the given AABB.
    ///
    /// If no AABB is provided, an infinitely-large one is conservatively
    /// chosen.
    fn new(aabb: Option<&Aabb>) -> Self {
        match aabb {
            Some(aabb) => MeshCullingData {
                aabb_center: aabb.center.extend(0.0),
                aabb_half_extents: aabb.half_extents.extend(0.0),
            },
            None => MeshCullingData {
                aabb_center: Vec3::ZERO.extend(0.0),
                aabb_half_extents: Vec3::INFINITY.extend(0.0),
            },
        }
    }

    /// Flushes this mesh instance culling data to the
    /// [`MeshCullingDataBuffer`].
    fn add_to(&self, mesh_culling_data_buffer: &mut MeshCullingDataBuffer) -> usize {
        mesh_culling_data_buffer.push(*self)
    }
}

impl Default for MeshCullingDataBuffer {
    #[inline]
    fn default() -> Self {
        Self(RawBufferVec::new(BufferUsages::STORAGE))
    }
}

/// Data that [`crate::material::queue_material_meshes`] and similar systems
/// need in order to place entities that contain meshes in the right batch.
#[derive(Deref)]
pub struct RenderMeshQueueData<'a> {
    /// General information about the mesh instance.
    #[deref]
    pub shared: &'a RenderMeshInstanceShared,
    /// The translation of the mesh instance.
    pub translation: Vec3,
}

/// A [`SystemSet`] that encompasses both [`extract_meshes_for_cpu_building`]
/// and [`extract_meshes_for_gpu_building`].
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ExtractMeshesSet;

/// Extracts meshes from the main world into the render world, populating the
/// [`RenderMeshInstances`].
///
/// This is the variant of the system that runs when we're *not* using GPU
/// [`MeshUniform`] building.
pub fn extract_meshes_for_cpu_building(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    mut render_mesh_instance_queues: Local<Parallel<Vec<(Entity, RenderMeshInstanceCpu)>>>,
    meshes_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            &Handle<Mesh>,
            Has<NotShadowReceiver>,
            Has<TransmittedShadowReceiver>,
            Has<NotShadowCaster>,
            Has<NoAutomaticBatching>,
            Has<VisibilityRange>,
        )>,
    >,
) {
    meshes_query.par_iter().for_each_init(
        || render_mesh_instance_queues.borrow_local_mut(),
        |queue,
         (
            entity,
            view_visibility,
            transform,
            previous_transform,
            handle,
            not_shadow_receiver,
            transmitted_receiver,
            not_shadow_caster,
            no_automatic_batching,
            visibility_range,
        )| {
            if !view_visibility.get() {
                return;
            }

            let mut lod_index = None;
            if visibility_range {
                lod_index = render_visibility_ranges.lod_index_for_entity(entity);
            }

            let mesh_flags = MeshFlags::from_components(
                transform,
                lod_index,
                not_shadow_receiver,
                transmitted_receiver,
            );

            let shared = RenderMeshInstanceShared::from_components(
                previous_transform,
                handle,
                not_shadow_caster,
                no_automatic_batching,
            );

            let world_from_local = transform.affine();
            queue.push((
                entity,
                RenderMeshInstanceCpu {
                    transforms: MeshTransforms {
                        world_from_local: (&world_from_local).into(),
                        previous_world_from_local: (&previous_transform
                            .map(|t| t.0)
                            .unwrap_or(world_from_local))
                            .into(),
                        flags: mesh_flags.bits(),
                    },
                    shared,
                },
            ));
        },
    );

    // Collect the render mesh instances.
    let RenderMeshInstances::CpuBuilding(ref mut render_mesh_instances) = *render_mesh_instances
    else {
        panic!(
            "`extract_meshes_for_cpu_building` should only be called if we're using CPU \
            `MeshUniform` building"
        );
    };

    render_mesh_instances.clear();
    for queue in render_mesh_instance_queues.iter_mut() {
        for (entity, render_mesh_instance) in queue.drain(..) {
            render_mesh_instances.insert_unique_unchecked(entity, render_mesh_instance);
        }
    }
}

/// Extracts meshes from the main world into the render world and queues
/// [`MeshInputUniform`]s to be uploaded to the GPU.
///
/// This is the variant of the system that runs when we're using GPU
/// [`MeshUniform`] building.
pub fn extract_meshes_for_gpu_building(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    mut batched_instance_buffers: ResMut<
        gpu_preprocessing::BatchedInstanceBuffers<MeshUniform, MeshInputUniform>,
    >,
    mut mesh_culling_data_buffer: ResMut<MeshCullingDataBuffer>,
    mut render_mesh_instance_queues: Local<Parallel<RenderMeshInstanceGpuQueue>>,
    meshes_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            Option<&Lightmap>,
            Option<&Aabb>,
            &Handle<Mesh>,
            Has<NotShadowReceiver>,
            Has<TransmittedShadowReceiver>,
            Has<NotShadowCaster>,
            Has<NoAutomaticBatching>,
            Has<VisibilityRange>,
        )>,
    >,
    cameras_query: Extract<Query<(), (With<Camera>, With<GpuCulling>)>>,
) {
    let any_gpu_culling = !cameras_query.is_empty();
    for render_mesh_instance_queue in render_mesh_instance_queues.iter_mut() {
        render_mesh_instance_queue.init(any_gpu_culling);
    }

    // Collect render mesh instances. Build up the uniform buffer.
    let RenderMeshInstances::GpuBuilding(ref mut render_mesh_instances) = *render_mesh_instances
    else {
        panic!(
            "`extract_meshes_for_gpu_building` should only be called if we're \
            using GPU `MeshUniform` building"
        );
    };

    meshes_query.par_iter().for_each_init(
        || render_mesh_instance_queues.borrow_local_mut(),
        |queue,
         (
            entity,
            view_visibility,
            transform,
            previous_transform,
            lightmap,
            aabb,
            handle,
            not_shadow_receiver,
            transmitted_receiver,
            not_shadow_caster,
            no_automatic_batching,
            visibility_range,
        )| {
            if !view_visibility.get() {
                return;
            }

            let mut lod_index = None;
            if visibility_range {
                lod_index = render_visibility_ranges.lod_index_for_entity(entity);
            }

            let mesh_flags = MeshFlags::from_components(
                transform,
                lod_index,
                not_shadow_receiver,
                transmitted_receiver,
            );

            let shared = RenderMeshInstanceShared::from_components(
                previous_transform,
                handle,
                not_shadow_caster,
                no_automatic_batching,
            );

            let lightmap_uv_rect =
                lightmap::pack_lightmap_uv_rect(lightmap.map(|lightmap| lightmap.uv_rect));

            let gpu_mesh_culling_data = any_gpu_culling.then(|| MeshCullingData::new(aabb));

            let previous_input_index = if shared
                .flags
                .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_TRANSFORM)
            {
                render_mesh_instances
                    .get(&entity)
                    .map(|render_mesh_instance| render_mesh_instance.current_uniform_index)
            } else {
                None
            };

            let gpu_mesh_instance_builder = RenderMeshInstanceGpuBuilder {
                shared,
                world_from_local: (&transform.affine()).into(),
                lightmap_uv_rect,
                mesh_flags,
                previous_input_index,
            };

            queue.push(entity, gpu_mesh_instance_builder, gpu_mesh_culling_data);
        },
    );

    collect_meshes_for_gpu_building(
        render_mesh_instances,
        &mut batched_instance_buffers,
        &mut mesh_culling_data_buffer,
        &mut render_mesh_instance_queues,
    );
}

/// A system that sets the [`RenderMeshInstanceFlags`] for each mesh based on
/// whether the previous frame had skins and/or morph targets.
///
/// Ordinarily, [`RenderMeshInstanceFlags`] are set during the extraction phase.
/// However, we can't do that for the flags related to skins and morph targets
/// because the previous frame's skin and morph targets are the responsibility
/// of [`extract_skins`] and [`extract_morphs`] respectively. We want to run
/// those systems in parallel with mesh extraction for performance, so we need
/// to defer setting of these mesh instance flags to after extraction, which
/// this system does. An alternative to having skin- and morph-target-related
/// data in [`RenderMeshInstanceFlags`] would be to have
/// [`crate::material::queue_material_meshes`] check the skin and morph target
/// tables for each mesh, but that would be too slow in the hot mesh queuing
/// loop.
fn set_mesh_motion_vector_flags(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    skin_indices: Res<SkinIndices>,
    morph_indices: Res<MorphIndices>,
) {
    for &entity in skin_indices.prev.keys() {
        render_mesh_instances
            .insert_mesh_instance_flags(entity, RenderMeshInstanceFlags::HAS_PREVIOUS_SKIN);
    }
    for &entity in morph_indices.prev.keys() {
        render_mesh_instances
            .insert_mesh_instance_flags(entity, RenderMeshInstanceFlags::HAS_PREVIOUS_MORPH);
    }
}

/// Creates the [`RenderMeshInstanceGpu`]s and [`MeshInputUniform`]s when GPU
/// mesh uniforms are built.
fn collect_meshes_for_gpu_building(
    render_mesh_instances: &mut RenderMeshInstancesGpu,
    batched_instance_buffers: &mut gpu_preprocessing::BatchedInstanceBuffers<
        MeshUniform,
        MeshInputUniform,
    >,
    mesh_culling_data_buffer: &mut MeshCullingDataBuffer,
    render_mesh_instance_queues: &mut Parallel<RenderMeshInstanceGpuQueue>,
) {
    // Collect render mesh instances. Build up the uniform buffer.

    let gpu_preprocessing::BatchedInstanceBuffers {
        ref mut current_input_buffer,
        ref mut previous_input_buffer,
        ..
    } = batched_instance_buffers;

    // Swap buffers.
    mem::swap(current_input_buffer, previous_input_buffer);

    // Build the [`RenderMeshInstance`]s and [`MeshInputUniform`]s.
    render_mesh_instances.clear();

    for queue in render_mesh_instance_queues.iter_mut() {
        match *queue {
            RenderMeshInstanceGpuQueue::None => {
                // This can only happen if the queue is empty.
            }
            RenderMeshInstanceGpuQueue::CpuCulling(ref mut queue) => {
                for (entity, mesh_instance_builder) in queue.drain(..) {
                    mesh_instance_builder.add_to(
                        entity,
                        render_mesh_instances,
                        current_input_buffer,
                    );
                }
            }
            RenderMeshInstanceGpuQueue::GpuCulling(ref mut queue) => {
                for (entity, mesh_instance_builder, mesh_culling_builder) in queue.drain(..) {
                    let instance_data_index = mesh_instance_builder.add_to(
                        entity,
                        render_mesh_instances,
                        current_input_buffer,
                    );
                    let culling_data_index = mesh_culling_builder.add_to(mesh_culling_data_buffer);
                    debug_assert_eq!(instance_data_index, culling_data_index);
                }
            }
        }
    }
}

/// All data needed to construct a pipeline for rendering 3D meshes.
#[derive(Resource, Clone)]
pub struct MeshPipeline {
    /// A reference to all the mesh pipeline view layouts.
    pub view_layouts: MeshPipelineViewLayouts,
    // This dummy white texture is to be used in place of optional StandardMaterial textures
    pub dummy_white_gpu_image: GpuImage,
    pub clustered_forward_buffer_binding_type: BufferBindingType,
    pub mesh_layouts: MeshLayouts,
    /// `MeshUniform`s are stored in arrays in buffers. If storage buffers are available, they
    /// are used and this will be `None`, otherwise uniform buffers will be used with batches
    /// of this many `MeshUniform`s, stored at dynamic offsets within the uniform buffer.
    /// Use code like this in custom shaders:
    /// ```wgsl
    /// ##ifdef PER_OBJECT_BUFFER_BATCH_SIZE
    /// @group(1) @binding(0) var<uniform> mesh: array<Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
    /// ##else
    /// @group(1) @binding(0) var<storage> mesh: array<Mesh>;
    /// ##endif // PER_OBJECT_BUFFER_BATCH_SIZE
    /// ```
    pub per_object_buffer_batch_size: Option<u32>,

    /// Whether binding arrays (a.k.a. bindless textures) are usable on the
    /// current render device.
    ///
    /// This affects whether reflection probes can be used.
    pub binding_arrays_are_usable: bool,
}

impl FromWorld for MeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
            Res<MeshPipelineViewLayouts>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue, view_layouts) =
            system_state.get_mut(world);

        let clustered_forward_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);

        // A 1x1x1 'all 1.0' texture to use as a dummy texture to use in place of optional StandardMaterial textures
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );

            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.size(),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        MeshPipeline {
            view_layouts: view_layouts.clone(),
            clustered_forward_buffer_binding_type,
            dummy_white_gpu_image,
            mesh_layouts: MeshLayouts::new(&render_device),
            per_object_buffer_batch_size: GpuArrayBuffer::<MeshUniform>::batch_size(&render_device),
            binding_arrays_are_usable: binding_arrays_are_usable(&render_device),
        }
    }
}

impl MeshPipeline {
    pub fn get_image_texture<'a>(
        &'a self,
        gpu_images: &'a RenderAssets<GpuImage>,
        handle_option: &Option<Handle<Image>>,
    ) -> Option<(&'a TextureView, &'a Sampler)> {
        if let Some(handle) = handle_option {
            let gpu_image = gpu_images.get(handle)?;
            Some((&gpu_image.texture_view, &gpu_image.sampler))
        } else {
            Some((
                &self.dummy_white_gpu_image.texture_view,
                &self.dummy_white_gpu_image.sampler,
            ))
        }
    }

    pub fn get_view_layout(&self, layout_key: MeshPipelineViewLayoutKey) -> &BindGroupLayout {
        self.view_layouts.get_view_layout(layout_key)
    }
}

impl GetBatchData for MeshPipeline {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<RenderLightmaps>,
        SRes<RenderAssets<GpuMesh>>,
    );
    // The material bind group ID, the mesh ID, and the lightmap ID,
    // respectively.
    type CompareData = (MaterialBindGroupId, AssetId<Mesh>, Option<AssetId<Image>>);

    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, lightmaps, _): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&entity)?;
        let maybe_lightmap = lightmaps.render_lightmaps.get(&entity);

        Some((
            MeshUniform::new(
                &mesh_instance.transforms,
                maybe_lightmap.map(|lightmap| lightmap.uv_rect),
            ),
            mesh_instance.should_batch().then_some((
                mesh_instance.material_bind_group_id.get(),
                mesh_instance.mesh_asset_id,
                maybe_lightmap.map(|lightmap| lightmap.image),
            )),
        ))
    }
}

impl GetFullBatchData for MeshPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (mesh_instances, lightmaps, _): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_index_and_compare_data` should never be called in CPU mesh uniform building \
                mode"
            );
            return None;
        };

        let mesh_instance = mesh_instances.get(&entity)?;
        let maybe_lightmap = lightmaps.render_lightmaps.get(&entity);

        Some((
            mesh_instance.current_uniform_index,
            mesh_instance.should_batch().then_some((
                mesh_instance.material_bind_group_id.get(),
                mesh_instance.mesh_asset_id,
                maybe_lightmap.map(|lightmap| lightmap.image),
            )),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, lightmaps, _): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&entity)?;
        let maybe_lightmap = lightmaps.render_lightmaps.get(&entity);

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            maybe_lightmap.map(|lightmap| lightmap.uv_rect),
        ))
    }

    fn get_binned_index(
        (mesh_instances, _, _): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<NonMaxU32> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_index` should never be called in CPU mesh uniform \
                building mode"
            );
            return None;
        };

        mesh_instances
            .get(&entity)
            .map(|entity| entity.current_uniform_index)
    }

    fn get_batch_indirect_parameters_index(
        (mesh_instances, _, meshes): &SystemParamItem<Self::Param>,
        indirect_parameters_buffer: &mut IndirectParametersBuffer,
        entity: Entity,
        instance_index: u32,
    ) -> Option<NonMaxU32> {
        get_batch_indirect_parameters_index(
            mesh_instances,
            meshes,
            indirect_parameters_buffer,
            entity,
            instance_index,
        )
    }
}

/// Pushes a set of [`IndirectParameters`] onto the [`IndirectParametersBuffer`]
/// for the given mesh instance, and returns the index of those indirect
/// parameters.
fn get_batch_indirect_parameters_index(
    mesh_instances: &RenderMeshInstances,
    meshes: &RenderAssets<GpuMesh>,
    indirect_parameters_buffer: &mut IndirectParametersBuffer,
    entity: Entity,
    instance_index: u32,
) -> Option<NonMaxU32> {
    // This should only be called during GPU building.
    let RenderMeshInstances::GpuBuilding(ref mesh_instances) = *mesh_instances else {
        error!(
            "`get_batch_indirect_parameters_index` should never be called in CPU mesh uniform \
                building mode"
        );
        return None;
    };

    let mesh_instance = mesh_instances.get(&entity)?;
    let mesh = meshes.get(mesh_instance.mesh_asset_id)?;

    // Note that `IndirectParameters` covers both of these structures, even
    // though they actually have distinct layouts. See the comment above that
    // type for more information.
    let indirect_parameters = match mesh.buffer_info {
        GpuBufferInfo::Indexed {
            count: index_count, ..
        } => IndirectParameters {
            vertex_or_index_count: index_count,
            instance_count: 0,
            first_vertex: 0,
            base_vertex_or_first_instance: 0,
            first_instance: instance_index,
        },
        GpuBufferInfo::NonIndexed => IndirectParameters {
            vertex_or_index_count: mesh.vertex_count,
            instance_count: 0,
            first_vertex: 0,
            base_vertex_or_first_instance: instance_index,
            first_instance: instance_index,
        },
    };

    (indirect_parameters_buffer.push(indirect_parameters) as u32)
        .try_into()
        .ok()
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    /// MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct MeshPipelineKey: u64 {
        // Nothing
        const NONE                              = 0;

        // Inherited bits
        const MORPH_TARGETS                     = BaseMeshPipelineKey::MORPH_TARGETS.bits();

        // Flag bits
        const HDR                               = 1 << 0;
        const TONEMAP_IN_SHADER                 = 1 << 1;
        const DEBAND_DITHER                     = 1 << 2;
        const DEPTH_PREPASS                     = 1 << 3;
        const NORMAL_PREPASS                    = 1 << 4;
        const DEFERRED_PREPASS                  = 1 << 5;
        const MOTION_VECTOR_PREPASS             = 1 << 6;
        const MAY_DISCARD                       = 1 << 7; // Guards shader codepaths that may discard, allowing early depth tests in most cases
                                                            // See: https://www.khronos.org/opengl/wiki/Early_Fragment_Test
        const ENVIRONMENT_MAP                   = 1 << 8;
        const SCREEN_SPACE_AMBIENT_OCCLUSION    = 1 << 9;
        const DEPTH_CLAMP_ORTHO                 = 1 << 10;
        const TEMPORAL_JITTER                   = 1 << 11;
        const READS_VIEW_TRANSMISSION_TEXTURE   = 1 << 12;
        const LIGHTMAPPED                       = 1 << 13;
        const IRRADIANCE_VOLUME                 = 1 << 14;
        const VISIBILITY_RANGE_DITHER           = 1 << 15;
        const SCREEN_SPACE_REFLECTIONS          = 1 << 16;
        const HAS_PREVIOUS_SKIN                 = 1 << 17;
        const HAS_PREVIOUS_MORPH                = 1 << 18;
        const LAST_FLAG                         = Self::HAS_PREVIOUS_MORPH.bits();

        // Bitfields
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        const BLEND_RESERVED_BITS               = Self::BLEND_MASK_BITS << Self::BLEND_SHIFT_BITS; // ← Bitmask reserving bits for the blend state
        const BLEND_OPAQUE                      = 0 << Self::BLEND_SHIFT_BITS;                     // ← Values are just sequential within the mask
        const BLEND_PREMULTIPLIED_ALPHA         = 1 << Self::BLEND_SHIFT_BITS;                     // ← As blend states is on 3 bits, it can range from 0 to 7
        const BLEND_MULTIPLY                    = 2 << Self::BLEND_SHIFT_BITS;                     // ← See `BLEND_MASK_BITS` for the number of bits available
        const BLEND_ALPHA                       = 3 << Self::BLEND_SHIFT_BITS;                     //
        const BLEND_ALPHA_TO_COVERAGE           = 4 << Self::BLEND_SHIFT_BITS;                     // ← We still have room for three more values without adding more bits
        const TONEMAP_METHOD_RESERVED_BITS      = Self::TONEMAP_METHOD_MASK_BITS << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_NONE               = 0 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD           = 1 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD_LUMINANCE = 2 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_ACES_FITTED        = 3 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_AGX                = 4 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM = 5 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_TONY_MC_MAPFACE     = 6 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_BLENDER_FILMIC      = 7 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_RESERVED_BITS = Self::SHADOW_FILTER_METHOD_MASK_BITS << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_HARDWARE_2X2  = 0 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_GAUSSIAN      = 1 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_TEMPORAL      = 2 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const VIEW_PROJECTION_RESERVED_BITS     = Self::VIEW_PROJECTION_MASK_BITS << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_NONSTANDARD       = 0 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_PERSPECTIVE       = 1 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_ORTHOGRAPHIC      = 2 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_RESERVED          = 3 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_RESERVED_BITS = Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_MASK_BITS << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_LOW = 0 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_MEDIUM = 1 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_HIGH = 2 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_ULTRA = 3 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const ALL_RESERVED_BITS =
            Self::BLEND_RESERVED_BITS.bits() |
            Self::MSAA_RESERVED_BITS.bits() |
            Self::TONEMAP_METHOD_RESERVED_BITS.bits() |
            Self::SHADOW_FILTER_METHOD_RESERVED_BITS.bits() |
            Self::VIEW_PROJECTION_RESERVED_BITS.bits() |
            Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_RESERVED_BITS.bits();
    }
}

impl MeshPipelineKey {
    const MSAA_MASK_BITS: u64 = 0b111;
    const MSAA_SHIFT_BITS: u64 = Self::LAST_FLAG.bits().trailing_zeros() as u64 + 1;

    const BLEND_MASK_BITS: u64 = 0b111;
    const BLEND_SHIFT_BITS: u64 = Self::MSAA_MASK_BITS.count_ones() as u64 + Self::MSAA_SHIFT_BITS;

    const TONEMAP_METHOD_MASK_BITS: u64 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u64 =
        Self::BLEND_MASK_BITS.count_ones() as u64 + Self::BLEND_SHIFT_BITS;

    const SHADOW_FILTER_METHOD_MASK_BITS: u64 = 0b11;
    const SHADOW_FILTER_METHOD_SHIFT_BITS: u64 =
        Self::TONEMAP_METHOD_MASK_BITS.count_ones() as u64 + Self::TONEMAP_METHOD_SHIFT_BITS;

    const VIEW_PROJECTION_MASK_BITS: u64 = 0b11;
    const VIEW_PROJECTION_SHIFT_BITS: u64 = Self::SHADOW_FILTER_METHOD_MASK_BITS.count_ones()
        as u64
        + Self::SHADOW_FILTER_METHOD_SHIFT_BITS;

    const SCREEN_SPACE_SPECULAR_TRANSMISSION_MASK_BITS: u64 = 0b11;
    const SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS: u64 =
        Self::VIEW_PROJECTION_MASK_BITS.count_ones() as u64 + Self::VIEW_PROJECTION_SHIFT_BITS;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() as u64 & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    pub fn from_hdr(hdr: bool) -> Self {
        if hdr {
            MeshPipelineKey::HDR
        } else {
            MeshPipelineKey::NONE
        }
    }

    pub fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u64)
            & BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits)
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits = (self.bits()
            >> BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
            & BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u64 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u64 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u64 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u64 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u64 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
        }
    }
}

// Ensure that we didn't overflow the number of bits available in `MeshPipelineKey`.
const_assert_eq!(
    (((MeshPipelineKey::LAST_FLAG.bits() << 1) - 1) | MeshPipelineKey::ALL_RESERVED_BITS.bits())
        & BaseMeshPipelineKey::all().bits(),
    0
);

// Ensure that the reserved bits don't overlap with the topology bits
const_assert_eq!(
    (BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS
        << BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
        & MeshPipelineKey::ALL_RESERVED_BITS.bits(),
    0
);

fn is_skinned(layout: &MeshVertexBufferLayoutRef) -> bool {
    layout.0.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
        && layout.0.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
}
pub fn setup_morph_and_skinning_defs(
    mesh_layouts: &MeshLayouts,
    layout: &MeshVertexBufferLayoutRef,
    offset: u32,
    key: &MeshPipelineKey,
    shader_defs: &mut Vec<ShaderDefVal>,
    vertex_attributes: &mut Vec<VertexAttributeDescriptor>,
) -> BindGroupLayout {
    let mut add_skin_data = || {
        shader_defs.push("SKINNED".into());
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(offset));
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(offset + 1));
    };
    let is_morphed = key.intersects(MeshPipelineKey::MORPH_TARGETS);
    let is_lightmapped = key.intersects(MeshPipelineKey::LIGHTMAPPED);
    let motion_vector_prepass = key.intersects(MeshPipelineKey::MOTION_VECTOR_PREPASS);
    match (
        is_skinned(layout),
        is_morphed,
        is_lightmapped,
        motion_vector_prepass,
    ) {
        (true, false, _, true) => {
            add_skin_data();
            mesh_layouts.skinned_motion.clone()
        }
        (true, false, _, false) => {
            add_skin_data();
            mesh_layouts.skinned.clone()
        }
        (true, true, _, true) => {
            add_skin_data();
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_skinned_motion.clone()
        }
        (true, true, _, false) => {
            add_skin_data();
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_skinned.clone()
        }
        (false, true, _, true) => {
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_motion.clone()
        }
        (false, true, _, false) => {
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed.clone()
        }
        (false, false, true, _) => mesh_layouts.lightmapped.clone(),
        (false, false, false, _) => mesh_layouts.model_only.clone(),
    }
}

impl SpecializedMeshPipeline for MeshPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        // Let the shader code know that it's running in a mesh pipeline.
        shader_defs.push("MESH_PIPELINE".into());

        shader_defs.push("VERTEX_OUTPUT_INSTANCE_INDEX".into());

        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_NORMAL) {
            shader_defs.push("VERTEX_NORMALS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(1));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            shader_defs.push("VERTEX_UVS_A".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_1) {
            shader_defs.push("VERTEX_UVS".into());
            shader_defs.push("VERTEX_UVS_B".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_1.at_shader_location(3));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push("VERTEX_TANGENTS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(4));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(5));
        }

        if cfg!(feature = "pbr_transmission_textures") {
            shader_defs.push("PBR_TRANSMISSION_TEXTURES_SUPPORTED".into());
        }
        if cfg!(feature = "pbr_multi_layer_material_textures") {
            shader_defs.push("PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED".into());
        }

        let mut bind_group_layout = vec![self.get_view_layout(key.into()).clone()];

        if key.msaa_samples() > 1 {
            shader_defs.push("MULTISAMPLED".into());
        };

        bind_group_layout.push(setup_morph_and_skinning_defs(
            &self.mesh_layouts,
            layout,
            6,
            &key,
            &mut shader_defs,
            &mut vertex_attributes,
        ));

        if key.contains(MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION) {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        let (label, blend, depth_write_enabled);
        let pass = key.intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
        let (mut is_opaque, mut alpha_to_coverage_enabled) = (false, false);
        if pass == MeshPipelineKey::BLEND_ALPHA {
            label = "alpha_blend_mesh_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA {
            label = "premultiplied_alpha_mesh_pipeline".into();
            blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
            shader_defs.push("PREMULTIPLY_ALPHA".into());
            shader_defs.push("BLEND_PREMULTIPLIED_ALPHA".into());
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_MULTIPLY {
            label = "multiply_mesh_pipeline".into();
            blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent::OVER,
            });
            shader_defs.push("PREMULTIPLY_ALPHA".into());
            shader_defs.push("BLEND_MULTIPLY".into());
            // For the multiply pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_ALPHA_TO_COVERAGE {
            label = "alpha_to_coverage_mesh_pipeline".into();
            // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases
            blend = None;
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
            is_opaque = !key.contains(MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE);
            alpha_to_coverage_enabled = true;
            shader_defs.push("ALPHA_TO_COVERAGE".into());
        } else {
            label = "opaque_mesh_pipeline".into();
            // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases
            blend = None;
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
            is_opaque = !key.contains(MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE);
        }

        if key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::HAS_PREVIOUS_SKIN) {
            shader_defs.push("HAS_PREVIOUS_SKIN".into());
        }

        if key.contains(MeshPipelineKey::HAS_PREVIOUS_MORPH) {
            shader_defs.push("HAS_PREVIOUS_MORPH".into());
        }

        if key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            shader_defs.push("DEFERRED_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::NORMAL_PREPASS) && key.msaa_samples() == 1 && is_opaque {
            shader_defs.push("LOAD_PREPASS_NORMALS".into());
        }

        let view_projection = key.intersection(MeshPipelineKey::VIEW_PROJECTION_RESERVED_BITS);
        if view_projection == MeshPipelineKey::VIEW_PROJECTION_NONSTANDARD {
            shader_defs.push("VIEW_PROJECTION_NONSTANDARD".into());
        } else if view_projection == MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE {
            shader_defs.push("VIEW_PROJECTION_PERSPECTIVE".into());
        } else if view_projection == MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC {
            shader_defs.push("VIEW_PROJECTION_ORTHOGRAPHIC".into());
        }

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("WEBGL2".into());

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                20,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                21,
            ));

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(MeshPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if key.contains(MeshPipelineKey::MAY_DISCARD) {
            shader_defs.push("MAY_DISCARD".into());
        }

        if key.contains(MeshPipelineKey::ENVIRONMENT_MAP) {
            shader_defs.push("ENVIRONMENT_MAP".into());
        }

        if key.contains(MeshPipelineKey::IRRADIANCE_VOLUME) && IRRADIANCE_VOLUMES_ARE_USABLE {
            shader_defs.push("IRRADIANCE_VOLUME".into());
        }

        if key.contains(MeshPipelineKey::LIGHTMAPPED) {
            shader_defs.push("LIGHTMAP".into());
        }

        if key.contains(MeshPipelineKey::TEMPORAL_JITTER) {
            shader_defs.push("TEMPORAL_JITTER".into());
        }

        let shadow_filter_method =
            key.intersection(MeshPipelineKey::SHADOW_FILTER_METHOD_RESERVED_BITS);
        if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2 {
            shader_defs.push("SHADOW_FILTER_METHOD_HARDWARE_2X2".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_GAUSSIAN {
            shader_defs.push("SHADOW_FILTER_METHOD_GAUSSIAN".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_TEMPORAL {
            shader_defs.push("SHADOW_FILTER_METHOD_TEMPORAL".into());
        }

        let blur_quality =
            key.intersection(MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_RESERVED_BITS);

        shader_defs.push(ShaderDefVal::Int(
            "SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS".into(),
            match blur_quality {
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_LOW => 4,
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_MEDIUM => 8,
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_HIGH => 16,
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_ULTRA => 32,
                _ => unreachable!(), // Not possible, since the mask is 2 bits, and we've covered all 4 cases
            },
        ));

        if key.contains(MeshPipelineKey::VISIBILITY_RANGE_DITHER) {
            shader_defs.push("VISIBILITY_RANGE_DITHER".into());
        }

        if self.binding_arrays_are_usable {
            shader_defs.push("MULTIPLE_LIGHT_PROBES_IN_ARRAY".into());
        }

        if IRRADIANCE_VOLUMES_ARE_USABLE {
            shader_defs.push("IRRADIANCE_VOLUMES_ARE_USABLE".into());
        }

        let format = if key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        // This is defined here so that custom shaders that use something other than
        // the mesh binding from bevy_pbr::mesh_bindings can easily make use of this
        // in their own shaders.
        if let Some(per_object_buffer_batch_size) = self.per_object_buffer_batch_size {
            shader_defs.push(ShaderDefVal::UInt(
                "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
                per_object_buffer_batch_size,
            ));
        }

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: MESH_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: MESH_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: bind_group_layout,
            push_constant_ranges: vec![],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled,
            },
            label: Some(label),
        })
    }
}

/// Bind groups for meshes currently loaded.
#[derive(Resource, Default)]
pub struct MeshBindGroups {
    model_only: Option<BindGroup>,
    skinned: Option<MeshBindGroupPair>,
    morph_targets: HashMap<AssetId<Mesh>, MeshBindGroupPair>,
    lightmaps: HashMap<AssetId<Image>, BindGroup>,
}

pub struct MeshBindGroupPair {
    motion_vectors: BindGroup,
    no_motion_vectors: BindGroup,
}

impl MeshBindGroups {
    pub fn reset(&mut self) {
        self.model_only = None;
        self.skinned = None;
        self.morph_targets.clear();
        self.lightmaps.clear();
    }
    /// Get the `BindGroup` for `GpuMesh` with given `handle_id` and lightmap
    /// key `lightmap`.
    pub fn get(
        &self,
        asset_id: AssetId<Mesh>,
        lightmap: Option<AssetId<Image>>,
        is_skinned: bool,
        morph: bool,
        motion_vectors: bool,
    ) -> Option<&BindGroup> {
        match (is_skinned, morph, lightmap) {
            (_, true, _) => self
                .morph_targets
                .get(&asset_id)
                .map(|bind_group_pair| bind_group_pair.get(motion_vectors)),
            (true, false, _) => self
                .skinned
                .as_ref()
                .map(|bind_group_pair| bind_group_pair.get(motion_vectors)),
            (false, false, Some(lightmap)) => self.lightmaps.get(&lightmap),
            (false, false, None) => self.model_only.as_ref(),
        }
    }
}

impl MeshBindGroupPair {
    fn get(&self, motion_vectors: bool) -> &BindGroup {
        if motion_vectors {
            &self.motion_vectors
        } else {
            &self.no_motion_vectors
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_mesh_bind_group(
    meshes: Res<RenderAssets<GpuMesh>>,
    images: Res<RenderAssets<GpuImage>>,
    mut groups: ResMut<MeshBindGroups>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
    cpu_batched_instance_buffer: Option<
        Res<no_gpu_preprocessing::BatchedInstanceBuffer<MeshUniform>>,
    >,
    gpu_batched_instance_buffers: Option<
        Res<gpu_preprocessing::BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    >,
    skins_uniform: Res<SkinUniforms>,
    weights_uniform: Res<MorphUniforms>,
    render_lightmaps: Res<RenderLightmaps>,
) {
    groups.reset();

    let layouts = &mesh_pipeline.mesh_layouts;

    let model = if let Some(cpu_batched_instance_buffer) = cpu_batched_instance_buffer {
        cpu_batched_instance_buffer
            .into_inner()
            .instance_data_binding()
    } else if let Some(gpu_batched_instance_buffers) = gpu_batched_instance_buffers {
        gpu_batched_instance_buffers
            .into_inner()
            .instance_data_binding()
    } else {
        return;
    };
    let Some(model) = model else { return };

    groups.model_only = Some(layouts.model_only(&render_device, &model));

    // Create the skinned mesh bind group with the current and previous buffers
    // (the latter being for motion vector computation). If there's no previous
    // buffer, just use the current one as the shader will ignore it.
    let skin = skins_uniform.current_buffer.buffer();
    if let Some(skin) = skin {
        let prev_skin = skins_uniform.prev_buffer.buffer().unwrap_or(skin);
        groups.skinned = Some(MeshBindGroupPair {
            motion_vectors: layouts.skinned_motion(&render_device, &model, skin, prev_skin),
            no_motion_vectors: layouts.skinned(&render_device, &model, skin),
        });
    }

    // Create the morphed bind groups just like we did for the skinned bind
    // group.
    if let Some(weights) = weights_uniform.current_buffer.buffer() {
        let prev_weights = weights_uniform.prev_buffer.buffer().unwrap_or(weights);
        for (id, gpu_mesh) in meshes.iter() {
            if let Some(targets) = gpu_mesh.morph_targets.as_ref() {
                let bind_group_pair = match skin.filter(|_| is_skinned(&gpu_mesh.layout)) {
                    Some(skin) => {
                        let prev_skin = skins_uniform.prev_buffer.buffer().unwrap_or(skin);
                        MeshBindGroupPair {
                            motion_vectors: layouts.morphed_skinned_motion(
                                &render_device,
                                &model,
                                skin,
                                weights,
                                targets,
                                prev_skin,
                                prev_weights,
                            ),
                            no_motion_vectors: layouts.morphed_skinned(
                                &render_device,
                                &model,
                                skin,
                                weights,
                                targets,
                            ),
                        }
                    }
                    None => MeshBindGroupPair {
                        motion_vectors: layouts.morphed_motion(
                            &render_device,
                            &model,
                            weights,
                            targets,
                            prev_weights,
                        ),
                        no_motion_vectors: layouts.morphed(
                            &render_device,
                            &model,
                            weights,
                            targets,
                        ),
                    },
                };
                groups.morph_targets.insert(id, bind_group_pair);
            }
        }
    }

    // Create lightmap bindgroups.
    for &image_id in &render_lightmaps.all_lightmap_images {
        if let (Entry::Vacant(entry), Some(image)) =
            (groups.lightmaps.entry(image_id), images.get(image_id))
        {
            entry.insert(layouts.lightmapped(&render_device, &model, image));
        }
    }
}

pub struct SetMeshViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMeshViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<ViewFogUniformOffset>,
        Read<ViewLightProbesUniformOffset>,
        Read<ViewScreenSpaceReflectionsUniformOffset>,
        Read<MeshViewBindGroup>,
    );
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, view_lights, view_fog, view_light_probes, view_ssr, mesh_view_bind_group): ROQueryItem<
            'w,
            Self::ViewQuery,
        >,
        _entity: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            &mesh_view_bind_group.value,
            &[
                view_uniform.offset,
                view_lights.offset,
                view_fog.offset,
                **view_light_probes,
                **view_ssr,
            ],
        );

        RenderCommandResult::Success
    }
}

pub struct SetMeshBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMeshBindGroup<I> {
    type Param = (
        SRes<MeshBindGroups>,
        SRes<RenderMeshInstances>,
        SRes<SkinIndices>,
        SRes<MorphIndices>,
        SRes<RenderLightmaps>,
    );
    type ViewQuery = Has<MotionVectorPrepass>;
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        has_motion_vector_prepass: bool,
        _item_query: Option<()>,
        (bind_groups, mesh_instances, skin_indices, morph_indices, lightmaps): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = bind_groups.into_inner();
        let mesh_instances = mesh_instances.into_inner();
        let skin_indices = skin_indices.into_inner();
        let morph_indices = morph_indices.into_inner();

        let entity = &item.entity();

        let Some(mesh_asset_id) = mesh_instances.mesh_asset_id(*entity) else {
            return RenderCommandResult::Success;
        };
        let current_skin_index = skin_indices.current.get(entity);
        let prev_skin_index = skin_indices.prev.get(entity);
        let current_morph_index = morph_indices.current.get(entity);
        let prev_morph_index = morph_indices.prev.get(entity);

        let is_skinned = current_skin_index.is_some();
        let is_morphed = current_morph_index.is_some();

        let lightmap = lightmaps
            .render_lightmaps
            .get(entity)
            .map(|render_lightmap| render_lightmap.image);

        let Some(bind_group) = bind_groups.get(
            mesh_asset_id,
            lightmap,
            is_skinned,
            is_morphed,
            has_motion_vector_prepass,
        ) else {
            error!(
                "The MeshBindGroups resource wasn't set in the render phase. \
                It should be set by the prepare_mesh_bind_group system.\n\
                This is a bevy bug! Please open an issue."
            );
            return RenderCommandResult::Failure;
        };

        let mut dynamic_offsets: [u32; 3] = Default::default();
        let mut offset_count = 0;
        if let Some(dynamic_offset) = item.extra_index().as_dynamic_offset() {
            dynamic_offsets[offset_count] = dynamic_offset.get();
            offset_count += 1;
        }
        if let Some(current_skin_index) = current_skin_index {
            dynamic_offsets[offset_count] = current_skin_index.index;
            offset_count += 1;
        }
        if let Some(current_morph_index) = current_morph_index {
            dynamic_offsets[offset_count] = current_morph_index.index;
            offset_count += 1;
        }

        // Attach motion vectors if needed.
        if has_motion_vector_prepass {
            // Attach the previous skin index for motion vector computation. If
            // there isn't one, just use zero as the shader will ignore it.
            if current_skin_index.is_some() {
                match prev_skin_index {
                    Some(prev_skin_index) => dynamic_offsets[offset_count] = prev_skin_index.index,
                    None => dynamic_offsets[offset_count] = 0,
                }
                offset_count += 1;
            }

            // Attach the previous morph index for motion vector computation. If
            // there isn't one, just use zero as the shader will ignore it.
            if current_morph_index.is_some() {
                match prev_morph_index {
                    Some(prev_morph_index) => {
                        dynamic_offsets[offset_count] = prev_morph_index.index;
                    }
                    None => dynamic_offsets[offset_count] = 0,
                }
                offset_count += 1;
            }
        }

        pass.set_bind_group(I, bind_group, &dynamic_offsets[0..offset_count]);

        RenderCommandResult::Success
    }
}

pub struct DrawMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawMesh {
    type Param = (
        SRes<RenderAssets<GpuMesh>>,
        SRes<RenderMeshInstances>,
        SRes<IndirectParametersBuffer>,
        SRes<PipelineCache>,
        Option<SRes<PreprocessPipelines>>,
    );
    type ViewQuery = Has<PreprocessBindGroup>;
    type ItemQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        has_preprocess_bind_group: ROQueryItem<Self::ViewQuery>,
        _item_query: Option<()>,
        (meshes, mesh_instances, indirect_parameters_buffer, pipeline_cache, preprocess_pipelines): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // If we're using GPU preprocessing, then we're dependent on that
        // compute shader having been run, which of course can only happen if
        // it's compiled. Otherwise, our mesh instance data won't be present.
        if let Some(preprocess_pipelines) = preprocess_pipelines {
            if !has_preprocess_bind_group
                || !preprocess_pipelines.pipelines_are_loaded(&pipeline_cache)
            {
                return RenderCommandResult::Failure;
            }
        }

        let meshes = meshes.into_inner();
        let mesh_instances = mesh_instances.into_inner();
        let indirect_parameters_buffer = indirect_parameters_buffer.into_inner();

        let Some(mesh_asset_id) = mesh_instances.mesh_asset_id(item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let Some(gpu_mesh) = meshes.get(mesh_asset_id) else {
            return RenderCommandResult::Failure;
        };

        // Calculate the indirect offset, and look up the buffer.
        let indirect_parameters = match item.extra_index().as_indirect_parameters_index() {
            None => None,
            Some(index) => match indirect_parameters_buffer.buffer() {
                None => {
                    warn!("Not rendering mesh because indirect parameters buffer wasn't present");
                    return RenderCommandResult::Failure;
                }
                Some(buffer) => Some((
                    index as u64 * mem::size_of::<IndirectParameters>() as u64,
                    buffer,
                )),
            },
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

        let batch_range = item.batch_range();

        // Draw either directly or indirectly, as appropriate.
        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                match indirect_parameters {
                    None => {
                        pass.draw_indexed(0..*count, 0, batch_range.clone());
                    }
                    Some((indirect_parameters_offset, indirect_parameters_buffer)) => pass
                        .draw_indexed_indirect(
                            indirect_parameters_buffer,
                            indirect_parameters_offset,
                        ),
                }
            }
            GpuBufferInfo::NonIndexed => match indirect_parameters {
                None => {
                    pass.draw(0..gpu_mesh.vertex_count, batch_range.clone());
                }
                Some((indirect_parameters_offset, indirect_parameters_buffer)) => {
                    pass.draw_indirect(indirect_parameters_buffer, indirect_parameters_offset);
                }
            },
        }
        RenderCommandResult::Success
    }
}

#[cfg(test)]
mod tests {
    use super::MeshPipelineKey;
    #[test]
    fn mesh_key_msaa_samples() {
        for i in [1, 2, 4, 8, 16, 32, 64, 128] {
            assert_eq!(MeshPipelineKey::from_msaa_samples(i).msaa_samples(), i);
        }
    }
}
