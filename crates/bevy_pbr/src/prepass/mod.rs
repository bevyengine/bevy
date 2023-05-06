use bevy_app::{Plugin, PreUpdate, Update};
use bevy_asset::{load_internal_asset, AssetServer, Handle, HandleUntyped};
use bevy_core_pipeline::{
    prelude::Camera3d,
    prepass::{
        AlphaMask3dPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass, Opaque3dPrepass,
        ViewPrepassTextures, DEPTH_PREPASS_FORMAT, MOTION_VECTOR_PREPASS_FORMAT,
        NORMAL_PREPASS_FORMAT,
    },
};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_math::Mat4;
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::ExtractedCamera,
    globals::{GlobalsBuffer, GlobalsUniform},
    mesh::MeshVertexBufferLayout,
    prelude::{Camera, Mesh},
    render_asset::RenderAssets,
    render_phase::{
        sort_phase_system, AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand,
        RenderCommandResult, RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, BlendState, BufferBindingType,
        ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
        DynamicUniformBuffer, Extent3d, FragmentState, FrontFace, MultisampleState, PipelineCache,
        PolygonMode, PrimitiveState, RenderPipelineDescriptor, Shader, ShaderDefVal, ShaderRef,
        ShaderStages, ShaderType, SpecializedMeshPipeline, SpecializedMeshPipelineError,
        SpecializedMeshPipelines, StencilFaceState, StencilState, TextureDescriptor,
        TextureDimension, TextureSampleType, TextureUsages, TextureViewDimension, VertexState,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::{FallbackImagesDepth, FallbackImagesMsaa, TextureCache},
    view::{ExtractedView, Msaa, ViewUniform, ViewUniformOffset, ViewUniforms, VisibleEntities},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{tracing::error, HashMap};

use crate::{
    prepare_lights, AlphaMode, DrawMesh, Material, MaterialPipeline, MaterialPipelineKey,
    MeshPipeline, MeshPipelineKey, MeshUniform, RenderMaterials, SetMaterialBindGroup,
    SetMeshBindGroup, MAX_CASCADES_PER_LIGHT, MAX_DIRECTIONAL_LIGHTS,
};

use std::{hash::Hash, marker::PhantomData};

pub const PREPASS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 921124473254008983);

pub const PREPASS_BINDINGS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5533152893177403494);

pub const PREPASS_UTILS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4603948296044544);

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
    fn build(&self, app: &mut bevy_app::App) {
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

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                Render,
                queue_prepass_view_bind_group::<M>.in_set(RenderSet::Queue),
            )
            .init_resource::<PrepassViewBindGroup>()
            .init_resource::<SpecializedMeshPipelines<PrepassPipeline<M>>>()
            .init_resource::<PreviousViewProjectionUniforms>();
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
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
    fn build(&self, app: &mut bevy_app::App) {
        let no_prepass_plugin_loaded = app.world.get_resource::<AnyPrepassPluginLoaded>().is_none();

        if no_prepass_plugin_loaded {
            app.insert_resource(AnyPrepassPluginLoaded)
                .add_systems(Update, update_previous_view_projections)
                // At the start of each frame, last frame's GlobalTransforms become this frame's PreviousGlobalTransforms
                .add_systems(PreUpdate, update_mesh_previous_global_transforms);
        }

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if no_prepass_plugin_loaded {
            render_app
                .init_resource::<DrawFunctions<Opaque3dPrepass>>()
                .init_resource::<DrawFunctions<AlphaMask3dPrepass>>()
                .add_systems(ExtractSchedule, extract_camera_prepass_phase)
                .add_systems(
                    Render,
                    (
                        prepare_prepass_textures
                            .in_set(RenderSet::Prepare)
                            .after(bevy_render::view::prepare_windows),
                        prepare_previous_view_projection_uniforms
                            .in_set(RenderSet::Prepare)
                            .after(PrepassLightsViewFlush),
                        apply_system_buffers
                            .in_set(RenderSet::Prepare)
                            .in_set(PrepassLightsViewFlush)
                            .after(prepare_lights),
                        sort_phase_system::<Opaque3dPrepass>.in_set(RenderSet::PhaseSort),
                        sort_phase_system::<AlphaMask3dPrepass>.in_set(RenderSet::PhaseSort),
                    ),
                );
        }

        render_app
            .add_render_command::<Opaque3dPrepass, DrawPrepass<M>>()
            .add_render_command::<AlphaMask3dPrepass, DrawPrepass<M>>()
            .add_systems(
                Render,
                queue_prepass_material_meshes::<M>.in_set(RenderSet::Queue),
            );
    }
}

