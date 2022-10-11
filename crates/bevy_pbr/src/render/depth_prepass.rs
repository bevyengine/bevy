use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Handle, HandleUntyped};
use bevy_core_pipeline::{core_3d::DepthPrepassSettings, prelude::Camera3d};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryState, With},
    system::{
        lifetimeless::{Read, SQuery, SRes},
        Commands, Query, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::ExtractedCamera,
    mesh::MeshVertexBufferLayout,
    prelude::{Camera, Color, Mesh},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{
        sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
        DrawFunctions, EntityPhaseItem, EntityRenderCommand, PhaseItem, RenderCommandResult,
        RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, BlendState, BufferBindingType, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
        Extent3d, FragmentState, FrontFace, LoadOp, MultisampleState, Operations, PipelineCache,
        PolygonMode, PrimitiveState, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Shader, ShaderStages, ShaderType,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
        StencilFaceState, StencilState, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages, VertexState,
    },
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, TextureCache},
    view::{
        ExtractedView, Msaa, ViewDepthTexture, ViewUniform, ViewUniformOffset, ViewUniforms,
        VisibleEntities,
    },
    Extract, RenderApp, RenderStage,
};
use bevy_utils::{tracing::error, FloatOrd, HashMap};

use crate::{
    AlphaMode, DrawMesh, Material, MeshPipeline, MeshPipelineKey, MeshUniform, RenderMaterials,
    SetMeshBindGroup,
};

use std::hash::Hash;

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the depth prepass node.
        pub const DEPTH_PREPASS: &str = "depth_prepass";
    }
}
pub const DEPTH_PREPASS_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub const DEPTH_PREPASS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 17179930919397780179);

pub struct DepthPrepassPlugin;

impl Plugin for DepthPrepassPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            DEPTH_PREPASS_SHADER_HANDLE,
            "depth_prepass.wgsl",
            Shader::from_wgsl
        );

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .add_system_to_stage(
                RenderStage::Extract,
                extract_core_3d_camera_depth_prepass_phase,
            )
            .add_system_to_stage(RenderStage::Prepare, prepare_core_3d_normal_textures)
            .add_system_to_stage(RenderStage::Queue, queue_depth_prepass_view_bind_group)
            .add_system_to_stage(
                RenderStage::PhaseSort,
                sort_phase_system::<OpaqueDepthPrepass>,
            )
            .add_system_to_stage(
                RenderStage::PhaseSort,
                sort_phase_system::<AlphaMaskDepthPrepass>,
            )
            .init_resource::<DepthPrepassPipeline>()
            .init_resource::<DrawFunctions<OpaqueDepthPrepass>>()
            .init_resource::<DrawFunctions<AlphaMaskDepthPrepass>>()
            .init_resource::<DepthPrepassViewBindGroup>()
            .init_resource::<SpecializedMeshPipelines<DepthPrepassPipeline>>();

        let depth_prepass_node = DepthPrepassNode::new(&mut render_app.world);
        render_app
            .add_render_command::<OpaqueDepthPrepass, DrawDepth>()
            .add_render_command::<AlphaMaskDepthPrepass, DrawDepth>();
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::DEPTH_PREPASS, depth_prepass_node);
        draw_3d_graph
            .add_node_edge(
                draw_3d_graph::node::DEPTH_PREPASS,
                bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
            )
            .unwrap();
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
                draw_3d_graph::node::DEPTH_PREPASS,
                DepthPrepassNode::IN_VIEW,
            )
            .unwrap();
    }
}

#[derive(Resource)]
pub struct DepthPrepassPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub skinned_mesh_layout: BindGroupLayout,
}

impl FromWorld for DepthPrepassPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

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
            label: Some("depth_prepass_view_layout"),
        });

        let mesh_pipeline = world.resource::<MeshPipeline>();
        let skinned_mesh_layout = mesh_pipeline.skinned_mesh_layout.clone();

        Self {
            view_layout,
            mesh_layout: mesh_pipeline.mesh_layout.clone(),
            skinned_mesh_layout,
        }
    }
}

