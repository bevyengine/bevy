mod prepass_bindings;

use crate::{
    alpha_mode_pipeline_key, binding_arrays_are_usable, buffer_layout,
    collect_meshes_for_gpu_building, init_material_pipeline, set_mesh_motion_vector_flags,
    setup_morph_and_skinning_defs, skin, DeferredDrawFunction, DeferredFragmentShader,
    DeferredVertexShader, DrawMesh, EntitySpecializationTicks, ErasedMaterialPipelineKey, Material,
    MaterialPipeline, MaterialProperties, MeshLayouts, MeshPipeline, MeshPipelineKey,
    OpaqueRendererMethod, PreparedMaterial, PrepassDrawFunction, PrepassFragmentShader,
    PrepassVertexShader, RenderLightmaps, RenderMaterialInstances, RenderMeshInstanceFlags,
    RenderMeshInstances, RenderPhaseType, SetMaterialBindGroup, SetMeshBindGroup, ShadowView,
};
use bevy_app::{App, Plugin, PreUpdate};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::{Camera, Camera3d};
use bevy_core_pipeline::{core_3d::CORE_3D_DEPTH_FORMAT, deferred::*, prepass::*};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_math::{Affine3A, Mat4, Vec4};
use bevy_mesh::{Mesh, Mesh3d, MeshVertexBufferLayoutRef};
use bevy_render::{
    alpha::AlphaMode,
    batching::gpu_preprocessing::GpuPreprocessingSupport,
    globals::{GlobalsBuffer, GlobalsUniform},
    mesh::{allocator::MeshAllocator, RenderMesh},
    render_asset::{prepare_assets, RenderAssets},
    render_phase::*,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    view::{
        ExtractedView, Msaa, RenderVisibilityRanges, RetainedViewEntity, ViewUniform,
        ViewUniformOffset, ViewUniforms, VISIBILITY_RANGES_STORAGE_BUFFER_COUNT,
    },
    Extract, ExtractSchedule, Render, RenderApp, RenderDebugFlags, RenderStartup, RenderSystems,
};
use bevy_shader::{load_shader_library, Shader, ShaderDefVal};
use bevy_transform::prelude::GlobalTransform;
pub use prepass_bindings::*;
use tracing::{error, warn};

#[cfg(feature = "meshlet")]
use crate::meshlet::{
    prepare_material_meshlet_meshes_prepass, queue_material_meshlet_meshes, InstanceManager,
    MeshletMesh3d,
};

use alloc::sync::Arc;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Tick, system::SystemChangeTick};
use bevy_platform::collections::HashMap;
use bevy_render::{
    erased_render_asset::ErasedRenderAssets,
    sync_world::MainEntityHashMap,
    view::RenderVisibleEntities,
    RenderSystems::{PrepareAssets, PrepareResources},
};
use bevy_utils::default;
use core::marker::PhantomData;

/// Sets up everything required to use the prepass pipeline.
///
/// This does not add the actual prepasses, see [`PrepassPlugin`] for that.
pub struct PrepassPipelinePlugin;

impl Plugin for PrepassPipelinePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "prepass.wgsl");

        load_shader_library!(app, "prepass_bindings.wgsl");
        load_shader_library!(app, "prepass_utils.wgsl");
        load_shader_library!(app, "prepass_io.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                RenderStartup,
                (
                    init_prepass_pipeline.after(init_material_pipeline),
                    init_prepass_view_bind_group,
                )
                    .chain(),
            )
            .add_systems(
                Render,
                prepare_prepass_view_bind_group.in_set(RenderSystems::PrepareBindGroups),
            )
            .init_resource::<SpecializedMeshPipelines<PrepassPipelineSpecializer>>();
    }
}

