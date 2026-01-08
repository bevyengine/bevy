//! Renderer debugging overlay

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, Handle};
use bevy_core_pipeline::{
    mip_generation::experimental::depth::ViewDepthPyramid,
    oit::OrderIndependentTransparencySettingsOffset, FullscreenShader,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::{Has, ReflectComponent},
    query::QueryItem,
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

/// Adds a rendering debug overlay to visualize various renderer buffers.
#[derive(Default)]
pub struct DebugOverlayPlugin;

impl Plugin for DebugOverlayPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "debug_overlay.wgsl");

        app.register_type::<RenderDebugOverlay>()
            .init_resource::<RenderDebugOverlay>()
            .add_plugins((
                ExtractResourcePlugin::<RenderDebugOverlay>::default(),
                ExtractComponentPlugin::<RenderDebugOverlay>::default(),
            ))
            .add_systems(bevy_app::Update, update_overlay);
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DebugOverlayPipeline>()
            .init_resource::<SpecializedRenderPipelines<DebugOverlayPipeline>>()
            .init_resource::<DebugOverlayUniforms>()
            .add_render_graph_node::<ViewNodeRunner<DebugOverlayNode>>(
                bevy_core_pipeline::core_3d::graph::Core3d,
                DebugOverlayLabel,
            )
            .add_render_graph_edge(
                bevy_core_pipeline::core_3d::graph::Core3d,
                bevy_core_pipeline::core_3d::graph::Node3d::Tonemapping,
                DebugOverlayLabel,
            )
            .add_systems(
                Render,
                (
                    prepare_debug_overlay_pipelines.in_set(RenderSystems::Prepare),
                    prepare_debug_buffer_uniforms.in_set(RenderSystems::PrepareResources),
                    prepare_debug_buffer_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

/// Automatically attach keybinds to make renderer debugging tools immediately available without
/// code changes.
pub fn update_overlay(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config_res: ResMut<RenderDebugOverlay>,
    cameras: Query<
        (
            Entity,
            Option<&RenderDebugOverlay>,
            Has<bevy_core_pipeline::prepass::DepthPrepass>,
            Has<bevy_core_pipeline::prepass::NormalPrepass>,
            Has<bevy_core_pipeline::prepass::MotionVectorPrepass>,
            Has<bevy_core_pipeline::prepass::DeferredPrepass>,
            Has<bevy_render::experimental::occlusion_culling::OcclusionCulling>,
            Has<bevy_pbr::ScreenSpaceReflections>,
        ),
        bevy_ecs::query::With<bevy_camera::Camera>,
    >,
) {
    let mut changed = false;

    if keyboard.just_pressed(KeyCode::F1) {
        let modes = [
            DebugMode::Depth,
            DebugMode::Normal,
            DebugMode::MotionVectors,
            DebugMode::Deferred,
            DebugMode::DeferredBaseColor,
            DebugMode::DeferredEmissive,
            DebugMode::DeferredMetallicRoughness,
            DebugMode::DepthPyramid { mip_level: 0 },
        ];

        let is_supported = |mode: &DebugMode| {
            cameras
                .iter()
                .any(|(_, _, depth, normal, motion, deferred, occlusion, _ssr)| {
                    match mode {
                        DebugMode::Depth => depth || deferred,
                        DebugMode::Normal => normal || deferred,
                        DebugMode::MotionVectors => motion,
                        DebugMode::Deferred
                        | DebugMode::DeferredBaseColor
                        | DebugMode::DeferredEmissive
                        | DebugMode::DeferredMetallicRoughness => deferred,
                        // We don't have a good way to check for DepthPyramid in the main world
                        // but it usually depends on DepthPrepass.
                        // However, we can at least check if OcclusionCulling is present.
                        DebugMode::DepthPyramid { .. } => depth && occlusion,
                    }
                })
        };

        if !config_res.enabled {
            for mode in modes {
                if is_supported(&mode) {
                    config_res.enabled = true;
                    config_res.mode = mode;
                    break;
                }
            }
        } else {
            let current_index = modes
                .iter()
                .position(|m| {
                    core::mem::discriminant(m) == core::mem::discriminant(&config_res.mode)
                })
                .unwrap_or(0);

            let mut next_mode = None;

            // First check if we can increment mip level
            if let DebugMode::DepthPyramid { mip_level } = config_res.mode {
                if mip_level < 7 {
                    config_res.mode = DebugMode::DepthPyramid {
                        mip_level: mip_level + 1,
                    };
                    next_mode = Some(config_res.mode);
                }
            }

            if next_mode.is_none() {
                // Look for next supported mode
                for i in 1..modes.len() {
                    let idx = (current_index + i) % modes.len();
                    if is_supported(&modes[idx]) {
                        next_mode = Some(modes[idx]);
                        break;
                    }
                }

                if let Some(mode) = next_mode {
                    let next_index = modes
                        .iter()
                        .position(|m| core::mem::discriminant(m) == core::mem::discriminant(&mode))
                        .unwrap();

                    if next_index <= current_index {
                        config_res.enabled = false;
                    } else {
                        config_res.mode = mode;
                    }
                } else {
                    config_res.enabled = false;
                }
            }
        }
        changed = true;

        if config_res.enabled {
            bevy_log::info!("Debug Overlay: {:?}", config_res.mode);
        } else {
            bevy_log::info!("Debug Overlay Disabled");
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
        bevy_log::info!("Debug Overlay Opacity: {}", config_res.opacity);
    }

    for (entity, existing_config, ..) in &cameras {
        if existing_config.is_none() || (changed && Some(config_res.as_ref()) != existing_config) {
            commands.entity(entity).insert(config_res.clone());
        }
    }
}

/// Configure the render debug overlay.
#[derive(Resource, Component, Clone, ExtractResource, ExtractComponent, Reflect, PartialEq)]
#[reflect(Resource, Component, Default)]
pub struct RenderDebugOverlay {
    /// Enables or disables drawing the overlay.
    pub enabled: bool,
    /// The kind of data to write to the overlay.
    pub mode: DebugMode,
    /// The opacity of the overlay, to allow seeing the rendered image underneath.
    pub opacity: f32,
}

impl Default for RenderDebugOverlay {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: DebugMode::Depth,
            opacity: 1.0,
        }
    }
}

/// The kind of renderer data to visualize.
#[allow(missing_docs)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum DebugMode {
    #[default]
    Depth,
    Normal,
    MotionVectors,
    Deferred,
    DeferredBaseColor,
    DeferredEmissive,
    DeferredMetallicRoughness,
    DepthPyramid {
        mip_level: u32,
    },
}

