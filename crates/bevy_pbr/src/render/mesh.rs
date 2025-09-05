use crate::material_bind_groups::{MaterialBindGroupIndex, MaterialBindGroupSlot};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetId};
use bevy_camera::{
    primitives::Aabb,
    visibility::{NoFrustumCulling, RenderLayers, ViewVisibility, VisibilityRange},
    Camera, Camera3d, Projection,
};
use bevy_core_pipeline::{
    core_3d::{AlphaMask3d, Opaque3d, Transmissive3d, Transparent3d, CORE_3D_DEPTH_FORMAT},
    deferred::{AlphaMask3dDeferred, Opaque3dDeferred},
    oit::{prepare_oit_buffers, OrderIndependentTransparencySettingsOffset},
    prepass::MotionVectorPrepass,
};
use bevy_derive::{Deref, DerefMut};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    prelude::*,
    query::{QueryData, ROQueryItem},
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_image::{BevyDefault, ImageSampler, TextureFormatPixelInfo};
use bevy_light::{
    EnvironmentMapLight, IrradianceVolume, NotShadowCaster, NotShadowReceiver,
    ShadowFilteringMethod, TransmittedShadowReceiver,
};
use bevy_math::{Affine3, Rect, UVec2, Vec3, Vec4};
use bevy_mesh::{
    skinning::SkinnedMesh, BaseMeshPipelineKey, Mesh, Mesh3d, MeshTag, MeshVertexBufferLayoutRef,
    VertexAttributeDescriptor,
};
use bevy_platform::collections::{hash_map::Entry, HashMap};
use bevy_render::{
    batching::{
        gpu_preprocessing::{
            self, GpuPreprocessingSupport, IndirectBatchSet, IndirectParametersBuffers,
            IndirectParametersCpuMetadata, IndirectParametersIndexed, IndirectParametersNonIndexed,
            InstanceInputUniformBuffer, UntypedPhaseIndirectParametersBuffers,
        },
        no_gpu_preprocessing, GetBatchData, GetFullBatchData, NoAutomaticBatching,
    },
    mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
    render_asset::RenderAssets,
    render_phase::{
        BinnedRenderPhasePlugin, InputUniformIndex, PhaseItem, PhaseItemExtraIndex, RenderCommand,
        RenderCommandResult, SortedRenderPhasePlugin, TrackedRenderPass,
    },
    render_resource::*,
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    sync_world::MainEntityHashSet,
    texture::{DefaultImageSampler, GpuImage},
    view::{
        self, NoIndirectDrawing, RenderVisibilityRanges, RetainedViewEntity, ViewTarget,
        ViewUniformOffset,
    },
    Extract,
};
use bevy_shader::{load_shader_library, Shader, ShaderDefVal, ShaderSettings};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{default, Parallel, TypeIdMap};
use core::any::TypeId;
use core::mem::size_of;
use material_bind_groups::MaterialBindingId;
use tracing::{error, warn};

use self::irradiance_volume::IRRADIANCE_VOLUMES_ARE_USABLE;
use crate::{
    render::{
        morph::{
            extract_morphs, no_automatic_morph_batching, prepare_morphs, MorphIndices,
            MorphUniforms,
        },
        skin::no_automatic_skin_batching,
    },
    *,
};
use bevy_core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy_core_pipeline::prepass::{DeferredPrepass, DepthPrepass, NormalPrepass};
use bevy_core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy_ecs::component::Tick;
use bevy_ecs::system::SystemChangeTick;
use bevy_render::camera::TemporalJitter;
use bevy_render::prelude::Msaa;
use bevy_render::sync_world::{MainEntity, MainEntityHashMap};
use bevy_render::view::ExtractedView;
use bevy_render::RenderSystems::PrepareAssets;

use bytemuck::{Pod, Zeroable};
use nonmax::{NonMaxU16, NonMaxU32};
use smallvec::{smallvec, SmallVec};
use static_assertions::const_assert_eq;

/// Provides support for rendering 3D meshes.
pub struct MeshRenderPlugin {
    /// Whether we're building [`MeshUniform`]s on GPU.
    ///
    /// This requires compute shader support and so will be forcibly disabled if
    /// the platform doesn't support those.
    pub use_gpu_instance_buffer_builder: bool,
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl MeshRenderPlugin {
    /// Creates a new [`MeshRenderPlugin`] with the given debug flags.
    pub fn new(debug_flags: RenderDebugFlags) -> MeshRenderPlugin {
        MeshRenderPlugin {
            use_gpu_instance_buffer_builder: false,
            debug_flags,
        }
    }
}

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
        load_shader_library!(app, "forward_io.wgsl");
        load_shader_library!(app, "mesh_view_types.wgsl", |settings| *settings =
            ShaderSettings {
                shader_defs: vec![
                    ShaderDefVal::UInt(
                        "MAX_DIRECTIONAL_LIGHTS".into(),
                        MAX_DIRECTIONAL_LIGHTS as u32
                    ),
                    ShaderDefVal::UInt(
                        "MAX_CASCADES_PER_LIGHT".into(),
                        MAX_CASCADES_PER_LIGHT as u32,
                    )
                ]
            });
        load_shader_library!(app, "mesh_view_bindings.wgsl");
        load_shader_library!(app, "mesh_types.wgsl");
        load_shader_library!(app, "mesh_functions.wgsl");
        load_shader_library!(app, "skinning.wgsl");
        load_shader_library!(app, "morph.wgsl");
        load_shader_library!(app, "occlusion_culling.wgsl");

        embedded_asset!(app, "mesh.wgsl");

        if app.get_sub_app(RenderApp).is_none() {
            return;
        }

        app.add_systems(
            PostUpdate,
            (no_automatic_skin_batching, no_automatic_morph_batching),
        )
        .add_plugins((
            BinnedRenderPhasePlugin::<Opaque3d, MeshPipeline>::new(self.debug_flags),
            BinnedRenderPhasePlugin::<AlphaMask3d, MeshPipeline>::new(self.debug_flags),
            BinnedRenderPhasePlugin::<Shadow, MeshPipeline>::new(self.debug_flags),
            BinnedRenderPhasePlugin::<Opaque3dDeferred, MeshPipeline>::new(self.debug_flags),
            BinnedRenderPhasePlugin::<AlphaMask3dDeferred, MeshPipeline>::new(self.debug_flags),
            SortedRenderPhasePlugin::<Transmissive3d, MeshPipeline>::new(self.debug_flags),
            SortedRenderPhasePlugin::<Transparent3d, MeshPipeline>::new(self.debug_flags),
        ));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<MorphUniforms>()
                .init_resource::<MorphIndices>()
                .init_resource::<MeshCullingDataBuffer>()
                .init_resource::<RenderMaterialInstances>()
                .configure_sets(
                    ExtractSchedule,
                    MeshExtractionSystems
                        .after(view::extract_visibility_ranges)
                        .after(late_sweep_material_instances),
                )
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_skins,
                        extract_morphs,
                        gpu_preprocessing::clear_batched_gpu_instance_buffers::<MeshPipeline>
                            .before(MeshExtractionSystems),
                    ),
                )
                .add_systems(
                    Render,
                    (
                        set_mesh_motion_vector_flags.in_set(RenderSystems::PrepareMeshes),
                        prepare_skins.in_set(RenderSystems::PrepareResources),
                        prepare_morphs.in_set(RenderSystems::PrepareResources),
                        prepare_mesh_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                        prepare_mesh_view_bind_groups
                            .in_set(RenderSystems::PrepareBindGroups)
                            .after(prepare_oit_buffers),
                        no_gpu_preprocessing::clear_batched_cpu_instance_buffers::<MeshPipeline>
                            .in_set(RenderSystems::Cleanup)
                            .after(RenderSystems::Render),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        let mut mesh_bindings_shader_defs = Vec::with_capacity(1);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ViewKeyCache>()
                .init_resource::<ViewSpecializationTicks>()
                .init_resource::<GpuPreprocessingSupport>()
                .init_resource::<SkinUniforms>()
                .add_systems(
                    Render,
                    check_views_need_specialization.in_set(PrepareAssets),
                );

            let gpu_preprocessing_support =
                render_app.world().resource::<GpuPreprocessingSupport>();
            let use_gpu_instance_buffer_builder =
                self.use_gpu_instance_buffer_builder && gpu_preprocessing_support.is_available();

            let render_mesh_instances = RenderMeshInstances::new(use_gpu_instance_buffer_builder);
            render_app.insert_resource(render_mesh_instances);

            if use_gpu_instance_buffer_builder {
                render_app
                    .init_resource::<gpu_preprocessing::BatchedInstanceBuffers<
                        MeshUniform,
                        MeshInputUniform
                    >>()
                    .init_resource::<RenderMeshInstanceGpuQueues>()
                    .init_resource::<MeshesToReextractNextFrame>()
                    .add_systems(
                        ExtractSchedule,
                        extract_meshes_for_gpu_building.in_set(MeshExtractionSystems),
                    )
                    .add_systems(
                        Render,
                        (
                            gpu_preprocessing::write_batched_instance_buffers::<MeshPipeline>
                                .in_set(RenderSystems::PrepareResourcesFlush),
                            gpu_preprocessing::delete_old_work_item_buffers::<MeshPipeline>
                                .in_set(RenderSystems::PrepareResources),
                            collect_meshes_for_gpu_building
                                .in_set(RenderSystems::PrepareMeshes)
                                // This must be before
                                // `set_mesh_motion_vector_flags` so it doesn't
                                // overwrite those flags.
                                .before(set_mesh_motion_vector_flags),
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
                        extract_meshes_for_cpu_building.in_set(MeshExtractionSystems),
                    )
                    .add_systems(
                        Render,
                        no_gpu_preprocessing::write_batched_instance_buffer::<MeshPipeline>
                            .in_set(RenderSystems::PrepareResourcesFlush),
                    );
            };

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
                .init_resource::<MeshPipelineViewLayouts>()
                .init_resource::<MeshPipeline>();
        }

        // Load the mesh_bindings shader module here as it depends on runtime information about
        // whether storage buffers are supported, or the maximum uniform buffer binding size.
        load_shader_library!(app, "mesh_bindings.wgsl", move |settings| *settings =
            ShaderSettings {
                shader_defs: mesh_bindings_shader_defs.clone(),
            });
    }
}

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewKeyCache(HashMap<RetainedViewEntity, MeshPipelineKey>);

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewSpecializationTicks(HashMap<RetainedViewEntity, Tick>);