/// Sets up the prepasses for a material.
///
/// This depends on the [`PrepassPipelinePlugin`].
pub struct PrepassPlugin {
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl PrepassPlugin {
    /// Creates a new [`PrepassPlugin`] with the given debug flags.
    pub fn new(debug_flags: RenderDebugFlags) -> Self {
        PrepassPlugin { debug_flags }
    }
}

impl Plugin for PrepassPlugin {
    fn build(&self, app: &mut App) {
        let no_prepass_plugin_loaded = app
            .world()
            .get_resource::<AnyPrepassPluginLoaded>()
            .is_none();

        if no_prepass_plugin_loaded {
            app.insert_resource(AnyPrepassPluginLoaded)
                // At the start of each frame, last frame's GlobalTransforms become this frame's PreviousGlobalTransforms
                // and last frame's view projection matrices become this frame's PreviousViewProjections
                .add_systems(
                    PreUpdate,
                    (
                        update_mesh_previous_global_transforms,
                        update_previous_view_data,
                    ),
                )
                .add_plugins((
                    BinnedRenderPhasePlugin::<Opaque3dPrepass, MeshPipeline>::new(self.debug_flags),
                    BinnedRenderPhasePlugin::<AlphaMask3dPrepass, MeshPipeline>::new(
                        self.debug_flags,
                    ),
                ));
        }

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if no_prepass_plugin_loaded {
            render_app
                .add_systems(ExtractSchedule, extract_camera_previous_view_data)
                .add_systems(
                    Render,
                    prepare_previous_view_uniforms.in_set(PrepareResources),
                );
        }

        render_app
            .init_resource::<ViewPrepassSpecializationTicks>()
            .init_resource::<ViewKeyPrepassCache>()
            .init_resource::<SpecializedPrepassMaterialPipelineCache>()
            .add_render_command::<Opaque3dPrepass, DrawPrepass>()
            .add_render_command::<AlphaMask3dPrepass, DrawPrepass>()
            .add_render_command::<Opaque3dDeferred, DrawPrepass>()
            .add_render_command::<AlphaMask3dDeferred, DrawPrepass>()
            .add_systems(
                Render,
                (
                    check_prepass_views_need_specialization.in_set(PrepareAssets),
                    specialize_prepass_material_meshes
                        .in_set(RenderSystems::PrepareMeshes)
                        .after(prepare_assets::<RenderMesh>)
                        .after(collect_meshes_for_gpu_building)
                        .after(set_mesh_motion_vector_flags),
                    queue_prepass_material_meshes.in_set(RenderSystems::QueueMeshes),
                ),
            );

        #[cfg(feature = "meshlet")]
        render_app.add_systems(
            Render,
            prepare_material_meshlet_meshes_prepass
                .in_set(RenderSystems::QueueMeshes)
                .before(queue_material_meshlet_meshes)
                .run_if(resource_exists::<InstanceManager>),
        );
    }
}

/// Marker resource for whether prepass is enabled globally for this material type
#[derive(Resource, Debug)]
pub struct PrepassEnabled<M: Material>(PhantomData<M>);

impl<M: Material> Default for PrepassEnabled<M> {
    fn default() -> Self {
        PrepassEnabled(PhantomData)
    }
}

#[derive(Resource)]
struct AnyPrepassPluginLoaded;

pub fn update_previous_view_data(
    mut commands: Commands,
    query: Query<(Entity, &Camera, &GlobalTransform), Or<(With<Camera3d>, With<ShadowView>)>>,
) {
    for (entity, camera, camera_transform) in &query {
        let world_from_view = camera_transform.affine();
        let view_from_world = Mat4::from(world_from_view.inverse());
        let view_from_clip = camera.clip_from_view().inverse();

        commands.entity(entity).try_insert(PreviousViewData {
            view_from_world,
            clip_from_world: camera.clip_from_view() * view_from_world,
            clip_from_view: camera.clip_from_view(),
            world_from_clip: Mat4::from(world_from_view) * view_from_clip,
            view_from_clip,
        });
    }
}

#[derive(Component, PartialEq, Default)]
pub struct PreviousGlobalTransform(pub Affine3A);

#[cfg(not(feature = "meshlet"))]
type PreviousMeshFilter = With<Mesh3d>;
#[cfg(feature = "meshlet")]
type PreviousMeshFilter = Or<(With<Mesh3d>, With<MeshletMesh3d>)>;

pub fn update_mesh_previous_global_transforms(
    mut commands: Commands,
    views: Query<&Camera, Or<(With<Camera3d>, With<ShadowView>)>>,
    new_meshes: Query<
        (Entity, &GlobalTransform),
        (PreviousMeshFilter, Without<PreviousGlobalTransform>),
    >,
    mut meshes: Query<(&GlobalTransform, &mut PreviousGlobalTransform), PreviousMeshFilter>,
) {
    let should_run = views.iter().any(|camera| camera.is_active);

    if should_run {
        for (entity, transform) in &new_meshes {
            let new_previous_transform = PreviousGlobalTransform(transform.affine());
            commands.entity(entity).try_insert(new_previous_transform);
        }
        meshes.par_iter_mut().for_each(|(transform, mut previous)| {
            previous.set_if_neq(PreviousGlobalTransform(transform.affine()));
        });
    }
}

#[derive(Resource, Clone)]
pub struct PrepassPipeline {
    pub view_layout_motion_vectors: BindGroupLayout,
    pub view_layout_no_motion_vectors: BindGroupLayout,
    pub mesh_layouts: MeshLayouts,
    pub empty_layout: BindGroupLayout,
    pub default_prepass_shader: Handle<Shader>,

    /// Whether skins will use uniform buffers on account of storage buffers
    /// being unavailable on this platform.
    pub skins_use_uniform_buffers: bool,

    pub depth_clip_control_supported: bool,

