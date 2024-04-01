use std::mem;

use bevy_asset::{load_internal_asset, AssetId};
use bevy_core_pipeline::{
    core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d, CORE_3D_DEPTH_FORMAT},
    deferred::{AlphaMask3dDeferred, Opaque3dDeferred},
};
use bevy_derive::Deref;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{
    prelude::*,
    query::ROQueryItem,
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_math::{Affine3, Rect, UVec2, Vec3, Vec4};
use bevy_render::{
    batching::{
        batch_and_prepare_binned_render_phase, batch_and_prepare_sorted_render_phase,
        sort_binned_render_phase, write_batched_instance_buffer, BatchedInstanceBuffers,
        GetBatchData, GetBinnedBatchData, NoAutomaticBatching,
    },
    mesh::*,
    render_asset::RenderAssets,
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, DefaultImageSampler, GpuImage, ImageSampler, TextureFormatPixelInfo},
    view::{ViewTarget, ViewUniformOffset, ViewVisibility},
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{tracing::error, Entry, HashMap, Parallel};

#[cfg(debug_assertions)]
use bevy_utils::warn_once;
use bytemuck::{Pod, Zeroable};

use crate::render::{
    morph::{
        extract_morphs, no_automatic_morph_batching, prepare_morphs, MorphIndices, MorphUniform,
    },
    skin::no_automatic_skin_batching,
};
use crate::*;

use self::irradiance_volume::IRRADIANCE_VOLUMES_ARE_USABLE;

use super::skin::SkinIndices;

/// Provides support for rendering PBR meshes.
pub struct MeshRenderPlugin {
    /// Whether we're building [`MeshUniform`]s on GPU.
    ///
    /// If this is false, we're building them on CPU.
    pub use_gpu_uniform_builder: bool,
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

impl Default for MeshRenderPlugin {
    fn default() -> Self {
        Self {
            use_gpu_uniform_builder: true,
        }
    }
}

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
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            let render_mesh_instances = RenderMeshInstances::new(self.use_gpu_uniform_builder);

            render_app
                .init_resource::<MeshBindGroups>()
                .init_resource::<SkinUniform>()
                .init_resource::<SkinIndices>()
                .init_resource::<MorphUniform>()
                .init_resource::<MorphIndices>()
                .insert_resource(render_mesh_instances)
                .add_systems(ExtractSchedule, (extract_skins, extract_morphs))
                .add_systems(
                    Render,
                    (
                        (
                            sort_binned_render_phase::<Opaque3d>,
                            sort_binned_render_phase::<AlphaMask3d>,
                            sort_binned_render_phase::<Shadow>,
                            sort_binned_render_phase::<Opaque3dDeferred>,
                            sort_binned_render_phase::<AlphaMask3dDeferred>,
                        )
                            .in_set(RenderSet::PhaseSort),
                        (
                            batch_and_prepare_binned_render_phase::<Opaque3d, MeshPipeline>,
                            batch_and_prepare_sorted_render_phase::<Transmissive3d, MeshPipeline>,
                            batch_and_prepare_sorted_render_phase::<Transparent3d, MeshPipeline>,
                            batch_and_prepare_binned_render_phase::<AlphaMask3d, MeshPipeline>,
                            batch_and_prepare_binned_render_phase::<Shadow, MeshPipeline>,
                            batch_and_prepare_binned_render_phase::<Opaque3dDeferred, MeshPipeline>,
                            batch_and_prepare_binned_render_phase::<
                                AlphaMask3dDeferred,
                                MeshPipeline,
                            >,
                        )
                            .in_set(RenderSet::PrepareResources),
                        write_batched_instance_buffer::<MeshPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                        prepare_skins.in_set(RenderSet::PrepareResources),
                        prepare_morphs.in_set(RenderSet::PrepareResources),
                        prepare_mesh_bind_group.in_set(RenderSet::PrepareBindGroups),
                        prepare_mesh_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    ),
                );

            if self.use_gpu_uniform_builder {
                render_app.add_systems(ExtractSchedule, extract_meshes_for_gpu_building);
            } else {
                render_app.add_systems(ExtractSchedule, extract_meshes_for_cpu_building);
            }
        }
    }

    fn finish(&self, app: &mut App) {
        let mut mesh_bindings_shader_defs = Vec::with_capacity(1);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            let render_device = render_app.world.resource::<RenderDevice>();
            let batched_instance_buffers =
                BatchedInstanceBuffers::<MeshUniform, MeshInputUniform>::new(
                    render_device,
                    self.use_gpu_uniform_builder,
                );

            if let Some(per_object_buffer_batch_size) =
                GpuArrayBuffer::<MeshUniform>::batch_size(render_device)
            {
                mesh_bindings_shader_defs.push(ShaderDefVal::UInt(
                    "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
                    per_object_buffer_batch_size,
                ));
            }

            render_app
                .insert_resource(batched_instance_buffers)
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
    pub transform: Affine3,
    pub previous_transform: Affine3,
    pub flags: u32,
}