#[derive(ShaderType)]
struct DebugOverlayUniform {
    pub opacity: f32,
    pub mip_level: u32,
}

#[derive(Resource, Default)]
struct DebugOverlayUniforms {
    pub uniforms: DynamicUniformBuffer<DebugOverlayUniform>,
}

#[derive(Component)]
struct DebugOverlayUniformOffset {
    pub offset: u32,
}

#[derive(Resource)]
struct DebugOverlayPipeline {
    shader: Handle<Shader>,
    mesh_view_layouts: MeshPipelineViewLayouts,
    bind_group_layout: BindGroupLayout,
    bind_group_layout_descriptor: BindGroupLayoutDescriptor,
    sampler: Sampler,
    fullscreen_vertex_shader: VertexState,
}

impl FromWorld for DebugOverlayPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<bevy_asset::AssetServer>();
        let mesh_view_layouts = world.resource::<MeshPipelineViewLayouts>().clone();
        let fullscreen_vertex_shader = world.resource::<FullscreenShader>().to_vertex_state();

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let bind_group_layout_descriptor = BindGroupLayoutDescriptor::new(
            "debug_overlay_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    binding_types::uniform_buffer::<DebugOverlayUniform>(true),
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
            shader: asset_server.load("embedded://bevy_render_debug/debug_overlay.wgsl"),
            mesh_view_layouts,
            bind_group_layout,
            bind_group_layout_descriptor,
            sampler,
            fullscreen_vertex_shader,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct DebugOverlayPipelineKey {
    mode: DebugMode,
    view_layout_key: MeshPipelineViewLayoutKey,
}

impl SpecializedRenderPipeline for DebugOverlayPipeline {
    type Key = DebugOverlayPipelineKey;

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
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
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
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
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
            DebugMode::DeferredBaseColor => {
                shader_defs.push("DEBUG_DEFERRED_BASE_COLOR".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            DebugMode::DeferredEmissive => {
                shader_defs.push("DEBUG_DEFERRED_EMISSIVE".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            DebugMode::DeferredMetallicRoughness => {
                shader_defs.push("DEBUG_DEFERRED_METALLIC_ROUGHNESS".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            DebugMode::DepthPyramid { .. } => shader_defs.push("DEBUG_DEPTH_PYRAMID".into()),
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
            label: Some("debug_overlay_pipeline".into()),
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

fn prepare_debug_overlay_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DebugOverlayPipeline>>,
    pipeline: Res<DebugOverlayPipeline>,
    views: Query<(
        Entity,
        &RenderDebugOverlay,
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
            DebugOverlayPipelineKey {
                mode: config.mode,
                view_layout_key,
            },
        );

        commands
            .entity(entity)
            .insert(DebugOverlayPipelineId(pipeline_id));
    }
}

#[derive(Component)]
struct DebugOverlayPipelineId(CachedRenderPipelineId);

fn prepare_debug_buffer_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniforms: ResMut<DebugOverlayUniforms>,
    views: Query<(Entity, &RenderDebugOverlay)>,
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
        let offset = writer.write(&DebugOverlayUniform {
            opacity: config.opacity,
            mip_level: if let DebugMode::DepthPyramid { mip_level } = config.mode {
                mip_level
            } else {
                0
            },
        });
        commands
            .entity(entity)
            .insert(DebugOverlayUniformOffset { offset });
    }
}

#[derive(Component)]
struct DebugOverlayBindGroup(BindGroup);

fn prepare_debug_buffer_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<DebugOverlayPipeline>,
    uniforms: Res<DebugOverlayUniforms>,
    fallback_image: Res<FallbackImage>,
    views: Query<(
        Entity,
        &DebugOverlayUniformOffset,
        Option<&ViewDepthPyramid>,
    )>,
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
            .insert(DebugOverlayBindGroup(bind_group));
    }
}

/// The render debug overlay.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct DebugOverlayLabel;

#[derive(Default)]
struct DebugOverlayNode;

impl ViewNode for DebugOverlayNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static RenderDebugOverlay,
        &'static DebugOverlayPipelineId,
        &'static DebugOverlayUniformOffset,
        &'static DebugOverlayBindGroup,
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