    /// Whether binding arrays (a.k.a. bindless textures) are usable on the
    /// current render device.
    pub binding_arrays_are_usable: bool,
    pub material_pipeline: MaterialPipeline,
}

pub fn init_prepass_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    mesh_pipeline: Res<MeshPipeline>,
    material_pipeline: Res<MaterialPipeline>,
    asset_server: Res<AssetServer>,
) {
    let visibility_ranges_buffer_binding_type =
        render_device.get_supported_read_only_binding_type(VISIBILITY_RANGES_STORAGE_BUFFER_COUNT);

    let view_layout_motion_vectors = render_device.create_bind_group_layout(
        "prepass_view_layout_motion_vectors",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::VERTEX_FRAGMENT,
            (
                // View
                (0, uniform_buffer::<ViewUniform>(true)),
                // Globals
                (1, uniform_buffer::<GlobalsUniform>(false)),
                // PreviousViewUniforms
                (2, uniform_buffer::<PreviousViewData>(true)),
                // VisibilityRanges
                (
                    14,
                    buffer_layout(
                        visibility_ranges_buffer_binding_type,
                        false,
                        Some(Vec4::min_size()),
                    )
                    .visibility(ShaderStages::VERTEX),
                ),
            ),
        ),
    );

    let view_layout_no_motion_vectors = render_device.create_bind_group_layout(
        "prepass_view_layout_no_motion_vectors",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::VERTEX_FRAGMENT,
            (
                // View
                (0, uniform_buffer::<ViewUniform>(true)),
                // Globals
                (1, uniform_buffer::<GlobalsUniform>(false)),
                // VisibilityRanges
                (
                    14,
                    buffer_layout(
                        visibility_ranges_buffer_binding_type,
                        false,
                        Some(Vec4::min_size()),
                    )
                    .visibility(ShaderStages::VERTEX),
                ),
            ),
        ),
    );

    let depth_clip_control_supported = render_device
        .features()
        .contains(WgpuFeatures::DEPTH_CLIP_CONTROL);
    commands.insert_resource(PrepassPipeline {
        view_layout_motion_vectors,
        view_layout_no_motion_vectors,
        mesh_layouts: mesh_pipeline.mesh_layouts.clone(),
        default_prepass_shader: load_embedded_asset!(asset_server.as_ref(), "prepass.wgsl"),
        skins_use_uniform_buffers: skin::skins_use_uniform_buffers(&render_device),
        depth_clip_control_supported,
        binding_arrays_are_usable: binding_arrays_are_usable(&render_device, &render_adapter),
        empty_layout: render_device.create_bind_group_layout("prepass_empty_layout", &[]),
        material_pipeline: material_pipeline.clone(),
    });
}

pub struct PrepassPipelineSpecializer {
    pub(crate) pipeline: PrepassPipeline,
    pub(crate) properties: Arc<MaterialProperties>,
}

impl SpecializedMeshPipeline for PrepassPipelineSpecializer {
    type Key = ErasedMaterialPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        if self.properties.bindless {
            shader_defs.push("BINDLESS".into());
        }
        let mut descriptor =
            self.pipeline
                .specialize(key.mesh_key, shader_defs, layout, &self.properties)?;

        // This is a bit risky because it's possible to change something that would
        // break the prepass but be fine in the main pass.
        // Since this api is pretty low-level it doesn't matter that much, but it is a potential issue.
        if let Some(specialize) = self.properties.specialize {
            specialize(
                &self.pipeline.material_pipeline,
                &mut descriptor,
                layout,
                key,
            )?;
        }

        Ok(descriptor)
    }
}

impl PrepassPipeline {
    fn specialize(
        &self,
        mesh_key: MeshPipelineKey,
        shader_defs: Vec<ShaderDefVal>,
        layout: &MeshVertexBufferLayoutRef,
        material_properties: &MaterialProperties,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = shader_defs;
        let mut bind_group_layouts = vec![
            if mesh_key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
                self.view_layout_motion_vectors.clone()
            } else {
                self.view_layout_no_motion_vectors.clone()
            },
            self.empty_layout.clone(),
        ];
        let mut vertex_attributes = Vec::new();

        // Let the shader code know that it's running in a prepass pipeline.
        // (PBR code will use this to detect that it's running in deferred mode,
        // since that's the only time it gets called from a prepass pipeline.)
        shader_defs.push("PREPASS_PIPELINE".into());