#[derive(ShaderType, Clone)]
pub struct MeshUniform {
    // Affine 4x3 matrices transposed to 3x4
    pub transform: [Vec4; 3],
    pub previous_transform: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub inverse_transpose_model_a: [Vec4; 2],
    pub inverse_transpose_model_b: f32,
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
    pub transform: [Vec4; 3],
    /// Four 16-bit unsigned normalized UV values packed into a `UVec2`:
    ///
    ///                         <--- MSB                   LSB --->
    ///                         +---- min v ----+ +---- min u ----+
    ///     lightmap_uv_rect.x: vvvvvvvv vvvvvvvv uuuuuuuu uuuuuuuu,
    ///                         +---- max v ----+ +---- max u ----+
    ///     lightmap_uv_rect.y: VVVVVVVV VVVVVVVV UUUUUUUU UUUUUUUU,
    ///
    /// (MSB: most significant bit; LSB: least significant bit.)
    pub lightmap_uv_rect: UVec2,
    /// Various [`MeshFlags`].
    pub flags: u32,
    /// The index of this mesh's [`MeshInputUniform`] in the previous frame's
    /// buffer, if applicable.
    ///
    /// This is used for TAA. If not present, this will be `!0`.
    pub previous_input_index: u32,
}

impl MeshUniform {
    pub fn new(mesh_transforms: &MeshTransforms, maybe_lightmap_uv_rect: Option<Rect>) -> Self {
        let (inverse_transpose_model_a, inverse_transpose_model_b) =
            mesh_transforms.transform.inverse_transpose_3x3();
        Self {
            transform: mesh_transforms.transform.to_transpose(),
            previous_transform: mesh_transforms.previous_transform.to_transpose(),
            lightmap_uv_rect: lightmap::pack_lightmap_uv_rect(maybe_lightmap_uv_rect),
            inverse_transpose_model_a,
            inverse_transpose_model_b,
            flags: mesh_transforms.flags,
        }
    }
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_types.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        const SHADOW_RECEIVER             = 1 << 0;
        const TRANSMITTED_SHADOW_RECEIVER = 1 << 1;
        // Indicates the sign of the determinant of the 3x3 model matrix. If the sign is positive,
        // then the flag should be set, else it should not be set.
        const SIGN_DETERMINANT_MODEL_3X3  = 1 << 31;
        const NONE                        = 0;
        const UNINITIALIZED               = 0xFFFF;
    }
}

impl MeshFlags {
    fn from_components(
        transform: &GlobalTransform,
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

        mesh_flags
    }
}

bitflags::bitflags! {
    /// Various useful flags for [`RenderMeshInstance`]s.
    #[derive(Clone, Copy)]
    pub struct RenderMeshInstanceFlags: u8 {
        /// The mesh casts shadows.
        const SHADOW_CASTER           = 1 << 0;
        /// The mesh can participate in automatic batching.
        const AUTOMATIC_BATCHING      = 1 << 1;
        /// The mesh had a transform last frame and so is eligible for TAA.
        const HAVE_PREVIOUS_TRANSFORM = 1 << 2;
    }
}

