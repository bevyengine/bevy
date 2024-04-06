mod prepass_bindings;

use bevy_render::batching::{batch_and_prepare_binned_render_phase, sort_binned_render_phase};
use bevy_render::mesh::MeshVertexBufferLayoutRef;
use bevy_render::render_resource::binding_types::uniform_buffer;
pub use prepass_bindings::*;

use bevy_asset::{load_internal_asset, AssetServer};
use bevy_core_pipeline::{core_3d::CORE_3D_DEPTH_FORMAT, prelude::Camera3d};
use bevy_core_pipeline::{deferred::*, prepass::*};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_math::{Affine3A, Mat4};
use bevy_render::{
    globals::{GlobalsBuffer, GlobalsUniform},
    prelude::{Camera, Mesh},
    render_asset::RenderAssets,
    render_phase::*,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    view::{ExtractedView, Msaa, ViewUniform, ViewUniformOffset, ViewUniforms, VisibleEntities},
    Extract,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::tracing::error;

#[cfg(feature = "meshlet")]
use crate::meshlet::{
    prepare_material_meshlet_meshes_prepass, queue_material_meshlet_meshes, MeshletGpuScene,
    MeshletMesh,
};
use crate::*;

use std::{hash::Hash, marker::PhantomData};

pub const PREPASS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(921124473254008983);

pub const PREPASS_BINDINGS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(5533152893177403494);

pub const PREPASS_UTILS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4603948296044544);

pub const PREPASS_IO_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(81212356509530944);

/// Sets up everything required to use the prepass pipeline.
///
/// This does not add the actual prepasses, see [`PrepassPlugin`] for that.
pub struct PrepassPipelinePlugin<M: Material>(PhantomData<M>);

impl<M: Material> Default for PrepassPipelinePlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: Material> Plugin for PrepassPipelinePlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PREPASS_SHADER_HANDLE,
            "prepass.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            PREPASS_BINDINGS_SHADER_HANDLE,
            "prepass_bindings.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            PREPASS_UTILS_SHADER_HANDLE,
            "prepass_utils.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            PREPASS_IO_SHADER_HANDLE,
            "prepass_io.wgsl",
            Shader::from_wgsl
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                Render,
                prepare_prepass_view_bind_group::<M>.in_set(RenderSet::PrepareBindGroups),
            )
            .init_resource::<PrepassViewBindGroup>()
            .init_resource::<SpecializedMeshPipelines<PrepassPipeline<M>>>()
            .allow_ambiguous_resource::<SpecializedMeshPipelines<PrepassPipeline<M>>>()
            .init_resource::<PreviousViewProjectionUniforms>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<PrepassPipeline<M>>();
    }
}

/// Sets up the prepasses for a [`Material`].
///
/// This depends on the [`PrepassPipelinePlugin`].
pub struct PrepassPlugin<M: Material>(PhantomData<M>);

impl<M: Material> Default for PrepassPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: Material> Plugin for PrepassPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
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
                        update_previous_view_projections,
                    ),
                );
        }

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if no_prepass_plugin_loaded {
            render_app
                .add_systems(ExtractSchedule, extract_camera_previous_view_projection)
                .add_systems(
                    Render,
                    (
                        (
                            sort_binned_render_phase::<Opaque3dPrepass>,
                            sort_binned_render_phase::<AlphaMask3dPrepass>
                        ).in_set(RenderSet::PhaseSort),
                        (
                            prepare_previous_view_projection_uniforms,
                            batch_and_prepare_binned_render_phase::<Opaque3dPrepass, MeshPipeline>,
                            batch_and_prepare_binned_render_phase::<AlphaMask3dPrepass,
                                MeshPipeline>,
                        ).in_set(RenderSet::PrepareResources),
                    )
                );
        }

        render_app
            .add_render_command::<Opaque3dPrepass, DrawPrepass<M>>()
            .add_render_command::<AlphaMask3dPrepass, DrawPrepass<M>>()
            .add_render_command::<Opaque3dDeferred, DrawPrepass<M>>()
            .add_render_command::<AlphaMask3dDeferred, DrawPrepass<M>>()
            .add_systems(
                Render,
                queue_prepass_material_meshes::<M>
                    .in_set(RenderSet::QueueMeshes)
                    .after(prepare_materials::<M>)
                    // queue_material_meshes only writes to `material_bind_group_id`, which `queue_prepass_material_meshes` doesn't read
                    .ambiguous_with(queue_material_meshes::<StandardMaterial>),
            );

        #[cfg(feature = "meshlet")]
        render_app.add_systems(
            Render,
            prepare_material_meshlet_meshes_prepass::<M>
                .in_set(RenderSet::Queue)
                .before(queue_material_meshlet_meshes::<M>)
                .run_if(resource_exists::<MeshletGpuScene>),
        );
    }
}