        shader_defs.push(ShaderDefVal::UInt(
            "MATERIAL_BIND_GROUP".into(),
            crate::MATERIAL_BIND_GROUP_INDEX as u32,
        ));
        // NOTE: Eventually, it would be nice to only add this when the shaders are overloaded by the Material.
        // The main limitation right now is that bind group order is hardcoded in shaders.
        bind_group_layouts.push(
            material_properties
                .material_layout
                .as_ref()
                .unwrap()
                .clone(),
        );
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("WEBGL2".into());
        shader_defs.push("VERTEX_OUTPUT_INSTANCE_INDEX".into());
        if mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }
        if mesh_key.contains(MeshPipelineKey::MAY_DISCARD) {
            shader_defs.push("MAY_DISCARD".into());
        }
        let blend_key = mesh_key.intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
        if blend_key == MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA {
            shader_defs.push("BLEND_PREMULTIPLIED_ALPHA".into());
        }
        if blend_key == MeshPipelineKey::BLEND_ALPHA {
            shader_defs.push("BLEND_ALPHA".into());
        }
        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }
        // For directional light shadow map views, use unclipped depth via either the native GPU feature,
        // or emulated by setting depth in the fragment shader for GPUs that don't support it natively.
        let emulate_unclipped_depth = mesh_key.contains(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO)
            && !self.depth_clip_control_supported;
        if emulate_unclipped_depth {
            shader_defs.push("UNCLIPPED_DEPTH_ORTHO_EMULATION".into());
            // PERF: This line forces the "prepass fragment shader" to always run in
            // common scenarios like "directional light calculation". Doing so resolves
            // a pretty nasty depth clamping bug, but it also feels a bit excessive.
            // We should try to find a way to resolve this without forcing the fragment
            // shader to run.
            // https://github.com/bevyengine/bevy/pull/8877
            shader_defs.push("PREPASS_FRAGMENT".into());
        }
        let unclipped_depth = mesh_key.contains(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO)
            && self.depth_clip_control_supported;
        if layout.0.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            shader_defs.push("VERTEX_UVS_A".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(1));
        }
        if layout.0.contains(Mesh::ATTRIBUTE_UV_1) {
            shader_defs.push("VERTEX_UVS".into());
            shader_defs.push("VERTEX_UVS_B".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_1.at_shader_location(2));
        }
        if mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }
        if mesh_key.intersects(MeshPipelineKey::NORMAL_PREPASS | MeshPipelineKey::DEFERRED_PREPASS)
        {
            shader_defs.push("NORMAL_PREPASS_OR_DEFERRED_PREPASS".into());
            if layout.0.contains(Mesh::ATTRIBUTE_NORMAL) {
                shader_defs.push("VERTEX_NORMALS".into());
                vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(3));
            } else if mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS) {
                warn!(
                    "The default normal prepass expects the mesh to have vertex normal attributes."
                );
            }
            if layout.0.contains(Mesh::ATTRIBUTE_TANGENT) {
                shader_defs.push("VERTEX_TANGENTS".into());
                vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(4));
            }
        }
        if mesh_key
            .intersects(MeshPipelineKey::MOTION_VECTOR_PREPASS | MeshPipelineKey::DEFERRED_PREPASS)
        {
            shader_defs.push("MOTION_VECTOR_PREPASS_OR_DEFERRED_PREPASS".into());
        }
        if mesh_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            shader_defs.push("DEFERRED_PREPASS".into());
        }
        if mesh_key.contains(MeshPipelineKey::LIGHTMAPPED) {
            shader_defs.push("LIGHTMAP".into());
        }
        if mesh_key.contains(MeshPipelineKey::LIGHTMAP_BICUBIC_SAMPLING) {
            shader_defs.push("LIGHTMAP_BICUBIC_SAMPLING".into());
        }
        if layout.0.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(7));
        }
        if mesh_key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }
        if mesh_key.contains(MeshPipelineKey::HAS_PREVIOUS_SKIN) {
            shader_defs.push("HAS_PREVIOUS_SKIN".into());
        }
        if mesh_key.contains(MeshPipelineKey::HAS_PREVIOUS_MORPH) {
            shader_defs.push("HAS_PREVIOUS_MORPH".into());
        }
        if self.binding_arrays_are_usable {
            shader_defs.push("MULTIPLE_LIGHTMAPS_IN_ARRAY".into());
        }
        if mesh_key.contains(MeshPipelineKey::VISIBILITY_RANGE_DITHER) {
            shader_defs.push("VISIBILITY_RANGE_DITHER".into());
        }
        if mesh_key.intersects(
            MeshPipelineKey::NORMAL_PREPASS
                | MeshPipelineKey::MOTION_VECTOR_PREPASS
                | MeshPipelineKey::DEFERRED_PREPASS,
        ) {
            shader_defs.push("PREPASS_FRAGMENT".into());
        }
        let bind_group = setup_morph_and_skinning_defs(
            &self.mesh_layouts,
            layout,
            5,
            &mesh_key,
            &mut shader_defs,
            &mut vertex_attributes,
            self.skins_use_uniform_buffers,
        );
        bind_group_layouts.insert(2, bind_group);
        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;
        // Setup prepass fragment targets - normals in slot 0 (or None if not needed), motion vectors in slot 1
        let mut targets = prepass_target_descriptors(
            mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS),
            mesh_key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS),
            mesh_key.contains(MeshPipelineKey::DEFERRED_PREPASS),
        );

        if targets.iter().all(Option::is_none) {
            // if no targets are required then clear the list, so that no fragment shader is required
            // (though one may still be used for discarding depth buffer writes)
            targets.clear();
        }

        // The fragment shader is only used when the normal prepass or motion vectors prepass
        // is enabled, the material uses alpha cutoff values and doesn't rely on the standard
        // prepass shader, or we are emulating unclipped depth in the fragment shader.
        let fragment_required = !targets.is_empty()
            || emulate_unclipped_depth
            || (mesh_key.contains(MeshPipelineKey::MAY_DISCARD)
                && material_properties
                    .get_shader(PrepassFragmentShader)
                    .is_some());

        let fragment = fragment_required.then(|| {
            // Use the fragment shader from the material
            let frag_shader_handle = if mesh_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
                match material_properties.get_shader(DeferredFragmentShader) {
                    Some(frag_shader_handle) => frag_shader_handle,
                    None => self.default_prepass_shader.clone(),
                }
            } else {
                match material_properties.get_shader(PrepassFragmentShader) {
                    Some(frag_shader_handle) => frag_shader_handle,
                    None => self.default_prepass_shader.clone(),
                }
            };

            FragmentState {
                shader: frag_shader_handle,
                shader_defs: shader_defs.clone(),
                targets,
                ..default()
            }
        });

        // Use the vertex shader from the material if present
        let vert_shader_handle = if mesh_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            if let Some(handle) = material_properties.get_shader(DeferredVertexShader) {
                handle
            } else {
                self.default_prepass_shader.clone()
            }
        } else if let Some(handle) = material_properties.get_shader(PrepassVertexShader) {
            handle
        } else {
            self.default_prepass_shader.clone()
        };
        let descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: vert_shader_handle,
                shader_defs,
                buffers: vec![vertex_buffer_layout],
                ..default()
            },
            fragment,
            layout: bind_group_layouts,
            primitive: PrimitiveState {
                topology: mesh_key.primitive_topology(),
                unclipped_depth,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
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
                count: mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("prepass_pipeline".into()),
            ..default()
        };
        Ok(descriptor)
    }
}