/// CPU data that the render world keeps for each entity, when *not* using GPU
/// mesh uniform building.
#[derive(Deref)]
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
#[derive(Deref)]
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
    pub current_uniform_index: u32,
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
/// From this, the [`MeshInputUniform`] and [`RenderMeshInstance`] are prepared.
pub struct RenderMeshInstanceGpuBuilder {
    /// Data that will be placed on the [`RenderMeshInstance`].
    pub shared: RenderMeshInstanceShared,
    /// The current transform.
    pub transform: Affine3,
    /// Four 16-bit unsigned normalized UV values packed into a [`UVec2`]:
    ///
    ///                         <--- MSB                   LSB --->
    ///                         +---- min v ----+ +---- min u ----+
    ///     lightmap_uv_rect.x: vvvvvvvv vvvvvvvv uuuuuuuu uuuuuuuu,
    ///                         +---- max v ----+ +---- max u ----+
    ///     lightmap_uv_rect.y: VVVVVVVV VVVVVVVV UUUUUUUU UUUUUUUU,
    ///
    /// (MSB: most significant bit; LSB: least significant bit.)
    pub lightmap_uv_rect: UVec2,
    /// Various flags.
    pub mesh_flags: MeshFlags,
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
            RenderMeshInstanceFlags::HAVE_PREVIOUS_TRANSFORM,
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
    /// Information needed when using CPU mesh uniform building.
    CpuBuilding(EntityHashMap<RenderMeshInstanceCpu>),
    /// Information needed when using GPU mesh uniform building.
    GpuBuilding(EntityHashMap<RenderMeshInstanceGpu>),
}

impl RenderMeshInstances {
    fn new(use_gpu_uniform_builder: bool) -> RenderMeshInstances {
        if use_gpu_uniform_builder {
            RenderMeshInstances::GpuBuilding(EntityHashMap::default())
        } else {
            RenderMeshInstances::CpuBuilding(EntityHashMap::default())
        }
    }

    /// Returns the ID of the mesh asset attached to the given entity, if any.
    pub(crate) fn mesh_asset_id(&self, entity: Entity) -> Option<AssetId<Mesh>> {
        match *self {
            RenderMeshInstances::CpuBuilding(ref instances) => instances
                .get(&entity)
                .map(|instance| instance.mesh_asset_id),
            RenderMeshInstances::GpuBuilding(ref instances) => instances
                .get(&entity)
                .map(|instance| instance.mesh_asset_id),
        }
    }