pub fn check_views_need_specialization(
    mut view_key_cache: ResMut<ViewKeyCache>,
    mut view_specialization_ticks: ResMut<ViewSpecializationTicks>,
    mut views: Query<(
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&ShadowFilteringMethod>,
        Has<ScreenSpaceAmbientOcclusion>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        Option<&Camera3d>,
        Has<TemporalJitter>,
        Option<&Projection>,
        Has<DistanceFog>,
        (
            Has<RenderViewLightProbes<EnvironmentMapLight>>,
            Has<RenderViewLightProbes<IrradianceVolume>>,
        ),
        Has<OrderIndependentTransparencySettings>,
    )>,
    ticks: SystemChangeTick,
) {
    for (
        view,
        msaa,
        tonemapping,
        dither,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
        camera_3d,
        temporal_jitter,
        projection,
        distance_fog,
        (has_environment_maps, has_irradiance_volumes),
        has_oit,
    ) in views.iter_mut()
    {
        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        if temporal_jitter {
            view_key |= MeshPipelineKey::TEMPORAL_JITTER;
        }

        if has_environment_maps {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        if has_irradiance_volumes {
            view_key |= MeshPipelineKey::IRRADIANCE_VOLUME;
        }

        if has_oit {
            view_key |= MeshPipelineKey::OIT_ENABLED;
        }

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
                Projection::Custom(_) => MeshPipelineKey::VIEW_PROJECTION_NONSTANDARD,
            };
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Gaussian => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_GAUSSIAN;
            }
            ShadowFilteringMethod::Temporal => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_TEMPORAL;
            }
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }
        if ssao {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }
        if distance_fog {
            view_key |= MeshPipelineKey::DISTANCE_FOG;
        }
        if let Some(camera_3d) = camera_3d {
            view_key |= screen_space_specular_transmission_pipeline_key(
                camera_3d.screen_space_specular_transmission_quality,
            );
        }
        if !view_key_cache
            .get_mut(&view.retained_view_entity)
            .is_some_and(|current_key| *current_key == view_key)
        {
            view_key_cache.insert(view.retained_view_entity, view_key);
            view_specialization_ticks.insert(view.retained_view_entity, ticks.this_run());
        }
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
    /// The index of this mesh's first vertex in the vertex buffer.
    ///
    /// Multiple meshes can be packed into a single vertex buffer (see
    /// [`MeshAllocator`]). This value stores the offset of the first vertex in
    /// this mesh in that buffer.
    pub first_vertex_index: u32,
    /// The current skin index, or `u32::MAX` if there's no skin.
    pub current_skin_index: u32,
    /// The material and lightmap indices, packed into 32 bits.
    ///
    /// Low 16 bits: index of the material inside the bind group data.
    /// High 16 bits: index of the lightmap in the binding array.
    pub material_and_lightmap_bind_group_slot: u32,
    /// User supplied tag to identify this mesh instance.
    pub tag: u32,
    /// Padding.
    pub pad: u32,
}

/// Information that has to be transferred from CPU to GPU in order to produce
/// the full [`MeshUniform`].
///
/// This is essentially a subset of the fields in [`MeshUniform`] above.
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Default, Debug)]
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
    /// The index of this mesh's first vertex in the vertex buffer.
    ///
    /// Multiple meshes can be packed into a single vertex buffer (see
    /// [`MeshAllocator`]). This value stores the offset of the first vertex in
    /// this mesh in that buffer.
    pub first_vertex_index: u32,
    /// The index of this mesh's first index in the index buffer, if any.
    ///
    /// Multiple meshes can be packed into a single index buffer (see
    /// [`MeshAllocator`]). This value stores the offset of the first index in
    /// this mesh in that buffer.
    ///
    /// If this mesh isn't indexed, this value is ignored.
    pub first_index_index: u32,
    /// For an indexed mesh, the number of indices that make it up; for a
    /// non-indexed mesh, the number of vertices in it.
    pub index_count: u32,
    /// The current skin index, or `u32::MAX` if there's no skin.
    pub current_skin_index: u32,
    /// The material and lightmap indices, packed into 32 bits.
    ///
    /// Low 16 bits: index of the material inside the bind group data.
    /// High 16 bits: index of the lightmap in the binding array.
    pub material_and_lightmap_bind_group_slot: u32,
    /// The number of the frame on which this [`MeshInputUniform`] was built.
    ///
    /// This is used to validate the previous transform and skin. If this
    /// [`MeshInputUniform`] wasn't updated on this frame, then we know that
    /// neither this mesh's transform nor that of its joints have been updated
    /// on this frame, and therefore the transforms of both this mesh and its
    /// joints must be identical to those for the previous frame.
    pub timestamp: u32,
    /// User supplied tag to identify this mesh instance.
    pub tag: u32,
    /// Padding.
    pub pad: u32,
}