#[derive(Resource)]
struct AnyPrepassPluginLoaded;

#[derive(Component, ShaderType, Clone)]
pub struct PreviousViewProjection {
    pub view_proj: Mat4,
}

pub fn update_previous_view_projections(
    mut commands: Commands,
    query: Query<(Entity, &Camera, &GlobalTransform), (With<Camera3d>, With<MotionVectorPrepass>)>,
) {
    for (entity, camera, camera_transform) in &query {
        commands.entity(entity).insert(PreviousViewProjection {
            view_proj: camera.projection_matrix() * camera_transform.compute_matrix().inverse(),
        });
    }
}

#[derive(Component)]
pub struct PreviousGlobalTransform(pub Mat4);

pub fn update_mesh_previous_global_transforms(
    mut commands: Commands,
    views: Query<&Camera, (With<Camera3d>, With<MotionVectorPrepass>)>,
    meshes: Query<(Entity, &GlobalTransform), With<Handle<Mesh>>>,
) {
    let should_run = views.iter().any(|camera| camera.is_active);

    if should_run {
        for (entity, transform) in &meshes {
            commands
                .entity(entity)
                .insert(PreviousGlobalTransform(transform.compute_matrix()));
        }
    }
}

#[derive(Resource)]
pub struct PrepassPipeline<M: Material> {
    pub view_layout_motion_vectors: BindGroupLayout,
    pub view_layout_no_motion_vectors: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub skinned_mesh_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub material_vertex_shader: Option<Handle<Shader>>,
    pub material_fragment_shader: Option<Handle<Shader>>,
    pub material_pipeline: MaterialPipeline<M>,
    _marker: PhantomData<M>,
}

