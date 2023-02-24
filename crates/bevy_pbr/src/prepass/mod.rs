use bevy_app::{IntoSystemAppConfig, Plugin};
use bevy_asset::{load_internal_asset, AssetServer, Handle, HandleUntyped};
use bevy_core_pipeline::{
    prelude::Camera3d,
    prepass::{
        AlphaMask3dPrepass, DepthPrepass, NormalPrepass, Opaque3dPrepass, ViewPrepassTextures,
        DEPTH_PREPASS_FORMAT, NORMAL_PREPASS_FORMAT,
    },
};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::ExtractedCamera,
    mesh::MeshVertexBufferLayout,
    prelude::{Camera, Mesh},
    render_asset::RenderAssets,
    render_phase::{
        sort_phase_system, AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand,
        RenderCommandResult, RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType, ColorTargetState,
        ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Extent3d, FragmentState,
        FrontFace, MultisampleState, PipelineCache, PolygonMode, PrimitiveState,
        RenderPipelineDescriptor, Shader, ShaderDefVal, ShaderRef, ShaderStages, ShaderType,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
        StencilFaceState, StencilState, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages, VertexState,
    },
    renderer::RenderDevice,
    texture::TextureCache,
    view::{ExtractedView, Msaa, ViewUniform, ViewUniformOffset, ViewUniforms, VisibleEntities},
    Extract, ExtractSchedule, RenderApp, RenderSet,
};
use bevy_utils::{tracing::error, HashMap};

use crate::{
    AlphaMode, DrawMesh, Material, MaterialPipeline, MaterialPipelineKey, MeshPipeline,
    MeshPipelineKey, MeshUniform, RenderMaterials, SetMaterialBindGroup, SetMeshBindGroup,
    MAX_CASCADES_PER_LIGHT, MAX_DIRECTIONAL_LIGHTS,
};

use std::{hash::Hash, marker::PhantomData};

pub const PREPASS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 921124473254008983);

pub const PREPASS_BINDINGS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5533152893177403494);

pub const PREPASS_UTILS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4603948296044544);

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
            .add_system(extract_camera_prepass_phase.in_schedule(ExtractSchedule))
            .add_system(
                prepare_prepass_textures
                    .in_set(RenderSet::Prepare)
                    .after(bevy_render::view::prepare_windows),
            )
            .add_system(queue_prepass_view_bind_group::<M>.in_set(RenderSet::Queue))
            .add_system(queue_prepass_material_meshes::<M>.in_set(RenderSet::Queue))
            .add_system(sort_phase_system::<Opaque3dPrepass>.in_set(RenderSet::PhaseSort))
            .add_system(sort_phase_system::<AlphaMask3dPrepass>.in_set(RenderSet::PhaseSort))
            .init_resource::<PrepassPipeline<M>>()
            .init_resource::<DrawFunctions<Opaque3dPrepass>>()
            .init_resource::<DrawFunctions<AlphaMask3dPrepass>>()
            .init_resource::<PrepassViewBindGroup>()
            .init_resource::<SpecializedMeshPipelines<PrepassPipeline<M>>>()
            .add_render_command::<Opaque3dPrepass, DrawPrepass<M>>()
            .add_render_command::<AlphaMask3dPrepass, DrawPrepass<M>>();
    }
}

#[derive(Resource)]
pub struct PrepassPipeline<M: Material> {
    pub view_layout: BindGroupLayout,
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

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            ],
            label: Some("prepass_view_layout"),
        });

        let mesh_pipeline = world.resource::<MeshPipeline>();

        PrepassPipeline {
            view_layout,
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
        let mut bind_group_layout = vec![self.view_layout.clone()];
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        // NOTE: Eventually, it would be nice to only add this when the shaders are overloaded by the Material.
        // The main limitation right now is that bind group order is hardcoded in shaders.
        bind_group_layout.insert(1, self.material_layout.clone());

        if key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.mesh_key.contains(MeshPipelineKey::ALPHA_MASK) {
            shader_defs.push("ALPHA_MASK".into());
        }

        if layout.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        shader_defs.push(ShaderDefVal::Int(
            "MAX_DIRECTIONAL_LIGHTS".to_string(),
            MAX_DIRECTIONAL_LIGHTS as i32,
        ));
        shader_defs.push(ShaderDefVal::Int(
            "MAX_CASCADES_PER_LIGHT".to_string(),
            MAX_CASCADES_PER_LIGHT as i32,
        ));

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

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            shader_defs.push("SKINNED".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
            bind_group_layout.insert(2, self.skinned_mesh_layout.clone());
        } else {
            bind_group_layout.insert(2, self.mesh_layout.clone());
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        // The fragment shader is only used when the normal prepass is enabled or the material uses an alpha mask
        let fragment = if key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS)
            || key.mesh_key.contains(MeshPipelineKey::ALPHA_MASK)
        {
            // Use the fragment shader from the material if present
            let frag_shader_handle = if let Some(handle) = &self.material_fragment_shader {
                handle.clone()
            } else {
                PREPASS_SHADER_HANDLE.typed::<Shader>()
            };

            let mut targets = vec![];
            // When the normal prepass is enabled we need a target to be able to write to it.
            if key.mesh_key.contains(MeshPipelineKey::NORMAL_PREPASS) {
                targets.push(Some(ColorTargetState {
                    format: TextureFormat::Rgb10a2Unorm,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }));
            }

            Some(FragmentState {
                shader: frag_shader_handle,
                entry_point: "fragment".into(),
                shader_defs: shader_defs.clone(),
                targets,
            })
        } else {
            None
        };

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
            layout: bind_group_layout,
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
            ),
            With<Camera3d>,
        >,
    >,
) {
    for (entity, camera, depth_prepass, normal_prepass) in cameras_3d.iter() {
        if !camera.is_active {
            continue;
        }

        let mut entity = commands.get_or_spawn(entity);
        if depth_prepass.is_some() || normal_prepass.is_some() {
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
    }
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
        ),
        (
            With<RenderPhase<Opaque3dPrepass>>,
            With<RenderPhase<AlphaMask3dPrepass>>,
        ),
    >,
) {
    let mut depth_textures = HashMap::default();
    let mut normal_textures = HashMap::default();
    for (entity, camera, depth_prepass, normal_prepass) in &views_3d {
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

        commands.entity(entity).insert(ViewPrepassTextures {
            depth: cached_depth_texture,
            normal: cached_normals_texture,
            size,
        });
    }
}

#[derive(Default, Resource)]
pub struct PrepassViewBindGroup {
    bind_group: Option<BindGroup>,
}

pub fn queue_prepass_view_bind_group<M: Material>(
    render_device: Res<RenderDevice>,
    prepass_pipeline: Res<PrepassPipeline<M>>,
    view_uniforms: Res<ViewUniforms>,
    mut prepass_view_bind_group: ResMut<PrepassViewBindGroup>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        prepass_view_bind_group.bind_group =
            Some(render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_binding,
                }],
                label: Some("prepass_view_bind_group"),
                layout: &prepass_pipeline.view_layout,
            }));
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
    ) in &mut views
    {
        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
        if depth_prepass.is_some() {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }
        if normal_prepass.is_some() {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
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
    type ViewWorldQuery = Read<ViewUniformOffset>;
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        view_uniform_offset: &'_ ViewUniformOffset,
        _entity: (),
        prepass_view_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let prepass_view_bind_group = prepass_view_bind_group.into_inner();
        pass.set_bind_group(
            I,
            prepass_view_bind_group.bind_group.as_ref().unwrap(),
            &[view_uniform_offset.offset],
        );
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