/// Information about each mesh instance needed to cull it on GPU.
///
/// This consists of its axis-aligned bounding box (AABB).
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Default)]
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
    pub fn new(
        mesh_transforms: &MeshTransforms,
        first_vertex_index: u32,
        material_bind_group_slot: MaterialBindGroupSlot,
        maybe_lightmap: Option<(LightmapSlotIndex, Rect)>,
        current_skin_index: Option<u32>,
        tag: Option<u32>,
    ) -> Self {
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            mesh_transforms.world_from_local.inverse_transpose_3x3();
        let lightmap_bind_group_slot = match maybe_lightmap {
            None => u16::MAX,
            Some((slot_index, _)) => slot_index.into(),
        };

        Self {
            world_from_local: mesh_transforms.world_from_local.to_transpose(),
            previous_world_from_local: mesh_transforms.previous_world_from_local.to_transpose(),
            lightmap_uv_rect: pack_lightmap_uv_rect(maybe_lightmap.map(|(_, uv_rect)| uv_rect)),
            local_from_world_transpose_a,
            local_from_world_transpose_b,
            flags: mesh_transforms.flags,
            first_vertex_index,
            current_skin_index: current_skin_index.unwrap_or(u32::MAX),
            material_and_lightmap_bind_group_slot: u32::from(material_bind_group_slot)
                | ((lightmap_bind_group_slot as u32) << 16),
            tag: tag.unwrap_or(0),
            pad: 0,
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
        /// Disables frustum culling for this mesh.
        ///
        /// This corresponds to the
        /// [`bevy_render::view::visibility::NoFrustumCulling`] component.
        const NO_FRUSTUM_CULLING          = 1 << 28;
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
        no_frustum_culling: bool,
        not_shadow_receiver: bool,
        transmitted_receiver: bool,
    ) -> MeshFlags {
        let mut mesh_flags = if not_shadow_receiver {
            MeshFlags::empty()
        } else {
            MeshFlags::SHADOW_RECEIVER
        };
        if no_frustum_culling {
            mesh_flags |= MeshFlags::NO_FRUSTUM_CULLING;
        }
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
    /// A slot for the material bind group index.
    pub material_bindings_index: MaterialBindingId,
    /// Various flags.
    pub flags: RenderMeshInstanceFlags,
    /// Index of the slab that the lightmap resides in, if a lightmap is
    /// present.
    pub lightmap_slab_index: Option<LightmapSlabIndex>,
    /// User supplied tag to identify this mesh instance.
    pub tag: u32,
    /// Render layers that this mesh instance belongs to.
    pub render_layers: Option<RenderLayers>,
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
    /// [`MeshCullingData`], so that we don't waste space when GPU
    /// culling is disabled.
    CpuCulling {
        /// Stores GPU data for each entity that became visible or changed in
        /// such a way that necessitates updating the [`MeshInputUniform`] (e.g.
        /// changed transform).
        changed: Vec<(MainEntity, RenderMeshInstanceGpuBuilder)>,
        /// Stores the IDs of entities that became invisible this frame.
        removed: Vec<MainEntity>,
    },
    /// The version of [`RenderMeshInstanceGpuQueue`] that contains the
    /// [`MeshCullingData`], used when any view has GPU culling
    /// enabled.
    GpuCulling {
        /// Stores GPU data for each entity that became visible or changed in
        /// such a way that necessitates updating the [`MeshInputUniform`] (e.g.
        /// changed transform).
        changed: Vec<(MainEntity, RenderMeshInstanceGpuBuilder, MeshCullingData)>,
        /// Stores the IDs of entities that became invisible this frame.
        removed: Vec<MainEntity>,
    },
}

/// The per-thread queues containing mesh instances, populated during the
/// extract phase.
///
/// These are filled in [`extract_meshes_for_gpu_building`] and consumed in
/// [`collect_meshes_for_gpu_building`].
#[derive(Resource, Default, Deref, DerefMut)]
pub struct RenderMeshInstanceGpuQueues(Parallel<RenderMeshInstanceGpuQueue>);

/// Holds a list of meshes that couldn't be extracted this frame because their
/// materials weren't prepared yet.
///
/// On subsequent frames, we try to reextract those meshes.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct MeshesToReextractNextFrame(MainEntityHashSet);

impl RenderMeshInstanceShared {
    /// A gpu builder will provide the mesh instance id
    /// during [`RenderMeshInstanceGpuBuilder::update`].
    fn for_gpu_building(
        previous_transform: Option<&PreviousGlobalTransform>,
        mesh: &Mesh3d,
        tag: Option<&MeshTag>,
        not_shadow_caster: bool,
        no_automatic_batching: bool,
        render_layers: Option<&RenderLayers>,
    ) -> Self {
        Self::for_cpu_building(
            previous_transform,
            mesh,
            tag,
            default(),
            not_shadow_caster,
            no_automatic_batching,
            render_layers,
        )
    }

    /// The cpu builder does not have an equivalent [`RenderMeshInstanceGpuBuilder::update`].
    fn for_cpu_building(
        previous_transform: Option<&PreviousGlobalTransform>,
        mesh: &Mesh3d,
        tag: Option<&MeshTag>,
        material_bindings_index: MaterialBindingId,
        not_shadow_caster: bool,
        no_automatic_batching: bool,
        render_layers: Option<&RenderLayers>,
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
            mesh_asset_id: mesh.id(),
            flags: mesh_instance_flags,
            material_bindings_index,
            lightmap_slab_index: None,
            tag: tag.map_or(0, |i| **i),
            render_layers: render_layers.cloned(),
        }
    }

    /// Returns true if this entity is eligible to participate in automatic
    /// batching.
    #[inline]
    pub fn should_batch(&self) -> bool {
        self.flags
            .contains(RenderMeshInstanceFlags::AUTOMATIC_BATCHING)
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
pub struct RenderMeshInstancesCpu(MainEntityHashMap<RenderMeshInstanceCpu>);

/// Information that the render world keeps about each entity that contains a
/// mesh, when using GPU mesh instance data building.
#[derive(Default, Deref, DerefMut)]
pub struct RenderMeshInstancesGpu(MainEntityHashMap<RenderMeshInstanceGpu>);

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
    pub fn mesh_asset_id(&self, entity: MainEntity) -> Option<AssetId<Mesh>> {
        match *self {
            RenderMeshInstances::CpuBuilding(ref instances) => instances.mesh_asset_id(entity),
            RenderMeshInstances::GpuBuilding(ref instances) => instances.mesh_asset_id(entity),
        }
    }

    /// Constructs [`RenderMeshQueueData`] for the given entity, if it has a
    /// mesh attached.
    pub fn render_mesh_queue_data(&self, entity: MainEntity) -> Option<RenderMeshQueueData<'_>> {
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
    fn insert_mesh_instance_flags(&mut self, entity: MainEntity, flags: RenderMeshInstanceFlags) {
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
    fn mesh_asset_id(&self, entity: MainEntity) -> Option<AssetId<Mesh>> {
        self.get(&entity)
            .map(|render_mesh_instance| render_mesh_instance.mesh_asset_id)
    }

    fn render_mesh_queue_data(&self, entity: MainEntity) -> Option<RenderMeshQueueData<'_>> {
        self.get(&entity)
            .map(|render_mesh_instance| RenderMeshQueueData {
                shared: &render_mesh_instance.shared,
                translation: render_mesh_instance.transforms.world_from_local.translation,
                current_uniform_index: InputUniformIndex::default(),
            })
    }

    /// Inserts the given flags into the render mesh instance data for the given
    /// mesh.
    fn insert_mesh_instance_flags(&mut self, entity: MainEntity, flags: RenderMeshInstanceFlags) {
        if let Some(instance) = self.get_mut(&entity) {
            instance.flags.insert(flags);
        }
    }
}

impl RenderMeshInstancesGpu {
    fn mesh_asset_id(&self, entity: MainEntity) -> Option<AssetId<Mesh>> {
        self.get(&entity)
            .map(|render_mesh_instance| render_mesh_instance.mesh_asset_id)
    }

    fn render_mesh_queue_data(&self, entity: MainEntity) -> Option<RenderMeshQueueData<'_>> {
        self.get(&entity)
            .map(|render_mesh_instance| RenderMeshQueueData {
                shared: &render_mesh_instance.shared,
                translation: render_mesh_instance.translation,
                current_uniform_index: InputUniformIndex(
                    render_mesh_instance.current_uniform_index.into(),
                ),
            })
    }

    /// Inserts the given flags into the render mesh instance data for the given
    /// mesh.
    fn insert_mesh_instance_flags(&mut self, entity: MainEntity, flags: RenderMeshInstanceFlags) {
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
            (true, RenderMeshInstanceGpuQueue::GpuCulling { changed, removed }) => {
                changed.clear();
                removed.clear();
            }
            (true, _) => {
                *self = RenderMeshInstanceGpuQueue::GpuCulling {
                    changed: vec![],
                    removed: vec![],
                }
            }
            (false, RenderMeshInstanceGpuQueue::CpuCulling { changed, removed }) => {
                changed.clear();
                removed.clear();
            }
            (false, _) => {
                *self = RenderMeshInstanceGpuQueue::CpuCulling {
                    changed: vec![],
                    removed: vec![],
                }
            }
        }
    }

    /// Adds a new mesh to this queue.
    fn push(
        &mut self,
        entity: MainEntity,
        instance_builder: RenderMeshInstanceGpuBuilder,
        culling_data_builder: Option<MeshCullingData>,
    ) {
        match (&mut *self, culling_data_builder) {
            (
                &mut RenderMeshInstanceGpuQueue::CpuCulling {
                    changed: ref mut queue,
                    ..
                },
                None,
            ) => {
                queue.push((entity, instance_builder));
            }
            (
                &mut RenderMeshInstanceGpuQueue::GpuCulling {
                    changed: ref mut queue,
                    ..
                },
                Some(culling_data_builder),
            ) => {
                queue.push((entity, instance_builder, culling_data_builder));
            }
            (_, None) => {
                *self = RenderMeshInstanceGpuQueue::CpuCulling {
                    changed: vec![(entity, instance_builder)],
                    removed: vec![],
                };
            }
            (_, Some(culling_data_builder)) => {
                *self = RenderMeshInstanceGpuQueue::GpuCulling {
                    changed: vec![(entity, instance_builder, culling_data_builder)],
                    removed: vec![],
                };
            }
        }
    }

    /// Adds the given entity to the `removed` list, queuing it for removal.
    ///
    /// The `gpu_culling` parameter specifies whether GPU culling is enabled.
    fn remove(&mut self, entity: MainEntity, gpu_culling: bool) {
        match (&mut *self, gpu_culling) {
            (RenderMeshInstanceGpuQueue::None, false) => {
                *self = RenderMeshInstanceGpuQueue::CpuCulling {
                    changed: vec![],
                    removed: vec![entity],
                }
            }
            (RenderMeshInstanceGpuQueue::None, true) => {
                *self = RenderMeshInstanceGpuQueue::GpuCulling {
                    changed: vec![],
                    removed: vec![entity],
                }
            }
            (RenderMeshInstanceGpuQueue::CpuCulling { removed, .. }, _)
            | (RenderMeshInstanceGpuQueue::GpuCulling { removed, .. }, _) => {
                removed.push(entity);
            }
        }
    }
}