#[derive(Resource)]
struct AnyPrepassPluginLoaded;

#[derive(Component, ShaderType, Clone)]
pub struct PreviousViewProjection {
    pub view_proj: Mat4,
}

#[cfg(not(feature = "meshlet"))]
type PreviousViewFilter = (With<Camera3d>, With<MotionVectorPrepass>);
#[cfg(feature = "meshlet")]
type PreviousViewFilter = With<Camera3d>;

pub fn update_previous_view_projections(
    mut commands: Commands,
    query: Query<(Entity, &Camera, &GlobalTransform), PreviousViewFilter>,
) {
    for (entity, camera, camera_transform) in &query {
        commands.entity(entity).try_insert(PreviousViewProjection {
            view_proj: camera.projection_matrix() * camera_transform.compute_matrix().inverse(),
        });
    }
}

#[derive(Component)]
pub struct PreviousGlobalTransform(pub Affine3A);

#[cfg(not(feature = "meshlet"))]
type PreviousMeshFilter = With<Handle<Mesh>>;
#[cfg(feature = "meshlet")]
type PreviousMeshFilter = Or<(With<Handle<Mesh>>, With<Handle<MeshletMesh>>)>;

pub fn update_mesh_previous_global_transforms(
    mut commands: Commands,
    views: Query<&Camera, PreviousViewFilter>,
    meshes: Query<(Entity, &GlobalTransform), PreviousMeshFilter>,
) {
    let should_run = views.iter().any(|camera| camera.is_active);

    if should_run {
        for (entity, transform) in &meshes {
            commands
                .entity(entity)
                .try_insert(PreviousGlobalTransform(transform.affine()));
        }
    }
}

#[derive(Resource)]
pub struct PrepassPipeline<M: Material> {
    pub view_layout_motion_vectors: BindGroupLayout,
    pub view_layout_no_motion_vectors: BindGroupLayout,
    pub mesh_layouts: MeshLayouts,
    pub material_layout: BindGroupLayout,
    pub prepass_material_vertex_shader: Option<Handle<Shader>>,
    pub prepass_material_fragment_shader: Option<Handle<Shader>>,
    pub deferred_material_vertex_shader: Option<Handle<Shader>>,
    pub deferred_material_fragment_shader: Option<Handle<Shader>>,
    pub material_pipeline: MaterialPipeline<M>,
    _marker: PhantomData<M>,
}

impl<M: Material> FromWorld for PrepassPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let view_layout_motion_vectors = render_device.create_bind_group_layout(
            "prepass_view_layout_motion_vectors",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // View
                    uniform_buffer::<ViewUniform>(true),
                    // Globals
                    uniform_buffer::<GlobalsUniform>(false),
                    // PreviousViewProjection
                    uniform_buffer::<PreviousViewProjection>(true),
                ),
            ),
        );

        let view_layout_no_motion_vectors = render_device.create_bind_group_layout(
            "prepass_view_layout_no_motion_vectors",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // View
                    uniform_buffer::<ViewUniform>(true),
                    // Globals
                    uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        let mesh_pipeline = world.resource::<MeshPipeline>();

        PrepassPipeline {
            view_layout_motion_vectors,
            view_layout_no_motion_vectors,
            mesh_layouts: mesh_pipeline.mesh_layouts.clone(),
            prepass_material_vertex_shader: match M::prepass_vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            prepass_material_fragment_shader: match M::prepass_fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            deferred_material_vertex_shader: match M::deferred_vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            deferred_material_fragment_shader: match M::deferred_fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            material_layout: M::bind_group_layout(render_device),
            material_pipeline: world.resource::<MaterialPipeline<M>>().clone(),
            _marker: PhantomData,
        }
    }
}