    /// Constructs [`RenderMeshQueueData`] for the given entity, if it has a
    /// mesh attached.
    pub fn render_mesh_queue_data(&self, entity: Entity) -> Option<RenderMeshQueueData> {
        match *self {
            RenderMeshInstances::CpuBuilding(ref instances) => {
                instances.get(&entity).map(|instance| RenderMeshQueueData {
                    shared: &instance.shared,
                    translation: instance.transforms.transform.translation,
                })
            }
            RenderMeshInstances::GpuBuilding(ref instances) => {
                instances.get(&entity).map(|instance| RenderMeshQueueData {
                    shared: &instance.shared,
                    translation: instance.translation,
                })
            }
        }
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

/// Extracts meshes from the main world into the render world, populating the
/// [`RenderMeshInstances`].
///
/// This is the variant of the system that runs when we're *not* using GPU
/// [`MeshUniform`] building.
pub fn extract_meshes_for_cpu_building(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
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
        )>,
    >,
) {
    meshes_query.par_iter().for_each(
        |(
            entity,
            view_visibility,
            transform,
            previous_transform,
            handle,
            not_shadow_receiver,
            transmitted_receiver,
            not_shadow_caster,
            no_automatic_batching,
        )| {
            if !view_visibility.get() {
                return;
            }

            let mesh_flags =
                MeshFlags::from_components(transform, not_shadow_receiver, transmitted_receiver);

            let shared = RenderMeshInstanceShared::from_components(
                previous_transform,
                handle,
                not_shadow_caster,
                no_automatic_batching,
            );

            render_mesh_instance_queues.scope(|queue| {
                let transform = transform.affine();
                queue.push((
                    entity,
                    RenderMeshInstanceCpu {
                        transforms: MeshTransforms {
                            transform: (&transform).into(),
                            previous_transform: (&previous_transform
                                .map(|t| t.0)
                                .unwrap_or(transform))
                                .into(),
                            flags: mesh_flags.bits(),
                        },
                        shared,
                    },
                ));
            });
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
        render_mesh_instances.extend(queue.drain(..));
    }
}

/// Extracts meshes from the main world into the render world and queues
/// [`MeshInputUniform`]s to be uploaded to the GPU.
///
/// This is the variant of the system that runs when we're using GPU
/// [`MeshUniform`] building.
pub fn extract_meshes_for_gpu_building(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    mut batched_instance_buffers: ResMut<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    mut render_mesh_instance_queues: Local<Parallel<Vec<(Entity, RenderMeshInstanceGpuBuilder)>>>,
    mut prev_render_mesh_instances: Local<EntityHashMap<RenderMeshInstanceGpu>>,
    meshes_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            Option<&Lightmap>,
            &Handle<Mesh>,
            Has<NotShadowReceiver>,
            Has<TransmittedShadowReceiver>,
            Has<NotShadowCaster>,
            Has<NoAutomaticBatching>,
        )>,
    >,
) {
    meshes_query.par_iter().for_each(
        |(
            entity,
            view_visibility,
            transform,
            previous_transform,
            lightmap,
            handle,
            not_shadow_receiver,
            transmitted_receiver,
            not_shadow_caster,
            no_automatic_batching,
        )| {
            if !view_visibility.get() {
                return;
            }

            let mesh_flags =
                MeshFlags::from_components(transform, not_shadow_receiver, transmitted_receiver);

            let shared = RenderMeshInstanceShared::from_components(
                previous_transform,
                handle,
                not_shadow_caster,
                no_automatic_batching,
            );

            let lightmap_uv_rect =
                lightmap::pack_lightmap_uv_rect(lightmap.map(|lightmap| lightmap.uv_rect));

            render_mesh_instance_queues.scope(|queue| {
                queue.push((
                    entity,
                    RenderMeshInstanceGpuBuilder {
                        shared,
                        transform: (&transform.affine()).into(),
                        lightmap_uv_rect,
                        mesh_flags,
                    },
                ));
            });
        },
    );

    collect_meshes_for_gpu_building(
        &mut render_mesh_instances,
        &mut batched_instance_buffers,
        &mut render_mesh_instance_queues,
        &mut prev_render_mesh_instances,
    );
}

/// Creates the [`RenderMeshInstance`]s and [`MeshInputUniform`]s when GPU mesh
/// uniforms are built.
fn collect_meshes_for_gpu_building(
    render_mesh_instances: &mut RenderMeshInstances,
    batched_instance_buffers: &mut BatchedInstanceBuffers<MeshUniform, MeshInputUniform>,
    render_mesh_instance_queues: &mut Parallel<Vec<(Entity, RenderMeshInstanceGpuBuilder)>>,
    prev_render_mesh_instances: &mut EntityHashMap<RenderMeshInstanceGpu>,
) {
    // Collect render mesh instances. Build up the uniform buffer.
    let RenderMeshInstances::GpuBuilding(ref mut render_mesh_instances) = *render_mesh_instances
    else {
        panic!(
            "`collect_render_mesh_instances_for_gpu_building` should only be called if we're \
            using GPU `MeshUniform` building"
        );
    };

    let BatchedInstanceBuffers::GpuBuilt {
        ref mut current_input_buffer,
        ref mut previous_input_buffer,
        ..
    } = *batched_instance_buffers
    else {
        unreachable!()
    };

    // Swap buffers.
    mem::swap(current_input_buffer, previous_input_buffer);
    mem::swap(render_mesh_instances, prev_render_mesh_instances);

    // Build the [`RenderMeshInstance`]s and [`MeshInputUniform`]s.
    render_mesh_instances.clear();
    for queue in render_mesh_instance_queues.iter_mut() {
        for (entity, builder) in queue.drain(..) {
            let previous_input_index = if builder
                .shared
                .flags
                .contains(RenderMeshInstanceFlags::HAVE_PREVIOUS_TRANSFORM)
            {
                prev_render_mesh_instances
                    .get(&entity)
                    .map(|render_mesh_instance| render_mesh_instance.current_uniform_index)
            } else {
                None
            };

            // Push the mesh input uniform.
            let current_uniform_index = current_input_buffer.push(MeshInputUniform {
                transform: builder.transform.to_transpose(),
                lightmap_uv_rect: builder.lightmap_uv_rect,
                flags: builder.mesh_flags.bits(),
                previous_input_index: previous_input_index.unwrap_or(!0),
            });

            // Record the [`RenderMeshInstance`].
            render_mesh_instances.insert(
                entity,
                RenderMeshInstanceGpu {
                    translation: builder.transform.translation,
                    shared: builder.shared,
                    current_uniform_index: current_uniform_index as u32,
                },
            );
        }
    }
}

#[derive(Resource, Clone)]
pub struct MeshPipeline {
    view_layouts: [MeshPipelineViewLayout; MeshPipelineViewLayoutKey::COUNT],
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
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);
        let clustered_forward_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);

        let view_layouts =
            generate_view_layouts(&render_device, clustered_forward_buffer_binding_type);

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
            view_layouts,
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
        gpu_images: &'a RenderAssets<Image>,
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
        let index = layout_key.bits() as usize;
        let layout = &self.view_layouts[index];

        #[cfg(debug_assertions)]
        if layout.texture_count > MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES {
            // Issue our own warning here because Naga's error message is a bit cryptic in this situation
            warn_once!("Too many textures in mesh pipeline view layout, this might cause us to hit `wgpu::Limits::max_sampled_textures_per_shader_stage` in some environments.");
        }

        &layout.bind_group_layout
    }
}