impl SpecializedMeshPipeline for DepthPrepassPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut bind_group_layout = vec![self.view_layout.clone()];
        let mut shader_defs = Vec::new();

        if key.contains(MeshPipelineKey::ALPHA_MASK) {
            shader_defs.push(String::from("ALPHA_MASK"));
            // // FIXME: This needs to be implemented per-material!
            // bind_group_layout.push(self.material_layout);
        }

        let mut vertex_attributes = vec![Mesh::ATTRIBUTE_POSITION.at_shader_location(0)];

        if key.contains(MeshPipelineKey::DEPTH_PREPASS_NORMALS) {
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(1));
            shader_defs.push(String::from("OUTPUT_NORMALS"));
        }

        if layout.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push(String::from("VERTEX_UVS"));
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push(String::from("VERTEX_TANGENTS"));
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(3));
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            shader_defs.push(String::from("SKINNED"));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
            bind_group_layout.push(self.skinned_mesh_layout.clone());
        } else {
            bind_group_layout.push(self.mesh_layout.clone());
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        let fragment = if key.contains(MeshPipelineKey::DEPTH_PREPASS_NORMALS)
            || key.contains(MeshPipelineKey::ALPHA_MASK)
        {
            Some(FragmentState {
                shader: DEPTH_PREPASS_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "fragment".into(),
                shader_defs: shader_defs.clone(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgb10a2Unorm,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            })
        } else {
            None
        };

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: DEPTH_PREPASS_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs,
                buffers: vec![vertex_buffer_layout],
            },
            fragment,
            layout: Some(bind_group_layout),
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                // FIXME: Should use from material... but that would need specialization
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                // FIXME: Same as main pass
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
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("depth_prepass_pipeline".into()),
        })
    }
}

pub fn extract_core_3d_camera_depth_prepass_phase(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera, &DepthPrepassSettings), With<Camera3d>>>,
) {
    for (entity, camera, depth_prepass_settings) in cameras_3d.iter() {
        if camera.is_active {
            commands.get_or_spawn(entity).insert_bundle((
                RenderPhase::<OpaqueDepthPrepass>::default(),
                RenderPhase::<AlphaMaskDepthPrepass>::default(),
                depth_prepass_settings.clone(),
            ));
        }
    }
}

#[derive(Component)]
pub struct ViewPrepassTextures {
    pub depth: Option<CachedTexture>,
    pub normal: Option<CachedTexture>,
    pub size: Extent3d,
}

pub fn prepare_core_3d_normal_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (Entity, &ExtractedCamera, &DepthPrepassSettings),
        (
            With<RenderPhase<OpaqueDepthPrepass>>,
            With<RenderPhase<AlphaMaskDepthPrepass>>,
        ),
    >,
) {
    let mut depth_textures = HashMap::default();
    let mut normal_textures = HashMap::default();
    for (entity, camera, depth_prepass_settings) in &views_3d {
        if let Some(physical_target_size) = camera.physical_target_size {
            let size = Extent3d {
                depth_or_array_layers: 1,
                width: physical_target_size.x,
                height: physical_target_size.y,
            };

            let cached_depth_texture = match depth_prepass_settings.depth_resource {
                true => Some(
                    depth_textures
                        .entry(camera.target.clone())
                        .or_insert_with(|| {
                            texture_cache.get(
                                &render_device,
                                TextureDescriptor {
                                    label: Some("view_depth_texture_resource"),
                                    size,
                                    mip_level_count: 1,
                                    sample_count: msaa.samples,
                                    dimension: TextureDimension::D2,
                                    format: TextureFormat::Depth32Float,
                                    usage: TextureUsages::COPY_DST
                                        | TextureUsages::RENDER_ATTACHMENT
                                        | TextureUsages::TEXTURE_BINDING,
                                },
                            )
                        })
                        .clone(),
                ),
                false => None,
            };
            let cached_normal_texture = match depth_prepass_settings.output_normals {
                true => Some(
                    normal_textures
                        .entry(camera.target.clone())
                        .or_insert_with(|| {
                            texture_cache.get(
                                &render_device,
                                TextureDescriptor {
                                    label: Some("view_normal_texture"),
                                    size,
                                    mip_level_count: 1,
                                    sample_count: msaa.samples,
                                    dimension: TextureDimension::D2,
                                    format: TextureFormat::Rgb10a2Unorm,
                                    usage: TextureUsages::RENDER_ATTACHMENT
                                        | TextureUsages::TEXTURE_BINDING,
                                },
                            )
                        })
                        .clone(),
                ),
                false => None,
            };
            commands.entity(entity).insert(ViewPrepassTextures {
                depth: cached_depth_texture,
                normal: cached_normal_texture,
                size,
            });
        }
    }
}