impl<M: Material> SpecializedMeshPipeline for PrepassPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = MaterialPipelineKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut bind_group_layouts = vec![if key
            .mesh_key
            .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
        {
            self.view_layout_motion_vectors.clone()
        } else {
            self.view_layout_no_motion_vectors.clone()
        }];
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        // Let the shader code know that it's running in a prepass pipeline.
        // (PBR code will use this to detect that it's running in deferred mode,
        // since that's the only time it gets called from a prepass pipeline.)
        shader_defs.push("PREPASS_PIPELINE".into());

        // NOTE: Eventually, it would be nice to only add this when the shaders are overloaded by the Material.
        // The main limitation right now is that bind group order is hardcoded in shaders.
        bind_group_layouts.push(self.material_layout.clone());

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("WEBGL2".into());

        shader_defs.push("VERTEX_OUTPUT_INSTANCE_INDEX".into());

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.mesh_key.contains(MeshPipelineKey::MAY_DISCARD) {
            shader_defs.push("MAY_DISCARD".into());
        }

        let blend_key = key
            .mesh_key
            .intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
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

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_CLAMP_ORTHO) {
            shader_defs.push("DEPTH_CLAMP_ORTHO".into());
            // PERF: This line forces the "prepass fragment shader" to always run in
            // common scenarios like "directional light calculation". Doing so resolves
            // a pretty nasty depth clamping bug, but it also feels a bit excessive.
            // We should try to find a way to resolve this without forcing the fragment
            // shader to run.
            // https://github.com/bevyengine/bevy/pull/8877
            shader_defs.push("PREPASS_FRAGMENT".into());
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(1));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_1) {
            shader_defs.push("VERTEX_UVS_B".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_1.at_shader_location(2));
        }

        if key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if key
            .mesh_key
            .intersects(MeshPipelineKey::NORMAL_PREPASS | MeshPipelineKey::DEFERRED_PREPASS)
        {
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(3));
            shader_defs.push("NORMAL_PREPASS_OR_DEFERRED_PREPASS".into());
            if layout.0.contains(Mesh::ATTRIBUTE_TANGENT) {
                shader_defs.push("VERTEX_TANGENTS".into());
                vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(4));
            }
        }

        if key
            .mesh_key
            .intersects(MeshPipelineKey::MOTION_VECTOR_PREPASS | MeshPipelineKey::DEFERRED_PREPASS)
        {
            shader_defs.push("MOTION_VECTOR_PREPASS_OR_DEFERRED_PREPASS".into());
        }

        if key.mesh_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            shader_defs.push("DEFERRED_PREPASS".into());
        }

        if layout.0.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(7));
        }

        if key
            .mesh_key
            .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
        {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        if key.mesh_key.intersects(
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
            &key.mesh_key,
            &mut shader_defs,
            &mut vertex_attributes,
        );
        bind_group_layouts.insert(1, bind_group);

        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        // Setup prepass fragment targets - normals in slot 0 (or None if not needed), motion vectors in slot 1
        let mut targets = vec![
            key.mesh_key
                .contains(MeshPipelineKey::NORMAL_PREPASS)
                .then_some(ColorTargetState {
                    format: NORMAL_PREPASS_FORMAT,
                    // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            key.mesh_key
                .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
                .then_some(ColorTargetState {
                    format: MOTION_VECTOR_PREPASS_FORMAT,
                    // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            key.mesh_key
                .contains(MeshPipelineKey::DEFERRED_PREPASS)
                .then_some(ColorTargetState {
                    format: DEFERRED_PREPASS_FORMAT,
                    // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            key.mesh_key
                .contains(MeshPipelineKey::DEFERRED_PREPASS)
                .then_some(ColorTargetState {
                    format: DEFERRED_LIGHTING_PASS_ID_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
        ];

        if targets.iter().all(Option::is_none) {
            // if no targets are required then clear the list, so that no fragment shader is required
            // (though one may still be used for discarding depth buffer writes)
            targets.clear();
        }

        // The fragment shader is only used when the normal prepass or motion vectors prepass
        // is enabled or the material uses alpha cutoff values and doesn't rely on the standard
        // prepass shader or we are clamping the orthographic depth.
        let fragment_required = !targets.is_empty()
            || key.mesh_key.contains(MeshPipelineKey::DEPTH_CLAMP_ORTHO)
            || (key.mesh_key.contains(MeshPipelineKey::MAY_DISCARD)
                && self.prepass_material_fragment_shader.is_some());

        let fragment = fragment_required.then(|| {
            // Use the fragment shader from the material
            let frag_shader_handle = if key.mesh_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
                match self.deferred_material_fragment_shader.clone() {
                    Some(frag_shader_handle) => frag_shader_handle,
                    _ => PREPASS_SHADER_HANDLE,
                }
            } else {
                match self.prepass_material_fragment_shader.clone() {
                    Some(frag_shader_handle) => frag_shader_handle,
                    _ => PREPASS_SHADER_HANDLE,
                }
            };

            FragmentState {
                shader: frag_shader_handle,
                entry_point: "fragment".into(),
                shader_defs: shader_defs.clone(),
                targets,
            }
        });

        // Use the vertex shader from the material if present
        let vert_shader_handle = if key.mesh_key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            if let Some(handle) = &self.deferred_material_vertex_shader {
                handle.clone()
            } else {
                PREPASS_SHADER_HANDLE
            }
        } else if let Some(handle) = &self.prepass_material_vertex_shader {
            handle.clone()
        } else {
            PREPASS_SHADER_HANDLE
        };

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

        let mut descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: vert_shader_handle,
                entry_point: "vertex".into(),
                shader_defs,
                buffers: vec![vertex_buffer_layout],
            },
            fragment,
            layout: bind_group_layouts,
            primitive: PrimitiveState {
                topology: key.mesh_key.primitive_topology(),
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
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
                count: key.mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges,
            label: Some("prepass_pipeline".into()),
        };

        // This is a bit risky because it's possible to change something that would
        // break the prepass but be fine in the main pass.
        // Since this api is pretty low-level it doesn't matter that much, but it is a potential issue.
        M::specialize(&self.material_pipeline, &mut descriptor, layout, key)?;

        Ok(descriptor)
    }
}

// Extract the render phases for the prepass
pub fn extract_camera_previous_view_projection(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera, Option<&PreviousViewProjection>), With<Camera3d>>>,
) {
    for (entity, camera, maybe_previous_view_proj) in cameras_3d.iter() {
        if camera.is_active {
            let mut entity = commands.get_or_spawn(entity);

            if let Some(previous_view) = maybe_previous_view_proj {
                entity.insert(previous_view.clone());
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct PreviousViewProjectionUniforms {
    pub uniforms: DynamicUniformBuffer<PreviousViewProjection>,
}

#[derive(Component)]
pub struct PreviousViewProjectionUniformOffset {
    pub offset: u32,
}

pub fn prepare_previous_view_projection_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<PreviousViewProjectionUniforms>,
    views: Query<
        (Entity, &ExtractedView, Option<&PreviousViewProjection>),
        With<MotionVectorPrepass>,
    >,
) {
    let views_iter = views.iter();
    let view_count = views_iter.len();
    let Some(mut writer) =
        view_uniforms
            .uniforms
            .get_writer(view_count, &render_device, &render_queue)
    else {
        return;
    };
    for (entity, camera, maybe_previous_view_proj) in views_iter {
        let view_projection = match maybe_previous_view_proj {
            Some(previous_view) => previous_view.clone(),
            None => PreviousViewProjection {
                view_proj: camera.projection * camera.transform.compute_matrix().inverse(),
            },
        };
        commands
            .entity(entity)
            .insert(PreviousViewProjectionUniformOffset {
                offset: writer.write(&view_projection),
            });
    }
}

#[derive(Default, Resource)]
pub struct PrepassViewBindGroup {
    pub motion_vectors: Option<BindGroup>,
    pub no_motion_vectors: Option<BindGroup>,
}

pub fn prepare_prepass_view_bind_group<M: Material>(
    render_device: Res<RenderDevice>,
    prepass_pipeline: Res<PrepassPipeline<M>>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    previous_view_proj_uniforms: Res<PreviousViewProjectionUniforms>,
    mut prepass_view_bind_group: ResMut<PrepassViewBindGroup>,
) {
    if let (Some(view_binding), Some(globals_binding)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        prepass_view_bind_group.no_motion_vectors = Some(render_device.create_bind_group(
            "prepass_view_no_motion_vectors_bind_group",
            &prepass_pipeline.view_layout_no_motion_vectors,
            &BindGroupEntries::sequential((view_binding.clone(), globals_binding.clone())),
        ));

        if let Some(previous_view_proj_binding) = previous_view_proj_uniforms.uniforms.binding() {
            prepass_view_bind_group.motion_vectors = Some(render_device.create_bind_group(
                "prepass_view_motion_vectors_bind_group",
                &prepass_pipeline.view_layout_motion_vectors,
                &BindGroupEntries::sequential((
                    view_binding,
                    globals_binding,
                    previous_view_proj_binding,
                )),
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_prepass_material_meshes<M: Material>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3dPrepass>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3dPrepass>>,
    opaque_deferred_draw_functions: Res<DrawFunctions<Opaque3dDeferred>>,
    alpha_mask_deferred_draw_functions: Res<DrawFunctions<AlphaMask3dDeferred>>,
    prepass_pipeline: Res<PrepassPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<PrepassPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_materials: Res<RenderMaterials<M>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    render_lightmaps: Res<RenderLightmaps>,
    mut views: Query<
        (
            &ExtractedView,
            &VisibleEntities,
            Option<&mut BinnedRenderPhase<Opaque3dPrepass>>,
            Option<&mut BinnedRenderPhase<AlphaMask3dPrepass>>,
            Option<&mut BinnedRenderPhase<Opaque3dDeferred>>,
            Option<&mut BinnedRenderPhase<AlphaMask3dDeferred>>,
            Option<&DepthPrepass>,
            Option<&NormalPrepass>,
            Option<&MotionVectorPrepass>,
            Option<&DeferredPrepass>,
        ),
        Or<(
            With<BinnedRenderPhase<Opaque3dPrepass>>,
            With<BinnedRenderPhase<AlphaMask3dPrepass>>,
            With<BinnedRenderPhase<Opaque3dDeferred>>,
            With<BinnedRenderPhase<AlphaMask3dDeferred>>,
        )>,
    >,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let opaque_draw_prepass = opaque_draw_functions
        .read()
        .get_id::<DrawPrepass<M>>()
        .unwrap();
    let alpha_mask_draw_prepass = alpha_mask_draw_functions
        .read()
        .get_id::<DrawPrepass<M>>()
        .unwrap();
    let opaque_draw_deferred = opaque_deferred_draw_functions
        .read()
        .get_id::<DrawPrepass<M>>()
        .unwrap();
    let alpha_mask_draw_deferred = alpha_mask_deferred_draw_functions
        .read()
        .get_id::<DrawPrepass<M>>()
        .unwrap();
    for (
        _view,
        visible_entities,
        mut opaque_phase,
        mut alpha_mask_phase,
        mut opaque_deferred_phase,
        mut alpha_mask_deferred_phase,
        depth_prepass,
        normal_prepass,
        motion_vector_prepass,
        deferred_prepass,
    ) in &mut views
    {
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

        for visible_entity in &visible_entities.entities {
            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.get(visible_entity) else {
                continue;
            };
            let Some(material) = render_materials.get(material_asset_id) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let mut mesh_key = view_key | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

            let alpha_mode = material.properties.alpha_mode;
            match alpha_mode {
                AlphaMode::Opaque => {}
                AlphaMode::Mask(_) => mesh_key |= MeshPipelineKey::MAY_DISCARD,
                AlphaMode::Blend
                | AlphaMode::Premultiplied
                | AlphaMode::Add
                | AlphaMode::Multiply => continue,
            }

            if material.properties.reads_view_transmission_texture {
                // No-op: Materials reading from `ViewTransmissionTexture` are not rendered in the `Opaque3d`
                // phase, and are therefore also excluded from the prepass much like alpha-blended materials.
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

            // Even though we don't use the lightmap in the prepass, the
            // `SetMeshBindGroup` render command will bind the data for it. So
            // we need to include the appropriate flag in the mesh pipeline key
            // to ensure that the necessary bind group layout entries are
            // present.
            if render_lightmaps
                .render_lightmaps
                .contains_key(visible_entity)
            {
                mesh_key |= MeshPipelineKey::LIGHTMAPPED;
            }

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &prepass_pipeline,
                MaterialPipelineKey {
                    mesh_key,
                    bind_group_data: material.key.clone(),
                },
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            match alpha_mode {
                AlphaMode::Opaque => {
                    if deferred {
                        opaque_deferred_phase.as_mut().unwrap().add(
                            OpaqueNoLightmap3dBinKey {
                                draw_function: opaque_draw_deferred,
                                pipeline: pipeline_id,
                                asset_id: mesh_instance.mesh_asset_id,
                                material_bind_group_id: material.get_bind_group_id().0,
                            },
                            *visible_entity,
                            mesh_instance.should_batch(),
                        );
                    } else if let Some(opaque_phase) = opaque_phase.as_mut() {
                        opaque_phase.add(
                            OpaqueNoLightmap3dBinKey {
                                draw_function: opaque_draw_prepass,
                                pipeline: pipeline_id,
                                asset_id: mesh_instance.mesh_asset_id,
                                material_bind_group_id: material.get_bind_group_id().0,
                            },
                            *visible_entity,
                            mesh_instance.should_batch(),
                        );
                    }
                }
                AlphaMode::Mask(_) => {
                    if deferred {
                        let bin_key = OpaqueNoLightmap3dBinKey {
                            pipeline: pipeline_id,
                            draw_function: alpha_mask_draw_deferred,
                            asset_id: mesh_instance.mesh_asset_id,
                            material_bind_group_id: material.get_bind_group_id().0,
                        };
                        alpha_mask_deferred_phase.as_mut().unwrap().add(
                            bin_key,
                            *visible_entity,
                            mesh_instance.should_batch(),
                        );
                    } else if let Some(alpha_mask_phase) = alpha_mask_phase.as_mut() {
                        let bin_key = OpaqueNoLightmap3dBinKey {
                            pipeline: pipeline_id,
                            draw_function: alpha_mask_draw_prepass,
                            asset_id: mesh_instance.mesh_asset_id,
                            material_bind_group_id: material.get_bind_group_id().0,
                        };
                        alpha_mask_phase.add(
                            bin_key,
                            *visible_entity,
                            mesh_instance.should_batch(),
                        );
                    }
                }
                AlphaMode::Blend
                | AlphaMode::Premultiplied
                | AlphaMode::Add
                | AlphaMode::Multiply => {}
            }
        }
    }
}

pub struct SetPrepassViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetPrepassViewBindGroup<I> {
    type Param = SRes<PrepassViewBindGroup>;
    type ViewQuery = (
        Read<ViewUniformOffset>,
        Option<Read<PreviousViewProjectionUniformOffset>>,
    );
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform_offset, previous_view_projection_uniform_offset): (
            &'_ ViewUniformOffset,
            Option<&'_ PreviousViewProjectionUniformOffset>,
        ),
        _entity: Option<()>,
        prepass_view_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let prepass_view_bind_group = prepass_view_bind_group.into_inner();

        if let Some(previous_view_projection_uniform_offset) =
            previous_view_projection_uniform_offset
        {
            pass.set_bind_group(
                I,
                prepass_view_bind_group.motion_vectors.as_ref().unwrap(),
                &[
                    view_uniform_offset.offset,
                    previous_view_projection_uniform_offset.offset,
                ],
            );
        } else {
            pass.set_bind_group(
                I,
                prepass_view_bind_group.no_motion_vectors.as_ref().unwrap(),
                &[view_uniform_offset.offset],
            );
        }

        RenderCommandResult::Success
    }
}

pub type DrawPrepass<M> = (
    SetItemPipeline,
    SetPrepassViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetMaterialBindGroup<M, 2>,
    DrawMesh,
);