impl RenderMeshInstanceGpuBuilder {
    /// Flushes this mesh instance to the [`RenderMeshInstanceGpu`] and
    /// [`MeshInputUniform`] tables, replacing the existing entry if applicable.
    fn update(
        mut self,
        entity: MainEntity,
        render_mesh_instances: &mut MainEntityHashMap<RenderMeshInstanceGpu>,
        current_input_buffer: &mut InstanceInputUniformBuffer<MeshInputUniform>,
        previous_input_buffer: &mut InstanceInputUniformBuffer<MeshInputUniform>,
        mesh_allocator: &MeshAllocator,
        mesh_material_ids: &RenderMaterialInstances,
        render_material_bindings: &RenderMaterialBindings,
        render_lightmaps: &RenderLightmaps,
        skin_uniforms: &SkinUniforms,
        timestamp: FrameCount,
        meshes_to_reextract_next_frame: &mut MeshesToReextractNextFrame,
    ) -> Option<u32> {
        let (first_vertex_index, vertex_count) =
            match mesh_allocator.mesh_vertex_slice(&self.shared.mesh_asset_id) {
                Some(mesh_vertex_slice) => (
                    mesh_vertex_slice.range.start,
                    mesh_vertex_slice.range.end - mesh_vertex_slice.range.start,
                ),
                None => (0, 0),
            };
        let (mesh_is_indexed, first_index_index, index_count) =
            match mesh_allocator.mesh_index_slice(&self.shared.mesh_asset_id) {
                Some(mesh_index_slice) => (
                    true,
                    mesh_index_slice.range.start,
                    mesh_index_slice.range.end - mesh_index_slice.range.start,
                ),
                None => (false, 0, 0),
            };
        let current_skin_index = match skin_uniforms.skin_byte_offset(entity) {
            Some(skin_index) => skin_index.index(),
            None => u32::MAX,
        };

        // Look up the material index. If we couldn't fetch the material index,
        // then the material hasn't been prepared yet, perhaps because it hasn't
        // yet loaded. In that case, add the mesh to
        // `meshes_to_reextract_next_frame` and bail.
        let mesh_material = mesh_material_ids.mesh_material(entity);
        let mesh_material_binding_id = if mesh_material != DUMMY_MESH_MATERIAL.untyped() {
            match render_material_bindings.get(&mesh_material) {
                Some(binding_id) => *binding_id,
                None => {
                    meshes_to_reextract_next_frame.insert(entity);
                    return None;
                }
            }
        } else {
            // Use a dummy material binding ID.
            MaterialBindingId::default()
        };
        self.shared.material_bindings_index = mesh_material_binding_id;

        let lightmap_slot = match render_lightmaps.render_lightmaps.get(&entity) {
            Some(render_lightmap) => u16::from(*render_lightmap.slot_index),
            None => u16::MAX,
        };
        let lightmap_slab_index = render_lightmaps
            .render_lightmaps
            .get(&entity)
            .map(|lightmap| lightmap.slab_index);
        self.shared.lightmap_slab_index = lightmap_slab_index;

        // Create the mesh input uniform.
        let mut mesh_input_uniform = MeshInputUniform {
            world_from_local: self.world_from_local.to_transpose(),
            lightmap_uv_rect: self.lightmap_uv_rect,
            flags: self.mesh_flags.bits(),
            previous_input_index: u32::MAX,
            timestamp: timestamp.0,
            first_vertex_index,
            first_index_index,
            index_count: if mesh_is_indexed {
                index_count
            } else {
                vertex_count
            },
            current_skin_index,
            material_and_lightmap_bind_group_slot: u32::from(
                self.shared.material_bindings_index.slot,
            ) | ((lightmap_slot as u32) << 16),
            tag: self.shared.tag,
            pad: 0,
        };

        // Did the last frame contain this entity as well?
        let current_uniform_index;
        match render_mesh_instances.entry(entity) {
            Entry::Occupied(mut occupied_entry) => {
                // Yes, it did. Replace its entry with the new one.

                // Reserve a slot.
                current_uniform_index = u32::from(occupied_entry.get_mut().current_uniform_index);

                // Save the old mesh input uniform. The mesh preprocessing
                // shader will need it to compute motion vectors.
                let previous_mesh_input_uniform =
                    current_input_buffer.get_unchecked(current_uniform_index);
                let previous_input_index = previous_input_buffer.add(previous_mesh_input_uniform);
                mesh_input_uniform.previous_input_index = previous_input_index;

                // Write in the new mesh input uniform.
                current_input_buffer.set(current_uniform_index, mesh_input_uniform);

                occupied_entry.replace_entry_with(|_, _| {
                    Some(RenderMeshInstanceGpu {
                        translation: self.world_from_local.translation,
                        shared: self.shared,
                        current_uniform_index: NonMaxU32::new(current_uniform_index)
                            .unwrap_or_default(),
                    })
                });
            }

            Entry::Vacant(vacant_entry) => {
                // No, this is a new entity. Push its data on to the buffer.
                current_uniform_index = current_input_buffer.add(mesh_input_uniform);

                vacant_entry.insert(RenderMeshInstanceGpu {
                    translation: self.world_from_local.translation,
                    shared: self.shared,
                    current_uniform_index: NonMaxU32::new(current_uniform_index)
                        .unwrap_or_default(),
                });
            }
        }

        Some(current_uniform_index)
    }
}

/// Removes a [`MeshInputUniform`] corresponding to an entity that became
/// invisible from the buffer.
fn remove_mesh_input_uniform(
    entity: MainEntity,
    render_mesh_instances: &mut MainEntityHashMap<RenderMeshInstanceGpu>,
    current_input_buffer: &mut InstanceInputUniformBuffer<MeshInputUniform>,
) -> Option<u32> {
    // Remove the uniform data.
    let removed_render_mesh_instance = render_mesh_instances.remove(&entity)?;

    let removed_uniform_index = removed_render_mesh_instance.current_uniform_index.get();
    current_input_buffer.remove(removed_uniform_index);
    Some(removed_uniform_index)
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
    /// [`MeshCullingDataBuffer`], replacing the existing entry if applicable.
    fn update(
        &self,
        mesh_culling_data_buffer: &mut MeshCullingDataBuffer,
        instance_data_index: usize,
    ) {
        while mesh_culling_data_buffer.len() < instance_data_index + 1 {
            mesh_culling_data_buffer.push(MeshCullingData::default());
        }
        mesh_culling_data_buffer.values_mut()[instance_data_index] = *self;
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
    /// The index of the [`MeshInputUniform`] in the GPU buffer for this mesh
    /// instance.
    pub current_uniform_index: InputUniformIndex,
}

/// A [`SystemSet`] that encompasses both [`extract_meshes_for_cpu_building`]
/// and [`extract_meshes_for_gpu_building`].
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct MeshExtractionSystems;

/// Deprecated alias for [`MeshExtractionSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `MeshExtractionSystems`.")]
pub type ExtractMeshesSet = MeshExtractionSystems;

/// Extracts meshes from the main world into the render world, populating the
/// [`RenderMeshInstances`].
///
/// This is the variant of the system that runs when we're *not* using GPU
/// [`MeshUniform`] building.
pub fn extract_meshes_for_cpu_building(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    mesh_material_ids: Res<RenderMaterialInstances>,
    render_material_bindings: Res<RenderMaterialBindings>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    mut render_mesh_instance_queues: Local<Parallel<Vec<(Entity, RenderMeshInstanceCpu)>>>,
    meshes_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            &Mesh3d,
            Option<&MeshTag>,
            Has<NoFrustumCulling>,
            Has<NotShadowReceiver>,
            Has<TransmittedShadowReceiver>,
            Has<NotShadowCaster>,
            Has<NoAutomaticBatching>,
            Has<VisibilityRange>,
            Option<&RenderLayers>,
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
            mesh,
            tag,
            no_frustum_culling,
            not_shadow_receiver,
            transmitted_receiver,
            not_shadow_caster,
            no_automatic_batching,
            visibility_range,
            render_layers,
        )| {
            if !view_visibility.get() {
                return;
            }

            let mut lod_index = None;
            if visibility_range {
                lod_index = render_visibility_ranges.lod_index_for_entity(entity.into());
            }

            let mesh_flags = MeshFlags::from_components(
                transform,
                lod_index,
                no_frustum_culling,
                not_shadow_receiver,
                transmitted_receiver,
            );

            let mesh_material = mesh_material_ids.mesh_material(MainEntity::from(entity));

            let material_bindings_index = render_material_bindings
                .get(&mesh_material)
                .copied()
                .unwrap_or_default();

            let shared = RenderMeshInstanceShared::for_cpu_building(
                previous_transform,
                mesh,
                tag,
                material_bindings_index,
                not_shadow_caster,
                no_automatic_batching,
                render_layers,
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
            render_mesh_instances.insert(entity.into(), render_mesh_instance);
        }
    }
}

/// All the data that we need from a mesh in the main world.
type GpuMeshExtractionQuery = (
    Entity,
    Read<ViewVisibility>,
    Read<GlobalTransform>,
    Option<Read<PreviousGlobalTransform>>,
    Option<Read<Lightmap>>,
    Option<Read<Aabb>>,
    Read<Mesh3d>,
    Option<Read<MeshTag>>,
    Has<NoFrustumCulling>,
    Has<NotShadowReceiver>,
    Has<TransmittedShadowReceiver>,
    Has<NotShadowCaster>,
    Has<NoAutomaticBatching>,
    Has<VisibilityRange>,
    Option<Read<RenderLayers>>,
);