#[derive(Default, Resource)]
pub struct DepthPrepassViewBindGroup {
    bind_group: Option<BindGroup>,
}

pub fn queue_depth_prepass_view_bind_group(
    render_device: Res<RenderDevice>,
    depth_prepass_pipeline: Res<DepthPrepassPipeline>,
    view_uniforms: Res<ViewUniforms>,
    mut depth_prepass_view_bind_group: ResMut<DepthPrepassViewBindGroup>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        depth_prepass_view_bind_group.bind_group =
            Some(render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_binding,
                }],
                label: Some("depth_prepass_view_bind_group"),
                layout: &depth_prepass_pipeline.view_layout,
            }));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_depth_prepass_material_meshes<M: Material>(
    opaque_draw_functions: Res<DrawFunctions<OpaqueDepthPrepass>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMaskDepthPrepass>>,
    // material_pipeline: Res<MaterialPipeline<M>>,
    depth_prepass_pipeline: Res<DepthPrepassPipeline>,
    // mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<DepthPrepassPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderMaterials<M>>,
    material_meshes: Query<(&Handle<M>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &DepthPrepassSettings,
        &mut RenderPhase<OpaqueDepthPrepass>,
        &mut RenderPhase<AlphaMaskDepthPrepass>,
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let opaque_draw_depth = opaque_draw_functions.read().get_id::<DrawDepth>().unwrap();
    let alpha_mask_draw_depth = alpha_mask_draw_functions
        .read()
        .get_id::<DrawDepth>()
        .unwrap();
    for (view, visible_entities, depth_prepass_settings, mut opaque_phase, mut alpha_mask_phase) in
        &mut views
    {
        let rangefinder = view.rangefinder3d();

        let mut view_key =
            MeshPipelineKey::DEPTH_PREPASS | MeshPipelineKey::from_msaa_samples(msaa.samples);
        if depth_prepass_settings.output_normals {
            view_key |= MeshPipelineKey::DEPTH_PREPASS_NORMALS;
        }

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if let Some(material) = render_materials.get(material_handle) {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mut key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | view_key;
                        let alpha_mode = material.properties.alpha_mode;
                        match alpha_mode {
                            AlphaMode::Opaque => {}
                            AlphaMode::Mask(_) => key |= MeshPipelineKey::ALPHA_MASK,
                            AlphaMode::Blend => continue,
                        }

                        let pipeline_id = pipelines.specialize(
                            &mut pipeline_cache,
                            &depth_prepass_pipeline,
                            key,
                            &mesh.layout,
                        );
                        let pipeline_id = match pipeline_id {
                            Ok(id) => id,
                            Err(err) => {
                                error!("{}", err);
                                continue;
                            }
                        };

                        let distance = rangefinder.distance(&mesh_uniform.transform)
                            + material.properties.depth_bias;
                        match alpha_mode {
                            AlphaMode::Opaque => {
                                opaque_phase.add(OpaqueDepthPrepass {
                                    entity: *visible_entity,
                                    draw_function: opaque_draw_depth,
                                    pipeline_id,
                                    distance,
                                });
                            }
                            AlphaMode::Mask(_) => {
                                alpha_mask_phase.add(AlphaMaskDepthPrepass {
                                    entity: *visible_entity,
                                    draw_function: alpha_mask_draw_depth,
                                    pipeline_id,
                                    distance,
                                });
                            }
                            AlphaMode::Blend => {}
                        }
                    }
                }
            }
        }
    }
}