impl GetBatchData for MeshPipeline {
    type Param = (SRes<RenderMeshInstances>, SRes<RenderLightmaps>);
    // The material bind group ID, the mesh ID, and the lightmap ID,
    // respectively.
    type CompareData = (MaterialBindGroupId, AssetId<Mesh>, Option<AssetId<Image>>);

    type BufferData = MeshUniform;

    type BufferInputData = MeshInputUniform;

    fn get_batch_data(
        (mesh_instances, lightmaps): &SystemParamItem<Self::Param>,
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

    fn get_batch_index(
        (mesh_instances, lightmaps): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<(u32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_index` should never be called in CPU mesh uniform \
                building mode"
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
}

impl GetBinnedBatchData for MeshPipeline {
    type Param = (SRes<RenderMeshInstances>, SRes<RenderLightmaps>);

    type BufferData = MeshUniform;

    type BufferInputData = MeshInputUniform;

    fn get_batch_data(
        (mesh_instances, lightmaps): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
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

    fn get_batch_index(
        (mesh_instances, _): &SystemParamItem<Self::Param>,
        entity: Entity,
    ) -> Option<u32> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_index` should never be called in CPU mesh uniform \
                building mode"
            );
            return None;
        };

        mesh_instances
            .get(&entity)
            .map(|entity| entity.current_uniform_index)
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    /// MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct MeshPipelineKey: u32 {
        const NONE                              = 0;
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
        const MORPH_TARGETS                     = 1 << 12;
        const READS_VIEW_TRANSMISSION_TEXTURE   = 1 << 13;
        const LIGHTMAPPED                       = 1 << 14;
        const IRRADIANCE_VOLUME                 = 1 << 15;
        const BLEND_RESERVED_BITS               = Self::BLEND_MASK_BITS << Self::BLEND_SHIFT_BITS; // ← Bitmask reserving bits for the blend state
        const BLEND_OPAQUE                      = 0 << Self::BLEND_SHIFT_BITS;                   // ← Values are just sequential within the mask, and can range from 0 to 3
        const BLEND_PREMULTIPLIED_ALPHA         = 1 << Self::BLEND_SHIFT_BITS;                   //
        const BLEND_MULTIPLY                    = 2 << Self::BLEND_SHIFT_BITS;                   // ← We still have room for one more value without adding more bits
        const BLEND_ALPHA                       = 3 << Self::BLEND_SHIFT_BITS;
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        const PRIMITIVE_TOPOLOGY_RESERVED_BITS  = Self::PRIMITIVE_TOPOLOGY_MASK_BITS << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
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
        const SHADOW_FILTER_METHOD_CASTANO_13    = 1 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_JIMENEZ_14    = 2 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
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
    }
}