/// Extracts meshes from the main world into the render world and queues
/// [`MeshInputUniform`]s to be uploaded to the GPU.
///
/// This is optimized to only look at entities that have changed since the last
/// frame.
///
/// This is the variant of the system that runs when we're using GPU
/// [`MeshUniform`] building.
pub fn extract_meshes_for_gpu_building(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    mut render_mesh_instance_queues: ResMut<RenderMeshInstanceGpuQueues>,
    changed_meshes_query: Extract<
        Query<
            GpuMeshExtractionQuery,
            Or<(
                Changed<ViewVisibility>,
                Changed<GlobalTransform>,
                Changed<PreviousGlobalTransform>,
                Changed<Lightmap>,
                Changed<Aabb>,
                Changed<Mesh3d>,
                Changed<NoFrustumCulling>,
                Changed<NotShadowReceiver>,
                Changed<TransmittedShadowReceiver>,
                Changed<NotShadowCaster>,
                Changed<NoAutomaticBatching>,
                Changed<VisibilityRange>,
                Changed<SkinnedMesh>,
            )>,
        >,
    >,
    all_meshes_query: Extract<Query<GpuMeshExtractionQuery>>,
    mut removed_meshes_query: Extract<RemovedComponents<Mesh3d>>,
    gpu_culling_query: Extract<Query<(), (With<Camera>, Without<NoIndirectDrawing>)>>,
    meshes_to_reextract_next_frame: ResMut<MeshesToReextractNextFrame>,
) {
    let any_gpu_culling = !gpu_culling_query.is_empty();

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

    // Find all meshes that have changed, and record information needed to
    // construct the `MeshInputUniform` for them.
    changed_meshes_query.par_iter().for_each_init(
        || render_mesh_instance_queues.borrow_local_mut(),
        |queue, query_row| {
            extract_mesh_for_gpu_building(
                query_row,
                &render_visibility_ranges,
                render_mesh_instances,
                queue,
                any_gpu_culling,
            );
        },
    );

    // Process materials that `collect_meshes_for_gpu_building` marked as
    // needing to be reextracted. This will happen when we extracted a mesh on
    // some previous frame, but its material hadn't been prepared yet, perhaps
    // because the material hadn't yet been loaded. We reextract such materials
    // on subsequent frames so that `collect_meshes_for_gpu_building` will check
    // to see if their materials have been prepared.
    let mut queue = render_mesh_instance_queues.borrow_local_mut();
    for &mesh_entity in &**meshes_to_reextract_next_frame {
        if let Ok(query_row) = all_meshes_query.get(*mesh_entity) {
            extract_mesh_for_gpu_building(
                query_row,
                &render_visibility_ranges,
                render_mesh_instances,
                &mut queue,
                any_gpu_culling,
            );
        }
    }

    // Also record info about each mesh that became invisible.
    for entity in removed_meshes_query.read() {
        // Only queue a mesh for removal if we didn't pick it up above.
        // It's possible that a necessary component was removed and re-added in
        // the same frame.
        let entity = MainEntity::from(entity);
        if !changed_meshes_query.contains(*entity)
            && !meshes_to_reextract_next_frame.contains(&entity)
        {
            queue.remove(entity, any_gpu_culling);
        }
    }
}

fn extract_mesh_for_gpu_building(
    (
        entity,
        view_visibility,
        transform,
        previous_transform,
        lightmap,
        aabb,
        mesh,
        tag,
        no_frustum_culling,
        not_shadow_receiver,
        transmitted_receiver,
        not_shadow_caster,
        no_automatic_batching,
        visibility_range,
        render_layers,
    ): <GpuMeshExtractionQuery as QueryData>::Item<'_, '_>,
    render_visibility_ranges: &RenderVisibilityRanges,
    render_mesh_instances: &RenderMeshInstancesGpu,
    queue: &mut RenderMeshInstanceGpuQueue,
    any_gpu_culling: bool,
) {
    if !view_visibility.get() {
        queue.remove(entity.into(), any_gpu_culling);
        return;
    }

    let mut lod_index = None;
    if visibility_range {
        lod_index = render_visibility_ranges.lod_index_for_entity(entity.into());
    }

    let mesh_flags = MeshFlags::from_components(
        transform,
        lod_index,
        no_frustum_culling,
        not_shadow_receiver,
        transmitted_receiver,
    );

    let shared = RenderMeshInstanceShared::for_gpu_building(
        previous_transform,
        mesh,
        tag,
        not_shadow_caster,
        no_automatic_batching,
        render_layers,
    );

    let lightmap_uv_rect = pack_lightmap_uv_rect(lightmap.map(|lightmap| lightmap.uv_rect));

    let gpu_mesh_culling_data = any_gpu_culling.then(|| MeshCullingData::new(aabb));

    let previous_input_index = if shared
        .flags
        .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_TRANSFORM)
    {
        render_mesh_instances
            .get(&MainEntity::from(entity))
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

    queue.push(
        entity.into(),
        gpu_mesh_instance_builder,
        gpu_mesh_culling_data,
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
pub(crate) fn set_mesh_motion_vector_flags(
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    skin_uniforms: Res<SkinUniforms>,
    morph_indices: Res<MorphIndices>,
) {
    for &entity in skin_uniforms.all_skins() {
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
pub fn collect_meshes_for_gpu_building(
    render_mesh_instances: ResMut<RenderMeshInstances>,
    batched_instance_buffers: ResMut<
        gpu_preprocessing::BatchedInstanceBuffers<MeshUniform, MeshInputUniform>,
    >,
    mut mesh_culling_data_buffer: ResMut<MeshCullingDataBuffer>,
    mut render_mesh_instance_queues: ResMut<RenderMeshInstanceGpuQueues>,
    mesh_allocator: Res<MeshAllocator>,
    mesh_material_ids: Res<RenderMaterialInstances>,
    render_material_bindings: Res<RenderMaterialBindings>,
    render_lightmaps: Res<RenderLightmaps>,
    skin_uniforms: Res<SkinUniforms>,
    frame_count: Res<FrameCount>,
    mut meshes_to_reextract_next_frame: ResMut<MeshesToReextractNextFrame>,
) {
    let RenderMeshInstances::GpuBuilding(render_mesh_instances) =
        render_mesh_instances.into_inner()
    else {
        return;
    };

    // We're going to rebuild `meshes_to_reextract_next_frame`.
    meshes_to_reextract_next_frame.clear();

    // Collect render mesh instances. Build up the uniform buffer.
    let gpu_preprocessing::BatchedInstanceBuffers {
        current_input_buffer,
        previous_input_buffer,
        ..
    } = batched_instance_buffers.into_inner();

    previous_input_buffer.clear();

    // Build the [`RenderMeshInstance`]s and [`MeshInputUniform`]s.

    for queue in render_mesh_instance_queues.iter_mut() {
        match *queue {
            RenderMeshInstanceGpuQueue::None => {
                // This can only happen if the queue is empty.
            }

            RenderMeshInstanceGpuQueue::CpuCulling {
                ref mut changed,
                ref mut removed,
            } => {
                for (entity, mesh_instance_builder) in changed.drain(..) {
                    mesh_instance_builder.update(
                        entity,
                        &mut *render_mesh_instances,
                        current_input_buffer,
                        previous_input_buffer,
                        &mesh_allocator,
                        &mesh_material_ids,
                        &render_material_bindings,
                        &render_lightmaps,
                        &skin_uniforms,
                        *frame_count,
                        &mut meshes_to_reextract_next_frame,
                    );
                }

                for entity in removed.drain(..) {
                    remove_mesh_input_uniform(
                        entity,
                        &mut *render_mesh_instances,
                        current_input_buffer,
                    );
                }
            }

            RenderMeshInstanceGpuQueue::GpuCulling {
                ref mut changed,
                ref mut removed,
            } => {
                for (entity, mesh_instance_builder, mesh_culling_builder) in changed.drain(..) {
                    let Some(instance_data_index) = mesh_instance_builder.update(
                        entity,
                        &mut *render_mesh_instances,
                        current_input_buffer,
                        previous_input_buffer,
                        &mesh_allocator,
                        &mesh_material_ids,
                        &render_material_bindings,
                        &render_lightmaps,
                        &skin_uniforms,
                        *frame_count,
                        &mut meshes_to_reextract_next_frame,
                    ) else {
                        continue;
                    };
                    mesh_culling_builder
                        .update(&mut mesh_culling_data_buffer, instance_data_index as usize);
                }

                for entity in removed.drain(..) {
                    remove_mesh_input_uniform(
                        entity,
                        &mut *render_mesh_instances,
                        current_input_buffer,
                    );
                }
            }
        }
    }

    // Buffers can't be empty. Make sure there's something in the previous input buffer.
    previous_input_buffer.ensure_nonempty();
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
    /// The shader asset handle.
    pub shader: Handle<Shader>,
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

    /// Whether clustered decals are usable on the current render device.
    pub clustered_decals_are_usable: bool,

    /// Whether skins will use uniform buffers on account of storage buffers
    /// being unavailable on this platform.
    pub skins_use_uniform_buffers: bool,
}

impl FromWorld for MeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = load_embedded_asset!(world, "mesh.wgsl");
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<RenderAdapter>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
            Res<MeshPipelineViewLayouts>,
        )> = SystemState::new(world);
        let (render_device, render_adapter, default_sampler, render_queue, view_layouts) =
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

            if let Ok(format_size) = image.texture_descriptor.format.pixel_size() {
                render_queue.write_texture(
                    texture.as_image_copy(),
                    image.data.as_ref().expect("Image was created without data"),
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(image.width() * format_size as u32),
                        rows_per_image: None,
                    },
                    image.texture_descriptor.size,
                );
            }

            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.texture_descriptor.size,
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        MeshPipeline {
            view_layouts: view_layouts.clone(),
            clustered_forward_buffer_binding_type,
            dummy_white_gpu_image,
            mesh_layouts: MeshLayouts::new(&render_device, &render_adapter),
            shader,
            per_object_buffer_batch_size: GpuArrayBuffer::<MeshUniform>::batch_size(&render_device),
            binding_arrays_are_usable: binding_arrays_are_usable(&render_device, &render_adapter),
            clustered_decals_are_usable: decal::clustered::clustered_decals_are_usable(
                &render_device,
                &render_adapter,
            ),
            skins_use_uniform_buffers: skins_use_uniform_buffers(&render_device),
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

    pub fn get_view_layout(
        &self,
        layout_key: MeshPipelineViewLayoutKey,
    ) -> &MeshPipelineViewLayout {
        self.view_layouts.get_view_layout(layout_key)
    }
}

impl GetBatchData for MeshPipeline {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<RenderLightmaps>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
        SRes<SkinUniforms>,
    );
    // The material bind group ID, the mesh ID, and the lightmap ID,
    // respectively.
    type CompareData = (
        MaterialBindGroupIndex,
        AssetId<Mesh>,
        Option<LightmapSlabIndex>,
    );

    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, lightmaps, _, mesh_allocator, skin_uniforms): &SystemParamItem<
            Self::Param,
        >,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };
        let maybe_lightmap = lightmaps.render_lightmaps.get(&main_entity);

        let current_skin_index = skin_uniforms.skin_index(main_entity);
        let material_bind_group_index = mesh_instance.material_bindings_index;

        Some((
            MeshUniform::new(
                &mesh_instance.transforms,
                first_vertex_index,
                material_bind_group_index.slot,
                maybe_lightmap.map(|lightmap| (lightmap.slot_index, lightmap.uv_rect)),
                current_skin_index,
                Some(mesh_instance.tag),
            ),
            mesh_instance.should_batch().then_some((
                material_bind_group_index.group,
                mesh_instance.mesh_asset_id,
                maybe_lightmap.map(|lightmap| lightmap.slab_index),
            )),
        ))
    }
}