// Extract the render phases for the prepass
pub fn extract_camera_previous_view_data(
    mut commands: Commands,
    cameras_3d: Extract<Query<(RenderEntity, &Camera, Option<&PreviousViewData>), With<Camera3d>>>,
) {
    for (entity, camera, maybe_previous_view_data) in cameras_3d.iter() {
        let mut entity = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if camera.is_active {
            if let Some(previous_view_data) = maybe_previous_view_data {
                entity.insert(previous_view_data.clone());
            }
        } else {
            entity.remove::<PreviousViewData>();
        }
    }
}

pub fn prepare_previous_view_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut previous_view_uniforms: ResMut<PreviousViewUniforms>,
    views: Query<
        (Entity, &ExtractedView, Option<&PreviousViewData>),
        Or<(With<Camera3d>, With<ShadowView>)>,
    >,
) {
    let views_iter = views.iter();
    let view_count = views_iter.len();
    let Some(mut writer) =
        previous_view_uniforms
            .uniforms
            .get_writer(view_count, &render_device, &render_queue)
    else {
        return;
    };

    for (entity, camera, maybe_previous_view_uniforms) in views_iter {
        let prev_view_data = match maybe_previous_view_uniforms {
            Some(previous_view) => previous_view.clone(),
            None => {
                let world_from_view = camera.world_from_view.affine();
                let view_from_world = Mat4::from(world_from_view.inverse());
                let view_from_clip = camera.clip_from_view.inverse();

                PreviousViewData {
                    view_from_world,
                    clip_from_world: camera.clip_from_view * view_from_world,
                    clip_from_view: camera.clip_from_view,
                    world_from_clip: Mat4::from(world_from_view) * view_from_clip,
                    view_from_clip,
                }
            }
        };

        commands.entity(entity).insert(PreviousViewUniformOffset {
            offset: writer.write(&prev_view_data),
        });
    }
}

#[derive(Resource)]
pub struct PrepassViewBindGroup {
    pub motion_vectors: Option<BindGroup>,
    pub no_motion_vectors: Option<BindGroup>,
    pub empty_bind_group: BindGroup,
}

pub fn init_prepass_view_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<PrepassPipeline>,
) {
    let empty_bind_group = render_device.create_bind_group(
        "prepass_view_empty_bind_group",
        &pipeline.empty_layout,
        &[],
    );
    commands.insert_resource(PrepassViewBindGroup {
        motion_vectors: None,
        no_motion_vectors: None,
        empty_bind_group,
    });
}

pub fn prepare_prepass_view_bind_group(
    render_device: Res<RenderDevice>,
    prepass_pipeline: Res<PrepassPipeline>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    previous_view_uniforms: Res<PreviousViewUniforms>,
    visibility_ranges: Res<RenderVisibilityRanges>,
    mut prepass_view_bind_group: ResMut<PrepassViewBindGroup>,
) {
    if let (Some(view_binding), Some(globals_binding), Some(visibility_ranges_buffer)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
        visibility_ranges.buffer().buffer(),
    ) {
        prepass_view_bind_group.no_motion_vectors = Some(render_device.create_bind_group(
            "prepass_view_no_motion_vectors_bind_group",
            &prepass_pipeline.view_layout_no_motion_vectors,
            &BindGroupEntries::with_indices((
                (0, view_binding.clone()),
                (1, globals_binding.clone()),
                (14, visibility_ranges_buffer.as_entire_binding()),
            )),
        ));

        if let Some(previous_view_uniforms_binding) = previous_view_uniforms.uniforms.binding() {
            prepass_view_bind_group.motion_vectors = Some(render_device.create_bind_group(
                "prepass_view_motion_vectors_bind_group",
                &prepass_pipeline.view_layout_motion_vectors,
                &BindGroupEntries::with_indices((
                    (0, view_binding),
                    (1, globals_binding),
                    (2, previous_view_uniforms_binding),
                    (14, visibility_ranges_buffer.as_entire_binding()),
                )),
            ));
        }
    }
}

/// Stores the [`SpecializedPrepassMaterialViewPipelineCache`] for each view.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct SpecializedPrepassMaterialPipelineCache {
    // view_entity -> view pipeline cache
    #[deref]
    map: HashMap<RetainedViewEntity, SpecializedPrepassMaterialViewPipelineCache>,
}

/// Stores the cached render pipeline ID for each entity in a single view, as
/// well as the last time it was changed.
#[derive(Deref, DerefMut, Default)]
pub struct SpecializedPrepassMaterialViewPipelineCache {
    // material entity -> (tick, pipeline_id)
    #[deref]
    map: MainEntityHashMap<(Tick, CachedRenderPipelineId)>,
}

#[derive(Resource, Deref, DerefMut, Default, Clone)]
pub struct ViewKeyPrepassCache(HashMap<RetainedViewEntity, MeshPipelineKey>);

#[derive(Resource, Deref, DerefMut, Default, Clone)]
pub struct ViewPrepassSpecializationTicks(HashMap<RetainedViewEntity, Tick>);