impl<M: Material> FromWorld for PrepassPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let view_layout_motion_vectors =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    // View
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(ViewUniform::min_size()),
                        },
                        count: None,
                    },
                    // Globals
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(GlobalsUniform::min_size()),
                        },
                        count: None,
                    },
                    // PreviousViewProjection
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(PreviousViewProjection::min_size()),
                        },
                        count: None,
                    },
                ],
                label: Some("prepass_view_layout_motion_vectors"),
            });

        let view_layout_no_motion_vectors =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    // View
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(ViewUniform::min_size()),
                        },
                        count: None,
                    },
                    // Globals
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(GlobalsUniform::min_size()),
                        },
                        count: None,
                    },
                ],
                label: Some("prepass_view_layout_no_motion_vectors"),
            });

        let mesh_pipeline = world.resource::<MeshPipeline>();

        PrepassPipeline {
            view_layout_motion_vectors,
            view_layout_no_motion_vectors,
            mesh_layout: mesh_pipeline.mesh_layout.clone(),
            skinned_mesh_layout: mesh_pipeline.skinned_mesh_layout.clone(),
            material_vertex_shader: match M::prepass_vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            material_fragment_shader: match M::prepass_fragment_shader() {
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
        layout: &MeshVertexBufferLayout,
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

        // NOTE: Eventually, it would be nice to only add this when the shaders are overloaded by the Material.
        // The main limitation right now is that bind group order is hardcoded in shaders.
        bind_group_layouts.insert(1, self.material_layout.clone());

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.mesh_key.contains(MeshPipelineKey::ALPHA_MASK) {
            shader_defs.push("ALPHA_MASK".into());
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

        if layout.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        shader_defs.push(ShaderDefVal::UInt(
            "MAX_DIRECTIONAL_LIGHTS".to_string(),
            MAX_DIRECTIONAL_LIGHTS as u32,
        ));
        shader_defs.push(ShaderDefVal::UInt(
            "MAX_CASCADES_PER_LIGHT".to_string(),
            MAX_CASCADES_PER_LIGHT as u32,
        ));
        if key.mesh_key.contains(MeshPipelineKey::DEPTH_CLAMP_ORTHO) {
            shader_defs.push("DEPTH_CLAMP_ORTHO".into());
        }

        if layout.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(1));
        }

        if key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(2));
            shader_defs.push("NORMAL_PREPASS".into());

            if layout.contains(Mesh::ATTRIBUTE_TANGENT) {
                shader_defs.push("VERTEX_TANGENTS".into());
                vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(3));
            }
        }

        if key
            .mesh_key
            .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
        {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        if key
            .mesh_key
            .intersects(MeshPipelineKey::NORMAL_PREPASS | MeshPipelineKey::MOTION_VECTOR_PREPASS)
        {
            shader_defs.push("PREPASS_FRAGMENT".into());
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            shader_defs.push("SKINNED".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
            bind_group_layouts.insert(2, self.skinned_mesh_layout.clone());
        } else {
            bind_group_layouts.insert(2, self.mesh_layout.clone());
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        // Setup prepass fragment targets - normals in slot 0 (or None if not needed), motion vectors in slot 1
        let mut targets = vec![];
        targets.push(
            key.mesh_key
                .contains(MeshPipelineKey::NORMAL_PREPASS)
                .then_some(ColorTargetState {
                    format: NORMAL_PREPASS_FORMAT,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }),
        );
        targets.push(
            key.mesh_key
                .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
                .then_some(ColorTargetState {
                    format: MOTION_VECTOR_PREPASS_FORMAT,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }),
        );
        if targets.iter().all(Option::is_none) {
            // if no targets are required then clear the list, so that no fragment shader is required
            // (though one may still be used for discarding depth buffer writes)
            targets.clear();
        }

        // The fragment shader is only used when the normal prepass or motion vectors prepass
        // is enabled or the material uses alpha cutoff values and doesn't rely on the standard
        // prepass shader
        let fragment_required = !targets.is_empty()
            || ((key.mesh_key.contains(MeshPipelineKey::ALPHA_MASK)
                || blend_key == MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA
                || blend_key == MeshPipelineKey::BLEND_ALPHA)
                && self.material_fragment_shader.is_some());

        let fragment = fragment_required.then(|| {
            // Use the fragment shader from the material
            let frag_shader_handle = match self.material_fragment_shader.clone() {
                Some(frag_shader_handle) => frag_shader_handle,
                _ => PREPASS_SHADER_HANDLE.typed::<Shader>(),
            };

            FragmentState {
                shader: frag_shader_handle,
                entry_point: "fragment".into(),
                shader_defs: shader_defs.clone(),
                targets,
            }
        });

        // Use the vertex shader from the material if present
        let vert_shader_handle = if let Some(handle) = &self.material_vertex_shader {
            handle.clone()
        } else {
            PREPASS_SHADER_HANDLE.typed::<Shader>()
        };

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
                format: DEPTH_PREPASS_FORMAT,
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
            push_constant_ranges: Vec::new(),
            label: Some("prepass_pipeline".into()),
        };

        // This is a bit risky because it's possible to change something that would
        // break the prepass but be fine in the main pass.
        // Since this api is pretty low-level it doesn't matter that much, but it is a potential issue.
        M::specialize(&self.material_pipeline, &mut descriptor, layout, key)?;

        Ok(descriptor)
    }
}