impl GetFullBatchData for MeshPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (mesh_instances, lightmaps, _, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_index_and_compare_data` should never be called in CPU mesh uniform building \
                mode"
            );
            return None;
        };

        let mesh_instance = mesh_instances.get(&main_entity)?;
        let maybe_lightmap = lightmaps.render_lightmaps.get(&main_entity);

        Some((
            mesh_instance.current_uniform_index,
            mesh_instance.should_batch().then_some((
                mesh_instance.material_bindings_index.group,
                mesh_instance.mesh_asset_id,
                maybe_lightmap.map(|lightmap| lightmap.slab_index),
            )),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, lightmaps, _, mesh_allocator, skin_uniforms): &SystemParamItem<
            Self::Param,
        >,
        main_entity: MainEntity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };
        let maybe_lightmap = lightmaps.render_lightmaps.get(&main_entity);

        let current_skin_index = skin_uniforms.skin_index(main_entity);

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            first_vertex_index,
            mesh_instance.material_bindings_index.slot,
            maybe_lightmap.map(|lightmap| (lightmap.slot_index, lightmap.uv_rect)),
            current_skin_index,
            Some(mesh_instance.tag),
        ))
    }

    fn get_binned_index(
        (mesh_instances, _, _, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
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
            .get(&main_entity)
            .map(|entity| entity.current_uniform_index)
    }

    fn write_batch_indirect_parameters_metadata(
        indexed: bool,
        base_output_index: u32,
        batch_set_index: Option<NonMaxU32>,
        phase_indirect_parameters_buffers: &mut UntypedPhaseIndirectParametersBuffers,
        indirect_parameters_offset: u32,
    ) {
        let indirect_parameters = IndirectParametersCpuMetadata {
            base_output_index,
            batch_set_index: match batch_set_index {
                Some(batch_set_index) => u32::from(batch_set_index),
                None => !0,
            },
        };

        if indexed {
            phase_indirect_parameters_buffers
                .indexed
                .set(indirect_parameters_offset, indirect_parameters);
        } else {
            phase_indirect_parameters_buffers
                .non_indexed
                .set(indirect_parameters_offset, indirect_parameters);
        }
    }
}

bitflags::bitflags! {
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
        const UNCLIPPED_DEPTH_ORTHO             = 1 << 10; // Disables depth clipping for use with directional light shadow views
                                                            // Emulated via fragment shader depth on hardware that doesn't support it natively
                                                            // See: https://www.w3.org/TR/webgpu/#depth-clipping and https://therealmjp.github.io/posts/shadow-maps/#disabling-z-clipping
        const TEMPORAL_JITTER                   = 1 << 11;
        const READS_VIEW_TRANSMISSION_TEXTURE   = 1 << 12;
        const LIGHTMAPPED                       = 1 << 13;
        const LIGHTMAP_BICUBIC_SAMPLING         = 1 << 14;
        const IRRADIANCE_VOLUME                 = 1 << 15;
        const VISIBILITY_RANGE_DITHER           = 1 << 16;
        const SCREEN_SPACE_REFLECTIONS          = 1 << 17;
        const HAS_PREVIOUS_SKIN                 = 1 << 18;
        const HAS_PREVIOUS_MORPH                = 1 << 19;
        const OIT_ENABLED                       = 1 << 20;
        const DISTANCE_FOG                      = 1 << 21;
        const LAST_FLAG                         = Self::DISTANCE_FOG.bits();

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
        const TONEMAP_METHOD_TONY_MC_MAPFACE    = 6 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_BLENDER_FILMIC     = 7 << Self::TONEMAP_METHOD_SHIFT_BITS;
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
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_LOW    = 0 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_MEDIUM = 1 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_HIGH   = 2 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_ULTRA  = 3 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
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
    skins_use_uniform_buffers: bool,
) -> BindGroupLayout {
    let is_morphed = key.intersects(MeshPipelineKey::MORPH_TARGETS);
    let is_lightmapped = key.intersects(MeshPipelineKey::LIGHTMAPPED);
    let motion_vector_prepass = key.intersects(MeshPipelineKey::MOTION_VECTOR_PREPASS);

    if skins_use_uniform_buffers {
        shader_defs.push("SKINS_USE_UNIFORM_BUFFERS".into());
    }

    let mut add_skin_data = || {
        shader_defs.push("SKINNED".into());
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(offset));
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(offset + 1));
    };

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
        if cfg!(feature = "pbr_anisotropy_texture") {
            shader_defs.push("PBR_ANISOTROPY_TEXTURE_SUPPORTED".into());
        }
        if cfg!(feature = "pbr_specular_textures") {
            shader_defs.push("PBR_SPECULAR_TEXTURES_SUPPORTED".into());
        }

        let bind_group_layout = self.get_view_layout(key.into());
        let mut bind_group_layout = vec![
            bind_group_layout.main_layout.clone(),
            bind_group_layout.binding_array_layout.clone(),
        ];

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
            self.skins_use_uniform_buffers,
        ));

        if key.contains(MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION) {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        let (label, blend, depth_write_enabled);
        let pass = key.intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
        let (mut is_opaque, mut alpha_to_coverage_enabled) = (false, false);
        if key.contains(MeshPipelineKey::OIT_ENABLED) && pass == MeshPipelineKey::BLEND_ALPHA {
            label = "oit_mesh_pipeline".into();
            // TODO tail blending would need alpha blending
            blend = None;
            shader_defs.push("OIT_ENABLED".into());
            // TODO it should be possible to use this to combine MSAA and OIT
            // alpha_to_coverage_enabled = true;
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_ALPHA {
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

        #[cfg(feature = "experimental_pbr_pcss")]
        shader_defs.push("PCSS_SAMPLERS_AVAILABLE".into());

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                TONEMAPPING_LUT_TEXTURE_BINDING_INDEX,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                TONEMAPPING_LUT_SAMPLER_BINDING_INDEX,
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
        if key.contains(MeshPipelineKey::LIGHTMAP_BICUBIC_SAMPLING) {
            shader_defs.push("LIGHTMAP_BICUBIC_SAMPLING".into());
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

        if key.contains(MeshPipelineKey::DISTANCE_FOG) {
            shader_defs.push("DISTANCE_FOG".into());
        }

        if self.binding_arrays_are_usable {
            shader_defs.push("MULTIPLE_LIGHT_PROBES_IN_ARRAY".into());
            shader_defs.push("MULTIPLE_LIGHTMAPS_IN_ARRAY".into());
        }

        if IRRADIANCE_VOLUMES_ARE_USABLE {
            shader_defs.push("IRRADIANCE_VOLUMES_ARE_USABLE".into());
        }

        if self.clustered_decals_are_usable {
            shader_defs.push("CLUSTERED_DECALS_ARE_USABLE".into());
            if cfg!(feature = "pbr_light_textures") {
                shader_defs.push("LIGHT_TEXTURES".into());
            }
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
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            layout: bind_group_layout,
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                topology: key.primitive_topology(),
                ..default()
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
            ..default()
        })
    }
}

/// The bind groups for meshes currently loaded.
///
/// If GPU mesh preprocessing isn't in use, these are global to the scene. If
/// GPU mesh preprocessing is in use, these are specific to a single phase.
#[derive(Default)]
pub struct MeshPhaseBindGroups {
    model_only: Option<BindGroup>,
    skinned: Option<MeshBindGroupPair>,
    morph_targets: HashMap<AssetId<Mesh>, MeshBindGroupPair>,
    lightmaps: HashMap<LightmapSlabIndex, BindGroup>,
}

pub struct MeshBindGroupPair {
    motion_vectors: BindGroup,
    no_motion_vectors: BindGroup,
}

/// All bind groups for meshes currently loaded.
#[derive(Resource)]
pub enum MeshBindGroups {
    /// The bind groups for the meshes for the entire scene, if GPU mesh
    /// preprocessing isn't in use.
    CpuPreprocessing(MeshPhaseBindGroups),
    /// A mapping from the type ID of a phase (e.g. [`Opaque3d`]) to the mesh
    /// bind groups for that phase.
    GpuPreprocessing(TypeIdMap<MeshPhaseBindGroups>),
}

impl MeshPhaseBindGroups {
    pub fn reset(&mut self) {
        self.model_only = None;
        self.skinned = None;
        self.morph_targets.clear();
        self.lightmaps.clear();
    }
    /// Get the `BindGroup` for `RenderMesh` with given `handle_id` and lightmap
    /// key `lightmap`.
    pub fn get(
        &self,
        asset_id: AssetId<Mesh>,
        lightmap: Option<LightmapSlabIndex>,
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
            (false, false, Some(lightmap_slab)) => self.lightmaps.get(&lightmap_slab),
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

/// Creates the per-mesh bind groups for each type of mesh and each phase.
pub fn prepare_mesh_bind_groups(
    mut commands: Commands,
    meshes: Res<RenderAssets<RenderMesh>>,
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
    mut render_lightmaps: ResMut<RenderLightmaps>,
) {
    // CPU mesh preprocessing path.
    if let Some(cpu_batched_instance_buffer) = cpu_batched_instance_buffer
        && let Some(instance_data_binding) = cpu_batched_instance_buffer
            .into_inner()
            .instance_data_binding()
    {
        // In this path, we only have a single set of bind groups for all phases.
        let cpu_preprocessing_mesh_bind_groups = prepare_mesh_bind_groups_for_phase(
            instance_data_binding,
            &meshes,
            &mesh_pipeline,
            &render_device,
            &skins_uniform,
            &weights_uniform,
            &mut render_lightmaps,
        );

        commands.insert_resource(MeshBindGroups::CpuPreprocessing(
            cpu_preprocessing_mesh_bind_groups,
        ));
        return;
    }

    // GPU mesh preprocessing path.
    if let Some(gpu_batched_instance_buffers) = gpu_batched_instance_buffers {
        let mut gpu_preprocessing_mesh_bind_groups = TypeIdMap::default();

        // Loop over each phase.
        for (phase_type_id, batched_phase_instance_buffers) in
            &gpu_batched_instance_buffers.phase_instance_buffers
        {
            let Some(instance_data_binding) =
                batched_phase_instance_buffers.instance_data_binding()
            else {
                continue;
            };

            let mesh_phase_bind_groups = prepare_mesh_bind_groups_for_phase(
                instance_data_binding,
                &meshes,
                &mesh_pipeline,
                &render_device,
                &skins_uniform,
                &weights_uniform,
                &mut render_lightmaps,
            );

            gpu_preprocessing_mesh_bind_groups.insert(*phase_type_id, mesh_phase_bind_groups);
        }

        commands.insert_resource(MeshBindGroups::GpuPreprocessing(
            gpu_preprocessing_mesh_bind_groups,
        ));
    }
}

/// Creates the per-mesh bind groups for each type of mesh, for a single phase.
fn prepare_mesh_bind_groups_for_phase(
    model: BindingResource,
    meshes: &RenderAssets<RenderMesh>,
    mesh_pipeline: &MeshPipeline,
    render_device: &RenderDevice,
    skins_uniform: &SkinUniforms,
    weights_uniform: &MorphUniforms,
    render_lightmaps: &mut RenderLightmaps,
) -> MeshPhaseBindGroups {
    let layouts = &mesh_pipeline.mesh_layouts;

    // TODO: Reuse allocations.
    let mut groups = MeshPhaseBindGroups {
        model_only: Some(layouts.model_only(render_device, &model)),
        ..default()
    };

    // Create the skinned mesh bind group with the current and previous buffers
    // (the latter being for motion vector computation).
    let (skin, prev_skin) = (&skins_uniform.current_buffer, &skins_uniform.prev_buffer);
    groups.skinned = Some(MeshBindGroupPair {
        motion_vectors: layouts.skinned_motion(render_device, &model, skin, prev_skin),
        no_motion_vectors: layouts.skinned(render_device, &model, skin),
    });

    // Create the morphed bind groups just like we did for the skinned bind
    // group.
    if let Some(weights) = weights_uniform.current_buffer.buffer() {
        let prev_weights = weights_uniform.prev_buffer.buffer().unwrap_or(weights);
        for (id, gpu_mesh) in meshes.iter() {
            if let Some(targets) = gpu_mesh.morph_targets.as_ref() {
                let bind_group_pair = if is_skinned(&gpu_mesh.layout) {
                    let prev_skin = &skins_uniform.prev_buffer;
                    MeshBindGroupPair {
                        motion_vectors: layouts.morphed_skinned_motion(
                            render_device,
                            &model,
                            skin,
                            weights,
                            targets,
                            prev_skin,
                            prev_weights,
                        ),
                        no_motion_vectors: layouts.morphed_skinned(
                            render_device,
                            &model,
                            skin,
                            weights,
                            targets,
                        ),
                    }
                } else {
                    MeshBindGroupPair {
                        motion_vectors: layouts.morphed_motion(
                            render_device,
                            &model,
                            weights,
                            targets,
                            prev_weights,
                        ),
                        no_motion_vectors: layouts.morphed(render_device, &model, weights, targets),
                    }
                };
                groups.morph_targets.insert(id, bind_group_pair);
            }
        }
    }

    // Create lightmap bindgroups. There will be one bindgroup for each slab.
    let bindless_supported = render_lightmaps.bindless_supported;
    for (lightmap_slab_id, lightmap_slab) in render_lightmaps.slabs.iter_mut().enumerate() {
        groups.lightmaps.insert(
            LightmapSlabIndex(NonMaxU32::new(lightmap_slab_id as u32).unwrap()),
            layouts.lightmapped(render_device, &model, lightmap_slab, bindless_supported),
        );
    }

    groups
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
        Read<ViewEnvironmentMapUniformOffset>,
        Read<MeshViewBindGroup>,
        Option<Read<OrderIndependentTransparencySettingsOffset>>,
    );
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (
            view_uniform,
            view_lights,
            view_fog,
            view_light_probes,
            view_ssr,
            view_environment_map,
            mesh_view_bind_group,
            maybe_oit_layers_count_offset,
        ): ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mut offsets: SmallVec<[u32; 8]> = smallvec![
            view_uniform.offset,
            view_lights.offset,
            view_fog.offset,
            **view_light_probes,
            **view_ssr,
            **view_environment_map,
        ];
        if let Some(layers_count_offset) = maybe_oit_layers_count_offset {
            offsets.push(layers_count_offset.offset);
        }
        pass.set_bind_group(I, &mesh_view_bind_group.main, &offsets);

        RenderCommandResult::Success
    }
}

pub struct SetMeshViewBindingArrayBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMeshViewBindingArrayBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<MeshViewBindGroup>,);
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (mesh_view_bind_group,): ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &mesh_view_bind_group.binding_array, &[]);

        RenderCommandResult::Success
    }
}

pub struct SetMeshViewEmptyBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMeshViewEmptyBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<MeshViewBindGroup>,);
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (mesh_view_bind_group,): ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &mesh_view_bind_group.empty, &[]);

        RenderCommandResult::Success
    }
}

pub struct SetMeshBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMeshBindGroup<I> {
    type Param = (
        SRes<RenderDevice>,
        SRes<MeshBindGroups>,
        SRes<RenderMeshInstances>,
        SRes<SkinUniforms>,
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
        (
            render_device,
            bind_groups,
            mesh_instances,
            skin_uniforms,
            morph_indices,
            lightmaps,
        ): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = bind_groups.into_inner();
        let mesh_instances = mesh_instances.into_inner();
        let skin_uniforms = skin_uniforms.into_inner();
        let morph_indices = morph_indices.into_inner();

        let entity = &item.main_entity();

        let Some(mesh_asset_id) = mesh_instances.mesh_asset_id(*entity) else {
            return RenderCommandResult::Success;
        };

        let current_skin_byte_offset = skin_uniforms.skin_byte_offset(*entity);
        let current_morph_index = morph_indices.current.get(entity);
        let prev_morph_index = morph_indices.prev.get(entity);

        let is_skinned = current_skin_byte_offset.is_some();
        let is_morphed = current_morph_index.is_some();

        let lightmap_slab_index = lightmaps
            .render_lightmaps
            .get(entity)
            .map(|render_lightmap| render_lightmap.slab_index);

        let Some(mesh_phase_bind_groups) = (match *bind_groups {
            MeshBindGroups::CpuPreprocessing(ref mesh_phase_bind_groups) => {
                Some(mesh_phase_bind_groups)
            }
            MeshBindGroups::GpuPreprocessing(ref mesh_phase_bind_groups) => {
                mesh_phase_bind_groups.get(&TypeId::of::<P>())
            }
        }) else {
            // This is harmless if e.g. we're rendering the `Shadow` phase and
            // there weren't any shadows.
            return RenderCommandResult::Success;
        };

        let Some(bind_group) = mesh_phase_bind_groups.get(
            mesh_asset_id,
            lightmap_slab_index,
            is_skinned,
            is_morphed,
            has_motion_vector_prepass,
        ) else {
            return RenderCommandResult::Failure(
                "The MeshBindGroups resource wasn't set in the render phase. \
                It should be set by the prepare_mesh_bind_group system.\n\
                This is a bevy bug! Please open an issue.",
            );
        };

        let mut dynamic_offsets: [u32; 5] = Default::default();
        let mut offset_count = 0;
        if let PhaseItemExtraIndex::DynamicOffset(dynamic_offset) = item.extra_index() {
            dynamic_offsets[offset_count] = dynamic_offset;
            offset_count += 1;
        }
        if let Some(current_skin_index) = current_skin_byte_offset
            && skins_use_uniform_buffers(&render_device)
        {
            dynamic_offsets[offset_count] = current_skin_index.byte_offset;
            offset_count += 1;
        }
        if let Some(current_morph_index) = current_morph_index {
            dynamic_offsets[offset_count] = current_morph_index.index;
            offset_count += 1;
        }

        // Attach motion vectors if needed.
        if has_motion_vector_prepass {
            // Attach the previous skin index for motion vector computation.
            if skins_use_uniform_buffers(&render_device)
                && let Some(current_skin_byte_offset) = current_skin_byte_offset
            {
                dynamic_offsets[offset_count] = current_skin_byte_offset.byte_offset;
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
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<IndirectParametersBuffers>,
        SRes<PipelineCache>,
        SRes<MeshAllocator>,
        Option<SRes<PreprocessPipelines>>,
        SRes<GpuPreprocessingSupport>,
    );
    type ViewQuery = Has<PreprocessBindGroups>;
    type ItemQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        has_preprocess_bind_group: ROQueryItem<Self::ViewQuery>,
        _item_query: Option<()>,
        (
            meshes,
            mesh_instances,
            indirect_parameters_buffer,
            pipeline_cache,
            mesh_allocator,
            preprocess_pipelines,
            preprocessing_support,
        ): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // If we're using GPU preprocessing, then we're dependent on that
        // compute shader having been run, which of course can only happen if
        // it's compiled. Otherwise, our mesh instance data won't be present.
        if let Some(preprocess_pipelines) = preprocess_pipelines
            && (!has_preprocess_bind_group
                || !preprocess_pipelines
                    .pipelines_are_loaded(&pipeline_cache, &preprocessing_support))
        {
            return RenderCommandResult::Skip;
        }

        let meshes = meshes.into_inner();
        let mesh_instances = mesh_instances.into_inner();
        let indirect_parameters_buffer = indirect_parameters_buffer.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(mesh_asset_id) = mesh_instances.mesh_asset_id(item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.get(mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(&mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

        let batch_range = item.batch_range();

        // Draw either directly or indirectly, as appropriate. If we're in
        // indirect mode, we can additionally multi-draw. (We can't multi-draw
        // in direct mode because `wgpu` doesn't expose that functionality.)
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(&mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);

                match item.extra_index() {
                    PhaseItemExtraIndex::None | PhaseItemExtraIndex::DynamicOffset(_) => {
                        pass.draw_indexed(
                            index_buffer_slice.range.start
                                ..(index_buffer_slice.range.start + *count),
                            vertex_buffer_slice.range.start as i32,
                            batch_range.clone(),
                        );
                    }
                    PhaseItemExtraIndex::IndirectParametersIndex {
                        range: indirect_parameters_range,
                        batch_set_index,
                    } => {
                        // Look up the indirect parameters buffer, as well as
                        // the buffer we're going to use for
                        // `multi_draw_indexed_indirect_count` (if available).
                        let Some(phase_indirect_parameters_buffers) =
                            indirect_parameters_buffer.get(&TypeId::of::<P>())
                        else {
                            warn!(
                                "Not rendering mesh because indexed indirect parameters buffer \
                                 wasn't present for this phase",
                            );
                            return RenderCommandResult::Skip;
                        };
                        let (Some(indirect_parameters_buffer), Some(batch_sets_buffer)) = (
                            phase_indirect_parameters_buffers.indexed.data_buffer(),
                            phase_indirect_parameters_buffers
                                .indexed
                                .batch_sets_buffer(),
                        ) else {
                            warn!(
                                "Not rendering mesh because indexed indirect parameters buffer \
                                 wasn't present",
                            );
                            return RenderCommandResult::Skip;
                        };

                        // Calculate the location of the indirect parameters
                        // within the buffer.
                        let indirect_parameters_offset = indirect_parameters_range.start as u64
                            * size_of::<IndirectParametersIndexed>() as u64;
                        let indirect_parameters_count =
                            indirect_parameters_range.end - indirect_parameters_range.start;

                        // If we're using `multi_draw_indirect_count`, take the
                        // number of batches from the appropriate position in
                        // the batch sets buffer. Otherwise, supply the size of
                        // the batch set.
                        match batch_set_index {
                            Some(batch_set_index) => {
                                let count_offset = u32::from(batch_set_index)
                                    * (size_of::<IndirectBatchSet>() as u32);
                                pass.multi_draw_indexed_indirect_count(
                                    indirect_parameters_buffer,
                                    indirect_parameters_offset,
                                    batch_sets_buffer,
                                    count_offset as u64,
                                    indirect_parameters_count,
                                );
                            }
                            None => {
                                pass.multi_draw_indexed_indirect(
                                    indirect_parameters_buffer,
                                    indirect_parameters_offset,
                                    indirect_parameters_count,
                                );
                            }
                        }
                    }
                }
            }

            RenderMeshBufferInfo::NonIndexed => match item.extra_index() {
                PhaseItemExtraIndex::None | PhaseItemExtraIndex::DynamicOffset(_) => {
                    pass.draw(vertex_buffer_slice.range, batch_range.clone());
                }
                PhaseItemExtraIndex::IndirectParametersIndex {
                    range: indirect_parameters_range,
                    batch_set_index,
                } => {
                    // Look up the indirect parameters buffer, as well as the
                    // buffer we're going to use for
                    // `multi_draw_indirect_count` (if available).
                    let Some(phase_indirect_parameters_buffers) =
                        indirect_parameters_buffer.get(&TypeId::of::<P>())
                    else {
                        warn!(
                            "Not rendering mesh because non-indexed indirect parameters buffer \
                                 wasn't present for this phase",
                        );
                        return RenderCommandResult::Skip;
                    };
                    let (Some(indirect_parameters_buffer), Some(batch_sets_buffer)) = (
                        phase_indirect_parameters_buffers.non_indexed.data_buffer(),
                        phase_indirect_parameters_buffers
                            .non_indexed
                            .batch_sets_buffer(),
                    ) else {
                        warn!(
                            "Not rendering mesh because non-indexed indirect parameters buffer \
                             wasn't present"
                        );
                        return RenderCommandResult::Skip;
                    };

                    // Calculate the location of the indirect parameters within
                    // the buffer.
                    let indirect_parameters_offset = indirect_parameters_range.start as u64
                        * size_of::<IndirectParametersNonIndexed>() as u64;
                    let indirect_parameters_count =
                        indirect_parameters_range.end - indirect_parameters_range.start;

                    // If we're using `multi_draw_indirect_count`, take the
                    // number of batches from the appropriate position in the
                    // batch sets buffer. Otherwise, supply the size of the
                    // batch set.
                    match batch_set_index {
                        Some(batch_set_index) => {
                            let count_offset =
                                u32::from(batch_set_index) * (size_of::<IndirectBatchSet>() as u32);
                            pass.multi_draw_indirect_count(
                                indirect_parameters_buffer,
                                indirect_parameters_offset,
                                batch_sets_buffer,
                                count_offset as u64,
                                indirect_parameters_count,
                            );
                        }
                        None => {
                            pass.multi_draw_indirect(
                                indirect_parameters_buffer,
                                indirect_parameters_offset,
                                indirect_parameters_count,
                            );
                        }
                    }
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