pub fn check_prepass_views_need_specialization(
    mut view_key_cache: ResMut<ViewKeyPrepassCache>,
    mut view_specialization_ticks: ResMut<ViewPrepassSpecializationTicks>,
    mut views: Query<(
        &ExtractedView,
        &Msaa,
        Option<&DepthPrepass>,
        Option<&NormalPrepass>,
        Option<&MotionVectorPrepass>,
    )>,
    ticks: SystemChangeTick,
) {
    for (view, msaa, depth_prepass, normal_prepass, motion_vector_prepass) in views.iter_mut() {
        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
        if depth_prepass.is_some() {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }
        if normal_prepass.is_some() {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }
        if motion_vector_prepass.is_some() {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if let Some(current_key) = view_key_cache.get_mut(&view.retained_view_entity) {
            if *current_key != view_key {
                view_key_cache.insert(view.retained_view_entity, view_key);
                view_specialization_ticks.insert(view.retained_view_entity, ticks.this_run());
            }
        } else {
            view_key_cache.insert(view.retained_view_entity, view_key);
            view_specialization_ticks.insert(view.retained_view_entity, ticks.this_run());
        }
    }
}

pub fn specialize_prepass_material_meshes(
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_material_instances: Res<RenderMaterialInstances>,
    render_lightmaps: Res<RenderLightmaps>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    view_key_cache: Res<ViewKeyPrepassCache>,
    views: Query<(
        &ExtractedView,
        &RenderVisibleEntities,
        &Msaa,
        Option<&MotionVectorPrepass>,
        Option<&DeferredPrepass>,
    )>,
    (
        opaque_prepass_render_phases,
        alpha_mask_prepass_render_phases,
        opaque_deferred_render_phases,
        alpha_mask_deferred_render_phases,
    ): (
        Res<ViewBinnedRenderPhases<Opaque3dPrepass>>,
        Res<ViewBinnedRenderPhases<AlphaMask3dPrepass>>,
        Res<ViewBinnedRenderPhases<Opaque3dDeferred>>,
        Res<ViewBinnedRenderPhases<AlphaMask3dDeferred>>,
    ),
    (
        mut specialized_material_pipeline_cache,
        ticks,
        prepass_pipeline,
        mut pipelines,
        pipeline_cache,
        view_specialization_ticks,
        entity_specialization_ticks,
    ): (
        ResMut<SpecializedPrepassMaterialPipelineCache>,
        SystemChangeTick,
        Res<PrepassPipeline>,
        ResMut<SpecializedMeshPipelines<PrepassPipelineSpecializer>>,
        Res<PipelineCache>,
        Res<ViewPrepassSpecializationTicks>,
        Res<EntitySpecializationTicks>,
    ),
) {
    for (extracted_view, visible_entities, msaa, motion_vector_prepass, deferred_prepass) in &views
    {
        if !opaque_deferred_render_phases.contains_key(&extracted_view.retained_view_entity)
            && !alpha_mask_deferred_render_phases.contains_key(&extracted_view.retained_view_entity)
            && !opaque_prepass_render_phases.contains_key(&extracted_view.retained_view_entity)
            && !alpha_mask_prepass_render_phases.contains_key(&extracted_view.retained_view_entity)
        {
            continue;
        }

        let Some(view_key) = view_key_cache.get(&extracted_view.retained_view_entity) else {
            continue;
        };

        let view_tick = view_specialization_ticks
            .get(&extracted_view.retained_view_entity)
            .unwrap();
        let view_specialized_material_pipeline_cache = specialized_material_pipeline_cache
            .entry(extracted_view.retained_view_entity)
            .or_default();

        for (_, visible_entity) in visible_entities.iter::<Mesh3d>() {
            let Some(material_instance) = render_material_instances.instances.get(visible_entity)
            else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let entity_tick = entity_specialization_ticks.get(visible_entity).unwrap();
            let last_specialized_tick = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .map(|(tick, _)| *tick);
            let needs_specialization = last_specialized_tick.is_none_or(|tick| {
                view_tick.is_newer_than(tick, ticks.this_run())
                    || entity_tick.is_newer_than(tick, ticks.this_run())
            });
            if !needs_specialization {
                continue;
            }
            let Some(material) = render_materials.get(material_instance.asset_id) else {
                continue;
            };
            if !material.properties.prepass_enabled && !material.properties.shadows_enabled {
                // If the material was previously specialized for prepass, remove it
                view_specialized_material_pipeline_cache.remove(visible_entity);
                continue;
            }
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let mut mesh_key = *view_key | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

            let alpha_mode = material.properties.alpha_mode;
            match alpha_mode {
                AlphaMode::Opaque | AlphaMode::AlphaToCoverage | AlphaMode::Mask(_) => {
                    mesh_key |= alpha_mode_pipeline_key(alpha_mode, msaa);
                }
                AlphaMode::Blend
                | AlphaMode::Premultiplied
                | AlphaMode::Add
                | AlphaMode::Multiply => {
                    // In case this material was previously in a valid alpha_mode, remove it to
                    // stop the queue system from assuming its retained cache to be valid.
                    view_specialized_material_pipeline_cache.remove(visible_entity);
                    continue;
                }
            }

            if material.properties.reads_view_transmission_texture {
                // No-op: Materials reading from `ViewTransmissionTexture` are not rendered in the `Opaque3d`
                // phase, and are therefore also excluded from the prepass much like alpha-blended materials.
                view_specialized_material_pipeline_cache.remove(visible_entity);
                continue;
            }

            let forward = match material.properties.render_method {
                OpaqueRendererMethod::Forward => true,
                OpaqueRendererMethod::Deferred => false,
                OpaqueRendererMethod::Auto => unreachable!(),
            };

            let deferred = deferred_prepass.is_some() && !forward;

            if deferred {
                mesh_key |= MeshPipelineKey::DEFERRED_PREPASS;
            }

            if let Some(lightmap) = render_lightmaps.render_lightmaps.get(visible_entity) {
                // Even though we don't use the lightmap in the forward prepass, the
                // `SetMeshBindGroup` render command will bind the data for it. So
                // we need to include the appropriate flag in the mesh pipeline key
                // to ensure that the necessary bind group layout entries are
                // present.
                mesh_key |= MeshPipelineKey::LIGHTMAPPED;

                if lightmap.bicubic_sampling && deferred {
                    mesh_key |= MeshPipelineKey::LIGHTMAP_BICUBIC_SAMPLING;
                }
            }

            if render_visibility_ranges.entity_has_crossfading_visibility_ranges(*visible_entity) {
                mesh_key |= MeshPipelineKey::VISIBILITY_RANGE_DITHER;
            }

            // If the previous frame has skins or morph targets, note that.
            if motion_vector_prepass.is_some() {
                if mesh_instance
                    .flags
                    .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_SKIN)
                {
                    mesh_key |= MeshPipelineKey::HAS_PREVIOUS_SKIN;
                }
                if mesh_instance
                    .flags
                    .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_MORPH)
                {
                    mesh_key |= MeshPipelineKey::HAS_PREVIOUS_MORPH;
                }
            }

            let erased_key = ErasedMaterialPipelineKey {
                mesh_key,
                material_key: material.properties.material_key.clone(),
                type_id: material_instance.asset_id.type_id(),
            };
            let prepass_pipeline_specializer = PrepassPipelineSpecializer {
                pipeline: prepass_pipeline.clone(),
                properties: material.properties.clone(),
            };
            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &prepass_pipeline_specializer,
                erased_key,
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            view_specialized_material_pipeline_cache
                .insert(*visible_entity, (ticks.this_run(), pipeline_id));
        }
    }
}

