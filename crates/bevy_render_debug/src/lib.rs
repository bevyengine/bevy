use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, Handle};
use bevy_core_pipeline::{
    mip_generation::experimental::depth::ViewDepthPyramid,
    oit::OrderIndependentTransparencySettingsOffset, FullscreenShader,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::ReflectComponent,
    query::{Has, QueryItem},
    reflect::ReflectResource,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::BevyDefault;
use bevy_input::{prelude::KeyCode, ButtonInput};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_graph::{
        NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
        DynamicUniformBuffer, FragmentState, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderStages,
        ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
        TextureSampleType, VertexState,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::FallbackImage,
    view::{Msaa, ViewTarget, ViewUniformOffset},
    Render, RenderApp, RenderSystems,
};
use bevy_shader::Shader;

use bevy_pbr::{
    MeshPipelineViewLayoutKey, MeshPipelineViewLayouts, MeshViewBindGroup,
    ViewEnvironmentMapUniformOffset, ViewFogUniformOffset, ViewLightProbesUniformOffset,
    ViewLightsUniformOffset, ViewScreenSpaceReflectionsUniformOffset,
};

#[derive(Default)]
pub struct DebugBufferPlugin;

impl Plugin for DebugBufferPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "render/debug_buffer.wgsl");

        app.register_type::<DebugBufferConfig>()
            .init_resource::<DebugBufferConfig>()
            .add_plugins((
                ExtractResourcePlugin::<DebugBufferConfig>::default(),
                ExtractComponentPlugin::<DebugBufferConfig>::default(),
            ))
            .add_systems(bevy_app::Update, update_debug_buffer_config);
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DebugBufferPipeline>()
            .init_resource::<SpecializedRenderPipelines<DebugBufferPipeline>>()
            .init_resource::<DebugBufferUniforms>()
            .add_render_graph_node::<ViewNodeRunner<DebugBufferNode>>(
                bevy_core_pipeline::core_3d::graph::Core3d,
                DebugBufferLabel,
            )
            .add_render_graph_edge(
                bevy_core_pipeline::core_3d::graph::Core3d,
                bevy_core_pipeline::core_3d::graph::Node3d::Tonemapping,
                DebugBufferLabel,
            )
            .add_systems(
                Render,
                (
                    prepare_debug_buffer_pipelines.in_set(RenderSystems::Prepare),
                    prepare_debug_buffer_uniforms.in_set(RenderSystems::PrepareResources),
                    prepare_debug_buffer_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

pub fn update_debug_buffer_config(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config_res: ResMut<DebugBufferConfig>,
    cameras: Query<
        (Entity, Option<&DebugBufferConfig>),
        bevy_ecs::query::With<bevy_camera::Camera>,
    >,
) {
    let mut changed = false;

    if keyboard.just_pressed(KeyCode::F1) {
        if !config_res.enabled {
            config_res.enabled = true;
            config_res.mode = DebugMode::Depth;
            config_res.mip_level = 0;
        } else {
            match config_res.mode {
                DebugMode::Depth => config_res.mode = DebugMode::Normal,
                DebugMode::Normal => config_res.mode = DebugMode::MotionVectors,
                DebugMode::MotionVectors => {
                    config_res.mode = DebugMode::DepthPyramid;
                    config_res.mip_level = 0;
                }
                DebugMode::DepthPyramid => {
                    if config_res.mip_level < 7 {
                        config_res.mip_level += 1;
                    } else {
                        config_res.enabled = false;
                    }
                }
                DebugMode::Deferred => config_res.mode = DebugMode::Depth,
            }
        }
        changed = true;

        if config_res.enabled {
            bevy_log::info!(
                "Debug Buffer: {:?} (mip {})",
                config_res.mode,
                config_res.mip_level
            );
        } else {
            bevy_log::info!("Debug Buffer Disabled");
        }
    }

    if keyboard.just_pressed(KeyCode::F2) {
        config_res.opacity = if config_res.opacity < 0.8 {
            0.8
        } else if config_res.opacity < 0.95 {
            0.95
        } else if config_res.opacity < 1.0 {
            1.0
        } else {
            0.8
        };
        changed = true;
        bevy_log::info!("Debug Buffer Opacity: {}", config_res.opacity);
    }

    if keyboard.just_pressed(KeyCode::F12) {
        config_res.enabled = !config_res.enabled;
        changed = true;
        bevy_log::info!("Debug Buffer Enabled: {}", config_res.enabled);
    }

    if keyboard.pressed(KeyCode::Equal) {
        config_res.opacity = (config_res.opacity + 0.01).min(1.0);
        changed = true;
    }
    if keyboard.pressed(KeyCode::Minus) {
        config_res.opacity = (config_res.opacity - 0.01).max(0.0);
        changed = true;
    }

    for (entity, existing_config) in &cameras {
        if existing_config.is_none() || (changed && Some(config_res.as_ref()) != existing_config) {
            commands.entity(entity).insert(config_res.clone());
        }
    }
}

#[derive(Resource, Component, Clone, ExtractResource, ExtractComponent, Reflect, PartialEq)]
#[reflect(Resource, Component, Default)]
pub struct DebugBufferConfig {
    pub enabled: bool,
    pub mode: DebugMode,
    pub opacity: f32,
    pub mip_level: u32,
}

impl Default for DebugBufferConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: DebugMode::Depth,
            opacity: 1.0,
            mip_level: 0,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum DebugMode {
    #[default]
    Depth,
    Normal,
    MotionVectors,
    Deferred,
    DepthPyramid,
}

#[derive(ShaderType)]
pub struct DebugBufferUniform {
    pub opacity: f32,
    pub mip_level: u32,
}

#[derive(Resource, Default)]
pub struct DebugBufferUniforms {
    pub uniforms: DynamicUniformBuffer<DebugBufferUniform>,
}

#[derive(Component)]
pub struct DebugBufferUniformOffset {
    pub offset: u32,
}

#[derive(Resource)]
pub struct DebugBufferPipeline {
    pub shader: Handle<Shader>,
    pub mesh_view_layouts: MeshPipelineViewLayouts,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group_layout_descriptor: BindGroupLayoutDescriptor,
    pub sampler: Sampler,
    pub fullscreen_vertex_shader: VertexState,
}

impl FromWorld for DebugBufferPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<bevy_asset::AssetServer>();
        let mesh_view_layouts = world.resource::<MeshPipelineViewLayouts>().clone();
        let fullscreen_vertex_shader = world.resource::<FullscreenShader>().to_vertex_state();

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let bind_group_layout_descriptor = BindGroupLayoutDescriptor::new(
            "debug_buffer_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    binding_types::uniform_buffer::<DebugBufferUniform>(true),
                    binding_types::texture_2d(TextureSampleType::Float { filterable: true }),
                    binding_types::sampler(
                        bevy_render::render_resource::SamplerBindingType::Filtering,
                    ),
                ),
            ),
        );

        let bind_group_layout = render_device.create_bind_group_layout(
            bind_group_layout_descriptor.label.as_ref(),
            &bind_group_layout_descriptor.entries,
        );

        Self {
            shader: asset_server.load("embedded://bevy_render_debug/render/debug_buffer.wgsl"),
            mesh_view_layouts,
            bind_group_layout,
            bind_group_layout_descriptor,
            sampler,
            fullscreen_vertex_shader,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct DebugBufferPipelineKey {
    pub mode: DebugMode,
    pub view_layout_key: MeshPipelineViewLayoutKey,
}

impl SpecializedRenderPipeline for DebugBufferPipeline {
    type Key = DebugBufferPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        match key.mode {
            DebugMode::Depth => {
                shader_defs.push("DEBUG_DEPTH".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEPTH_PREPASS)
                {
                    shader_defs.push("DEPTH_PREPASS".into());
                }
            }
            DebugMode::Normal => {
                shader_defs.push("DEBUG_NORMAL".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::NORMAL_PREPASS)
                {
                    shader_defs.push("NORMAL_PREPASS".into());
                }
            }
            DebugMode::MotionVectors => {
                shader_defs.push("DEBUG_MOTION_VECTORS".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS)
                {
                    shader_defs.push("MOTION_VECTOR_PREPASS".into());
                }
            }
            DebugMode::Deferred => {
                shader_defs.push("DEBUG_DEFERRED".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            DebugMode::DepthPyramid => shader_defs.push("DEBUG_DEPTH_PYRAMID".into()),
        }

        if key
            .view_layout_key
            .contains(MeshPipelineViewLayoutKey::MULTISAMPLED)
        {
            shader_defs.push("MULTISAMPLED".into());
        }

        let mesh_view_layout_descriptor = self
            .mesh_view_layouts
            .get_view_layout(key.view_layout_key)
            .main_layout
            .clone();

        RenderPipelineDescriptor {
            label: Some("debug_buffer_pipeline".into()),
            layout: vec![
                mesh_view_layout_descriptor,
                self.bind_group_layout_descriptor.clone(),
            ],
            vertex: self.fullscreen_vertex_shader.clone(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(bevy_render::render_resource::BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: bevy_render::render_resource::PrimitiveState::default(),
            depth_stencil: None,
            multisample: bevy_render::render_resource::MultisampleState::default(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        }
    }
}

fn prepare_debug_buffer_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DebugBufferPipeline>>,
    pipeline: Res<DebugBufferPipeline>,
    views: Query<(
        Entity,
        &DebugBufferConfig,
        &Msaa,
        Has<bevy_core_pipeline::prepass::DepthPrepass>,
        Has<bevy_core_pipeline::prepass::NormalPrepass>,
        Has<bevy_core_pipeline::prepass::MotionVectorPrepass>,
        Has<bevy_core_pipeline::prepass::DeferredPrepass>,
        Has<bevy_core_pipeline::oit::OrderIndependentTransparencySettings>,
        Has<bevy_pbr::ExtractedAtmosphere>,
    )>,
) {
    for (
        entity,
        config,
        msaa,
        depth_prepass,
        normal_prepass,
        motion_vector_prepass,
        deferred_prepass,
        has_oit,
        has_atmosphere,
    ) in &views
    {
        if !config.enabled {
            continue;
        }

        let mut view_layout_key = MeshPipelineViewLayoutKey::from(*msaa);
        if depth_prepass {
            view_layout_key |= MeshPipelineViewLayoutKey::DEPTH_PREPASS;
        }
        if normal_prepass {
            view_layout_key |= MeshPipelineViewLayoutKey::NORMAL_PREPASS;
        }
        if motion_vector_prepass {
            view_layout_key |= MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS;
        }
        if deferred_prepass {
            view_layout_key |= MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
        }
        if has_oit {
            view_layout_key |= MeshPipelineViewLayoutKey::OIT_ENABLED;
        }
        if has_atmosphere {
            view_layout_key |= MeshPipelineViewLayoutKey::ATMOSPHERE;
        }

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            DebugBufferPipelineKey {
                mode: config.mode,
                view_layout_key,
            },
        );

        commands
            .entity(entity)
            .insert(DebugBufferPipelineId(pipeline_id));
    }
}

#[derive(Component)]
pub struct DebugBufferPipelineId(CachedRenderPipelineId);

fn prepare_debug_buffer_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniforms: ResMut<DebugBufferUniforms>,
    views: Query<(Entity, &DebugBufferConfig)>,
) {
    let len = views.iter().len();
    if len == 0 {
        return;
    }

    let Some(mut writer) = uniforms
        .uniforms
        .get_writer(len, &render_device, &render_queue)
    else {
        return;
    };
    for (entity, config) in &views {
        let offset = writer.write(&DebugBufferUniform {
            opacity: config.opacity,
            mip_level: config.mip_level,
        });
        commands
            .entity(entity)
            .insert(DebugBufferUniformOffset { offset });
    }
}

#[derive(Component)]
pub struct DebugBufferBindGroup(BindGroup);

fn prepare_debug_buffer_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<DebugBufferPipeline>,
    uniforms: Res<DebugBufferUniforms>,
    fallback_image: Res<FallbackImage>,
    views: Query<(Entity, &DebugBufferUniformOffset, Option<&ViewDepthPyramid>)>,
) {
    let Some(uniform_binding) = uniforms.uniforms.binding() else {
        return;
    };
    for (entity, _uniform_offset, depth_pyramid) in &views {
        let depth_pyramid_view = if let Some(dp) = depth_pyramid {
            &dp.all_mips
        } else {
            &fallback_image.d2.texture_view
        };

        let bind_group = render_device.create_bind_group(
            "debug_buffer_bind_group",
            &pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                uniform_binding.clone(),
                depth_pyramid_view,
                &pipeline.sampler,
            )),
        );
        commands
            .entity(entity)
            .insert(DebugBufferBindGroup(bind_group));
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct DebugBufferLabel;

#[derive(Default)]
pub struct DebugBufferNode;

impl ViewNode for DebugBufferNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static DebugBufferConfig,
        &'static DebugBufferPipelineId,
        &'static DebugBufferUniformOffset,
        &'static DebugBufferBindGroup,
        &'static MeshViewBindGroup,
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
        &'static ViewLightProbesUniformOffset,
        &'static ViewScreenSpaceReflectionsUniformOffset,
        &'static ViewEnvironmentMapUniformOffset,
        Has<bevy_core_pipeline::oit::OrderIndependentTransparencySettings>,
        Option<&'static OrderIndependentTransparencySettingsOffset>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            target,
            config,
            pipeline_id,
            uniform_offset,
            debug_bind_group,
            mesh_view_bind_group,
            view_uniform_offset,
            view_lights_offset,
            view_fog_offset,
            view_light_probes_offset,
            view_ssr_offset,
            view_environment_map_offset,
            has_oit,
            view_oit_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !config.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let post_process = target.post_process_write();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("debug_buffer_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: bevy_render::render_resource::LoadOp::Load,
                    store: bevy_render::render_resource::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);

        let mut dynamic_offsets = vec![
            view_uniform_offset.offset,
            view_lights_offset.offset,
            view_fog_offset.offset,
            **view_light_probes_offset,
            **view_ssr_offset,
            **view_environment_map_offset,
        ];
        if has_oit {
            if let Some(view_oit_offset) = view_oit_offset {
                dynamic_offsets.push(view_oit_offset.offset);
            }
        }

        render_pass.set_bind_group(0, &mesh_view_bind_group.main, &dynamic_offsets);
        render_pass.set_bind_group(1, &debug_bind_group.0, &[uniform_offset.offset]);

        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