pub fn get_bind_group_layout_entries(
    bindings: [u32; 3],
    multisampled: bool,
) -> [BindGroupLayoutEntry; 3] {
    [
        // Depth texture
        BindGroupLayoutEntry {
            binding: bindings[0],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled,
                sample_type: TextureSampleType::Depth,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // Normal texture
        BindGroupLayoutEntry {
            binding: bindings[1],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled,
                sample_type: TextureSampleType::Float { filterable: false },
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // Motion Vectors texture
        BindGroupLayoutEntry {
            binding: bindings[2],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled,
                sample_type: TextureSampleType::Float { filterable: false },
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
    ]
}

pub fn get_bindings<'a>(
    prepass_textures: Option<&'a ViewPrepassTextures>,
    fallback_images: &'a mut FallbackImagesMsaa,
    fallback_depths: &'a mut FallbackImagesDepth,
    msaa: &'a Msaa,
    bindings: [u32; 3],
) -> [BindGroupEntry<'a>; 3] {
    let depth_view = match prepass_textures.and_then(|x| x.depth.as_ref()) {
        Some(texture) => &texture.default_view,
        None => {
            &fallback_depths
                .image_for_samplecount(msaa.samples())
                .texture_view
        }
    };

    let normal_motion_vectors_fallback = &fallback_images
        .image_for_samplecount(msaa.samples())
        .texture_view;

    let normal_view = match prepass_textures.and_then(|x| x.normal.as_ref()) {
        Some(texture) => &texture.default_view,
        None => normal_motion_vectors_fallback,
    };

    let motion_vectors_view = match prepass_textures.and_then(|x| x.motion_vectors.as_ref()) {
        Some(texture) => &texture.default_view,
        None => normal_motion_vectors_fallback,
    };

    [
        BindGroupEntry {
            binding: bindings[0],
            resource: BindingResource::TextureView(depth_view),
        },
        BindGroupEntry {
            binding: bindings[1],
            resource: BindingResource::TextureView(normal_view),
        },
        BindGroupEntry {
            binding: bindings[2],
            resource: BindingResource::TextureView(motion_vectors_view),
        },
    ]
}

// Extract the render phases for the prepass
pub fn extract_camera_prepass_phase(
    mut commands: Commands,
    cameras_3d: Extract<
        Query<
            (
                Entity,
                &Camera,
                Option<&DepthPrepass>,
                Option<&NormalPrepass>,
                Option<&MotionVectorPrepass>,
                Option<&PreviousViewProjection>,
            ),
            With<Camera3d>,
        >,
    >,
) {
    for (
        entity,
        camera,
        depth_prepass,
        normal_prepass,
        motion_vector_prepass,
        maybe_previous_view_proj,
    ) in cameras_3d.iter()
    {
        if camera.is_active {
            let mut entity = commands.get_or_spawn(entity);

            if depth_prepass.is_some()
                || normal_prepass.is_some()
                || motion_vector_prepass.is_some()
            {
                entity.insert((
                    RenderPhase::<Opaque3dPrepass>::default(),
                    RenderPhase::<AlphaMask3dPrepass>::default(),
                ));
            }

            if depth_prepass.is_some() {
                entity.insert(DepthPrepass);
            }
            if normal_prepass.is_some() {
                entity.insert(NormalPrepass);
            }
            if motion_vector_prepass.is_some() {
                entity.insert(MotionVectorPrepass);
            }

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
    view_uniforms.uniforms.clear();

    for (entity, camera, maybe_previous_view_proj) in &views {
        let view_projection = match maybe_previous_view_proj {
            Some(previous_view) => previous_view.clone(),
            None => PreviousViewProjection {
                view_proj: camera.projection * camera.transform.compute_matrix().inverse(),
            },
        };
        commands
            .entity(entity)
            .insert(PreviousViewProjectionUniformOffset {
                offset: view_uniforms.uniforms.push(view_projection),
            });
    }

    view_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

// Prepares the textures used by the prepass
pub fn prepare_prepass_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (
            Entity,
            &ExtractedCamera,
            Option<&DepthPrepass>,
            Option<&NormalPrepass>,
            Option<&MotionVectorPrepass>,
        ),
        (
            With<RenderPhase<Opaque3dPrepass>>,
            With<RenderPhase<AlphaMask3dPrepass>>,
        ),
    >,
) {
    let mut depth_textures = HashMap::default();
    let mut normal_textures = HashMap::default();
    let mut motion_vectors_textures = HashMap::default();
    for (entity, camera, depth_prepass, normal_prepass, motion_vector_prepass) in &views_3d {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let size = Extent3d {
            depth_or_array_layers: 1,
            width: physical_target_size.x,
            height: physical_target_size.y,
        };

        let cached_depth_texture = depth_prepass.is_some().then(|| {
            depth_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    let descriptor = TextureDescriptor {
                        label: Some("prepass_depth_texture"),
                        size,
                        mip_level_count: 1,
                        sample_count: msaa.samples(),
                        dimension: TextureDimension::D2,
                        format: DEPTH_PREPASS_FORMAT,
                        usage: TextureUsages::COPY_DST
                            | TextureUsages::RENDER_ATTACHMENT
                            | TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    };
                    texture_cache.get(&render_device, descriptor)
                })
                .clone()
        });

        let cached_normals_texture = normal_prepass.is_some().then(|| {
            normal_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("prepass_normal_texture"),
                            size,
                            mip_level_count: 1,
                            sample_count: msaa.samples(),
                            dimension: TextureDimension::D2,
                            format: NORMAL_PREPASS_FORMAT,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                    )
                })
                .clone()
        });

        let cached_motion_vectors_texture = motion_vector_prepass.is_some().then(|| {
            motion_vectors_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("prepass_motion_vectors_textures"),
                            size,
                            mip_level_count: 1,
                            sample_count: msaa.samples(),
                            dimension: TextureDimension::D2,
                            format: MOTION_VECTOR_PREPASS_FORMAT,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                    )
                })
                .clone()
        });

        commands.entity(entity).insert(ViewPrepassTextures {
            depth: cached_depth_texture,
            normal: cached_normals_texture,
            motion_vectors: cached_motion_vectors_texture,
            size,
        });
    }
}