impl MeshPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();

    const PRIMITIVE_TOPOLOGY_MASK_BITS: u32 = 0b111;
    const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u32 =
        Self::MSAA_SHIFT_BITS - Self::PRIMITIVE_TOPOLOGY_MASK_BITS.count_ones();

    const BLEND_MASK_BITS: u32 = 0b11;
    const BLEND_SHIFT_BITS: u32 =
        Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS - Self::BLEND_MASK_BITS.count_ones();

    const TONEMAP_METHOD_MASK_BITS: u32 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u32 =
        Self::BLEND_SHIFT_BITS - Self::TONEMAP_METHOD_MASK_BITS.count_ones();

    const SHADOW_FILTER_METHOD_MASK_BITS: u32 = 0b11;
    const SHADOW_FILTER_METHOD_SHIFT_BITS: u32 =
        Self::TONEMAP_METHOD_SHIFT_BITS - Self::SHADOW_FILTER_METHOD_MASK_BITS.count_ones();

    const VIEW_PROJECTION_MASK_BITS: u32 = 0b11;
    const VIEW_PROJECTION_SHIFT_BITS: u32 =
        Self::SHADOW_FILTER_METHOD_SHIFT_BITS - Self::VIEW_PROJECTION_MASK_BITS.count_ones();

    const SCREEN_SPACE_SPECULAR_TRANSMISSION_MASK_BITS: u32 = 0b11;
    const SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS: u32 = Self::VIEW_PROJECTION_SHIFT_BITS
        - Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_MASK_BITS.count_ones();

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
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
        let primitive_topology_bits = ((primitive_topology as u32)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits)
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits = (self.bits() >> Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u32 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u32 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u32 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u32 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u32 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
        }
    }
}

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
    match (is_skinned(layout), is_morphed, is_lightmapped) {
        (true, false, _) => {
            add_skin_data();
            mesh_layouts.skinned.clone()
        }
        (true, true, _) => {
            add_skin_data();
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_skinned.clone()
        }
        (false, true, _) => {
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed.clone()
        }
        (false, false, true) => mesh_layouts.lightmapped.clone(),
        (false, false, false) => mesh_layouts.model_only.clone(),
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
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_1) {
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
        let mut is_opaque = false;
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

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED ".into());
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
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13 {
            shader_defs.push("SHADOW_FILTER_METHOD_CASTANO_13".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14 {
            shader_defs.push("SHADOW_FILTER_METHOD_JIMENEZ_14".into());
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

        let mut push_constant_ranges = Vec::with_capacity(1);
        if cfg!(all(
            feature = "webgl",
            target_arch = "wasm32",
            not(feature = "webgpu")
        )) {
            push_constant_ranges.push(PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..4,
            });
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
            push_constant_ranges,
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
                alpha_to_coverage_enabled: false,
            },
            label: Some(label),
        })
    }
}