pub struct OpaqueDepthPrepass {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline_id: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for OpaqueDepthPrepass {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
    }
}

impl EntityPhaseItem for OpaqueDepthPrepass {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for OpaqueDepthPrepass {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline_id
    }
}

pub struct AlphaMaskDepthPrepass {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline_id: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for AlphaMaskDepthPrepass {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
    }
}

impl EntityPhaseItem for AlphaMaskDepthPrepass {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMaskDepthPrepass {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline_id
    }
}

pub struct DepthPrepassNode {
    main_view_query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<OpaqueDepthPrepass>,
            &'static RenderPhase<AlphaMaskDepthPrepass>,
            &'static ViewDepthTexture,
            &'static ViewPrepassTextures,
        ),
        With<ExtractedView>,
    >,
}

impl DepthPrepassNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
        }
    }
}

impl Node for DepthPrepassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(DepthPrepassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        if let Ok((
            camera,
            opaque_depth_prepass_phase,
            alpha_mask_depth_prepass_phase,
            view_depth_texture,
            view_prepass_textures,
        )) = self.main_view_query.get_manual(world, view_entity)
        {
            if opaque_depth_prepass_phase.items.is_empty()
                && alpha_mask_depth_prepass_phase.items.is_empty()
            {
                return Ok(());
            }

            let mut color_attachments = vec![];
            if let Some(view_normal_texture) = &view_prepass_textures.normal {
                color_attachments.push(Some(RenderPassColorAttachment {
                    view: &view_normal_texture.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK.into()),
                        store: true,
                    },
                }));
            }

            {
                // Set up the pass descriptor with the depth attachment and maybe colour attachment
                let pass_descriptor = RenderPassDescriptor {
                    label: Some("depth_prepass"),
                    color_attachments: &color_attachments,
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &view_depth_texture.view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(0.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                };

                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&pass_descriptor);
                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                if let Some(viewport) = camera.viewport.as_ref() {
                    tracked_pass.set_camera_viewport(viewport);
                }

                {
                    // Run the depth prepass, sorted front-to-back
                    #[cfg(feature = "trace")]
                    let _opaque_depth_prepass_span = info_span!("opaque_depth_prepass").entered();
                    let draw_functions = world.resource::<DrawFunctions<OpaqueDepthPrepass>>();

                    let mut draw_functions = draw_functions.write();
                    for item in &opaque_depth_prepass_phase.items {
                        let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                        draw_function.draw(world, &mut tracked_pass, view_entity, item);
                    }
                }

                {
                    // Run the depth prepass, sorted front-to-back
                    #[cfg(feature = "trace")]
                    let _alpha_mask_depth_prepass_span =
                        info_span!("alpha_mask_depth_prepass").entered();
                    let draw_functions = world.resource::<DrawFunctions<AlphaMaskDepthPrepass>>();

                    let mut draw_functions = draw_functions.write();
                    for item in &alpha_mask_depth_prepass_phase.items {
                        let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                        draw_function.draw(world, &mut tracked_pass, view_entity, item);
                    }
                }
            }

            if let Some(view_depth_texture_resource) = &view_prepass_textures.depth {
                // copy depth buffer to texture
                render_context.command_encoder.copy_texture_to_texture(
                    view_depth_texture.texture.as_image_copy(),
                    view_depth_texture_resource.texture.as_image_copy(),
                    view_prepass_textures.size,
                );
            }
        }

        Ok(())
    }
}

pub type DrawDepth = (
    SetItemPipeline,
    SetDepthViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);

pub struct SetDepthViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetDepthViewBindGroup<I> {
    type Param = (
        SRes<DepthPrepassViewBindGroup>,
        SQuery<Read<ViewUniformOffset>>,
    );
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        (depth_prepass_view_bind_group, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform_offset = view_query.get(view).unwrap();
        let depth_prepass_view_bind_group = depth_prepass_view_bind_group.into_inner();
        pass.set_bind_group(
            I,
            depth_prepass_view_bind_group.bind_group.as_ref().unwrap(),
            &[view_uniform_offset.offset],
        );

        RenderCommandResult::Success
    }
}