pub fn queue_prepass_material_meshes(
    render_mesh_instances: Res<RenderMeshInstances>,
    render_materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    render_material_instances: Res<RenderMaterialInstances>,
    mesh_allocator: Res<MeshAllocator>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mut opaque_prepass_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3dPrepass>>,
    mut alpha_mask_prepass_render_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3dPrepass>>,
    mut opaque_deferred_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3dDeferred>>,
    mut alpha_mask_deferred_render_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3dDeferred>>,
    views: Query<(&ExtractedView, &RenderVisibleEntities)>,
    specialized_material_pipeline_cache: Res<SpecializedPrepassMaterialPipelineCache>,
) {
    for (extracted_view, visible_entities) in &views {
        let (
            mut opaque_phase,
            mut alpha_mask_phase,
            mut opaque_deferred_phase,
            mut alpha_mask_deferred_phase,
        ) = (
            opaque_prepass_render_phases.get_mut(&extracted_view.retained_view_entity),
            alpha_mask_prepass_render_phases.get_mut(&extracted_view.retained_view_entity),
            opaque_deferred_render_phases.get_mut(&extracted_view.retained_view_entity),
            alpha_mask_deferred_render_phases.get_mut(&extracted_view.retained_view_entity),
        );

        let Some(view_specialized_material_pipeline_cache) =
            specialized_material_pipeline_cache.get(&extracted_view.retained_view_entity)
        else {
            continue;
        };

        // Skip if there's no place to put the mesh.
        if opaque_phase.is_none()
            && alpha_mask_phase.is_none()
            && opaque_deferred_phase.is_none()
            && alpha_mask_deferred_phase.is_none()
        {
            continue;
        }

        for (render_entity, visible_entity) in visible_entities.iter::<Mesh3d>() {
            let Some((current_change_tick, pipeline_id)) =
                view_specialized_material_pipeline_cache.get(visible_entity)
            else {
                continue;
            };

            // Skip the entity if it's cached in a bin and up to date.
            if opaque_phase.as_mut().is_some_and(|phase| {
                phase.validate_cached_entity(*visible_entity, *current_change_tick)
            }) || alpha_mask_phase.as_mut().is_some_and(|phase| {
                phase.validate_cached_entity(*visible_entity, *current_change_tick)
            }) || opaque_deferred_phase.as_mut().is_some_and(|phase| {
                phase.validate_cached_entity(*visible_entity, *current_change_tick)
            }) || alpha_mask_deferred_phase.as_mut().is_some_and(|phase| {
                phase.validate_cached_entity(*visible_entity, *current_change_tick)
            }) {
                continue;
            }

            let Some(material_instance) = render_material_instances.instances.get(visible_entity)
            else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let Some(material) = render_materials.get(material_instance.asset_id) else {
                continue;
            };
            let (vertex_slab, index_slab) = mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);

            let deferred = match material.properties.render_method {
                OpaqueRendererMethod::Forward => false,
                OpaqueRendererMethod::Deferred => true,
                OpaqueRendererMethod::Auto => unreachable!(),
            };

            match material.properties.render_phase_type {
                RenderPhaseType::Opaque => {
                    if deferred {
                        opaque_deferred_phase.as_mut().unwrap().add(
                            OpaqueNoLightmap3dBatchSetKey {
                                draw_function: material
                                    .properties
                                    .get_draw_function(DeferredDrawFunction)
                                    .unwrap(),
                                pipeline: *pipeline_id,
                                material_bind_group_index: Some(material.binding.group.0),
                                vertex_slab: vertex_slab.unwrap_or_default(),
                                index_slab,
                            },
                            OpaqueNoLightmap3dBinKey {
                                asset_id: mesh_instance.mesh_asset_id.into(),
                            },
                            (*render_entity, *visible_entity),
                            mesh_instance.current_uniform_index,
                            BinnedRenderPhaseType::mesh(
                                mesh_instance.should_batch(),
                                &gpu_preprocessing_support,
                            ),
                            *current_change_tick,
                        );
                    } else if let Some(opaque_phase) = opaque_phase.as_mut() {
                        let (vertex_slab, index_slab) =
                            mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);
                        opaque_phase.add(
                            OpaqueNoLightmap3dBatchSetKey {
                                draw_function: material
                                    .properties
                                    .get_draw_function(PrepassDrawFunction)
                                    .unwrap(),
                                pipeline: *pipeline_id,
                                material_bind_group_index: Some(material.binding.group.0),
                                vertex_slab: vertex_slab.unwrap_or_default(),
                                index_slab,
                            },
                            OpaqueNoLightmap3dBinKey {
                                asset_id: mesh_instance.mesh_asset_id.into(),
                            },
                            (*render_entity, *visible_entity),
                            mesh_instance.current_uniform_index,
                            BinnedRenderPhaseType::mesh(
                                mesh_instance.should_batch(),
                                &gpu_preprocessing_support,
                            ),
                            *current_change_tick,
                        );
                    }
                }
                RenderPhaseType::AlphaMask => {
                    if deferred {
                        let (vertex_slab, index_slab) =
                            mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);
                        let batch_set_key = OpaqueNoLightmap3dBatchSetKey {
                            draw_function: material
                                .properties
                                .get_draw_function(DeferredDrawFunction)
                                .unwrap(),
                            pipeline: *pipeline_id,
                            material_bind_group_index: Some(material.binding.group.0),
                            vertex_slab: vertex_slab.unwrap_or_default(),
                            index_slab,
                        };
                        let bin_key = OpaqueNoLightmap3dBinKey {
                            asset_id: mesh_instance.mesh_asset_id.into(),
                        };
                        alpha_mask_deferred_phase.as_mut().unwrap().add(
                            batch_set_key,
                            bin_key,
                            (*render_entity, *visible_entity),
                            mesh_instance.current_uniform_index,
                            BinnedRenderPhaseType::mesh(
                                mesh_instance.should_batch(),
                                &gpu_preprocessing_support,
                            ),
                            *current_change_tick,
                        );
                    } else if let Some(alpha_mask_phase) = alpha_mask_phase.as_mut() {
                        let (vertex_slab, index_slab) =
                            mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);
                        let batch_set_key = OpaqueNoLightmap3dBatchSetKey {
                            draw_function: material
                                .properties
                                .get_draw_function(PrepassDrawFunction)
                                .unwrap(),
                            pipeline: *pipeline_id,
                            material_bind_group_index: Some(material.binding.group.0),
                            vertex_slab: vertex_slab.unwrap_or_default(),
                            index_slab,
                        };
                        let bin_key = OpaqueNoLightmap3dBinKey {
                            asset_id: mesh_instance.mesh_asset_id.into(),
                        };
                        alpha_mask_phase.add(
                            batch_set_key,
                            bin_key,
                            (*render_entity, *visible_entity),
                            mesh_instance.current_uniform_index,
                            BinnedRenderPhaseType::mesh(
                                mesh_instance.should_batch(),
                                &gpu_preprocessing_support,
                            ),
                            *current_change_tick,
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct SetPrepassViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetPrepassViewBindGroup<I> {
    type Param = SRes<PrepassViewBindGroup>;
    type ViewQuery = (
        Read<ViewUniformOffset>,
        Has<MotionVectorPrepass>,
        Option<Read<PreviousViewUniformOffset>>,
    );
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform_offset, has_motion_vector_prepass, previous_view_uniform_offset): (
            &'_ ViewUniformOffset,
            bool,
            Option<&'_ PreviousViewUniformOffset>,
        ),
        _entity: Option<()>,
        prepass_view_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let prepass_view_bind_group = prepass_view_bind_group.into_inner();

        match previous_view_uniform_offset {
            Some(previous_view_uniform_offset) if has_motion_vector_prepass => {
                pass.set_bind_group(
                    I,
                    prepass_view_bind_group.motion_vectors.as_ref().unwrap(),
                    &[
                        view_uniform_offset.offset,
                        previous_view_uniform_offset.offset,
                    ],
                );
            }
            _ => {
                pass.set_bind_group(
                    I,
                    prepass_view_bind_group.no_motion_vectors.as_ref().unwrap(),
                    &[view_uniform_offset.offset],
                );
            }
        }
        RenderCommandResult::Success
    }
}

pub struct SetPrepassViewEmptyBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetPrepassViewEmptyBindGroup<I> {
    type Param = SRes<PrepassViewBindGroup>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        _entity: Option<()>,
        prepass_view_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let prepass_view_bind_group = prepass_view_bind_group.into_inner();
        pass.set_bind_group(I, &prepass_view_bind_group.empty_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub type DrawPrepass = (
    SetItemPipeline,
    SetPrepassViewBindGroup<0>,
    SetPrepassViewEmptyBindGroup<1>,
    SetMeshBindGroup<2>,
    SetMaterialBindGroup<3>,
    DrawMesh,
);