/// Bind groups for meshes currently loaded.
#[derive(Resource, Default)]
pub struct MeshBindGroups {
    model_only: Option<BindGroup>,
    skinned: Option<BindGroup>,
    morph_targets: HashMap<AssetId<Mesh>, BindGroup>,
    lightmaps: HashMap<AssetId<Image>, BindGroup>,
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
    ) -> Option<&BindGroup> {
        match (is_skinned, morph, lightmap) {
            (_, true, _) => self.morph_targets.get(&asset_id),
            (true, false, _) => self.skinned.as_ref(),
            (false, false, Some(lightmap)) => self.lightmaps.get(&lightmap),
            (false, false, None) => self.model_only.as_ref(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_mesh_bind_group(
    meshes: Res<RenderAssets<Mesh>>,
    images: Res<RenderAssets<Image>>,
    mut groups: ResMut<MeshBindGroups>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
    mesh_uniforms: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    skins_uniform: Res<SkinUniform>,
    weights_uniform: Res<MorphUniform>,
    render_lightmaps: Res<RenderLightmaps>,
) {
    groups.reset();
    let layouts = &mesh_pipeline.mesh_layouts;
    let Some(model) = mesh_uniforms.uniform_binding() else {
        return;
    };
    groups.model_only = Some(layouts.model_only(&render_device, &model));

    let skin = skins_uniform.buffer.buffer();
    if let Some(skin) = skin {
        groups.skinned = Some(layouts.skinned(&render_device, &model, skin));
    }

    if let Some(weights) = weights_uniform.buffer.buffer() {
        for (id, gpu_mesh) in meshes.iter() {
            if let Some(targets) = gpu_mesh.morph_targets.as_ref() {
                let group = if let Some(skin) = skin.filter(|_| is_skinned(&gpu_mesh.layout)) {
                    layouts.morphed_skinned(&render_device, &model, skin, weights, targets)
                } else {
                    layouts.morphed(&render_device, &model, weights, targets)
                };
                groups.morph_targets.insert(id, group);
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
        Read<MeshViewBindGroup>,
    );
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, view_lights, view_fog, view_light_probes, mesh_view_bind_group): ROQueryItem<
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
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
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
        let skin_index = skin_indices.get(entity);
        let morph_index = morph_indices.get(entity);

        let is_skinned = skin_index.is_some();
        let is_morphed = morph_index.is_some();

        let lightmap = lightmaps
            .render_lightmaps
            .get(entity)
            .map(|render_lightmap| render_lightmap.image);

        let Some(bind_group) = bind_groups.get(mesh_asset_id, lightmap, is_skinned, is_morphed)
        else {
            error!(
                "The MeshBindGroups resource wasn't set in the render phase. \
                It should be set by the prepare_mesh_bind_group system.\n\
                This is a bevy bug! Please open an issue."
            );
            return RenderCommandResult::Failure;
        };

        let mut dynamic_offsets: [u32; 3] = Default::default();
        let mut offset_count = 0;
        if let Some(dynamic_offset) = item.dynamic_offset() {
            dynamic_offsets[offset_count] = dynamic_offset.get();
            offset_count += 1;
        }
        if let Some(skin_index) = skin_index {
            dynamic_offsets[offset_count] = skin_index.index;
            offset_count += 1;
        }
        if let Some(morph_index) = morph_index {
            dynamic_offsets[offset_count] = morph_index.index;
            offset_count += 1;
        }
        pass.set_bind_group(I, bind_group, &dynamic_offsets[0..offset_count]);

        RenderCommandResult::Success
    }
}

pub struct DrawMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawMesh {
    type Param = (SRes<RenderAssets<Mesh>>, SRes<RenderMeshInstances>);
    type ViewQuery = ();
    type ItemQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (meshes, mesh_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let mesh_instances = mesh_instances.into_inner();

        let Some(mesh_asset_id) = mesh_instances.mesh_asset_id(item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let Some(gpu_mesh) = meshes.get(mesh_asset_id) else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

        let batch_range = item.batch_range();
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            &(batch_range.start as i32).to_le_bytes(),
        );
        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, batch_range.clone());
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_mesh.vertex_count, batch_range.clone());
            }
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