#[derive(Default, Resource)]
pub struct PrepassViewBindGroup {
    motion_vectors: Option<BindGroup>,
    no_motion_vectors: Option<BindGroup>,
}

pub fn queue_prepass_view_bind_group<M: Material>(
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
        prepass_view_bind_group.no_motion_vectors =
            Some(render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: view_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: globals_binding.clone(),
                    },
                ],
                label: Some("prepass_view_no_motion_vectors_bind_group"),
                layout: &prepass_pipeline.view_layout_no_motion_vectors,
            }));

        if let Some(previous_view_proj_binding) = previous_view_proj_uniforms.uniforms.binding() {
            prepass_view_bind_group.motion_vectors =
                Some(render_device.create_bind_group(&BindGroupDescriptor {
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: view_binding,
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: globals_binding,
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: previous_view_proj_binding,
                        },
                    ],
                    label: Some("prepass_view_motion_vectors_bind_group"),
                    layout: &prepass_pipeline.view_layout_motion_vectors,
                }));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_prepass_material_meshes<M: Material>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3dPrepass>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3dPrepass>>,
    prepass_pipeline: Res<PrepassPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<PrepassPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderMaterials<M>>,
    material_meshes: Query<(&Handle<M>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Opaque3dPrepass>,
        &mut RenderPhase<AlphaMask3dPrepass>,
        Option<&DepthPrepass>,
        Option<&NormalPrepass>,
        Option<&MotionVectorPrepass>,
    )>,
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
    for (
        view,
        visible_entities,
        mut opaque_phase,
        mut alpha_mask_phase,
        depth_prepass,
        normal_prepass,
        motion_vector_prepass,
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

        let rangefinder = view.rangefinder3d();

        for visible_entity in &visible_entities.entities {
            let Ok((material_handle, mesh_handle, mesh_uniform)) = material_meshes.get(*visible_entity) else {
                continue;
            };

            let (Some(material), Some(mesh)) = (
                render_materials.get(material_handle),
                render_meshes.get(mesh_handle),
            ) else {
                continue;
            };

            let mut mesh_key =
                MeshPipelineKey::from_primitive_topology(mesh.primitive_topology) | view_key;
            let alpha_mode = material.properties.alpha_mode;
            match alpha_mode {
                AlphaMode::Opaque => {}
                AlphaMode::Mask(_) => mesh_key |= MeshPipelineKey::ALPHA_MASK,
                AlphaMode::Blend
                | AlphaMode::Premultiplied
                | AlphaMode::Add
                | AlphaMode::Multiply => continue,
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

            let distance =
                rangefinder.distance(&mesh_uniform.transform) + material.properties.depth_bias;
            match alpha_mode {
                AlphaMode::Opaque => {
                    opaque_phase.add(Opaque3dPrepass {
                        entity: *visible_entity,
                        draw_function: opaque_draw_prepass,
                        pipeline_id,
                        distance,
                    });
                }
                AlphaMode::Mask(_) => {
                    alpha_mask_phase.add(AlphaMask3dPrepass {
                        entity: *visible_entity,
                        draw_function: alpha_mask_draw_prepass,
                        pipeline_id,
                        distance,
                    });
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
    type ViewWorldQuery = (
        Read<ViewUniformOffset>,
        Option<Read<PreviousViewProjectionUniformOffset>>,
    );
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform_offset, previous_view_projection_uniform_offset): (
            &'_ ViewUniformOffset,
            Option<&'_ PreviousViewProjectionUniformOffset>,
        ),
        _entity: (),
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
    SetMaterialBindGroup<M, 1>,
    SetMeshBindGroup<2>,
    DrawMesh,
);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct PrepassLightsViewFlush;
