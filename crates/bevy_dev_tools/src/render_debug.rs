//! Renderer debugging overlay

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, Handle};
use bevy_core_pipeline::{
    mip_generation::experimental::depth::ViewDepthPyramid,
    oit::OrderIndependentTransparencySettingsOffset,
    schedule::{Core3d, Core3dSystems},
    FullscreenShader,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    message::{Message, MessageReader, MessageWriter},
    prelude::{Has, ReflectComponent, World},
    reflect::ReflectResource,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::FromWorld,
};
use bevy_input::{prelude::KeyCode, ButtonInput};
use bevy_log::info;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_asset::RenderAssets,
    render_resource::{
        binding_types, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
        DynamicUniformBuffer, FragmentState, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderStages,
        ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
        TextureSampleType, VertexState,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
    texture::{FallbackImage, GpuImage},
    view::{Msaa, ViewTarget, ViewUniformOffset},
    Render, RenderApp, RenderSystems,
};
use bevy_shader::Shader;

use bevy_pbr::{
    Bluenoise, MeshPipelineViewLayoutKey, MeshPipelineViewLayouts, MeshViewBindGroup,
    ViewContactShadowsUniformOffset, ViewEnvironmentMapUniformOffset, ViewFogUniformOffset,
    ViewLightProbesUniformOffset, ViewLightsUniformOffset, ViewScreenSpaceReflectionsUniformOffset,
};

/// Adds a rendering debug overlay to visualize various renderer buffers.
#[derive(Default)]
pub struct RenderDebugOverlayPlugin;

impl Plugin for RenderDebugOverlayPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "debug_overlay.wgsl");

        app.register_type::<RenderDebugOverlay>()
            .init_resource::<RenderDebugOverlay>()
            .add_message::<RenderDebugOverlayEvent>()
            .add_plugins((
                ExtractResourcePlugin::<RenderDebugOverlay>::default(),
                ExtractComponentPlugin::<RenderDebugOverlay>::default(),
            ))
            .add_systems(bevy_app::Update, (handle_input, update_overlay).chain());
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderDebugOverlayPipeline>()
            .init_resource::<SpecializedRenderPipelines<RenderDebugOverlayPipeline>>()
            .init_resource::<RenderDebugOverlayUniforms>()
            .add_systems(
                Render,
                (
                    prepare_debug_overlay_pipelines.in_set(RenderSystems::Prepare),
                    prepare_debug_overlay_resources.in_set(RenderSystems::PrepareResources),
                ),
            )
            .add_systems(
                Core3d,
                render_debug_overlay.in_set(Core3dSystems::PostProcess),
            );
    }
}

/// Automatically attach keybinds to make render debug overlays available to users without code
/// changes when the feature is enabled.
pub fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: MessageWriter<RenderDebugOverlayEvent>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        events.write(RenderDebugOverlayEvent::CycleMode);
    }
    if keyboard.just_pressed(KeyCode::F2) {
        events.write(RenderDebugOverlayEvent::CycleOpacity);
    }
}

/// Listen to messages to update the debug overlay configuration.
pub fn update_overlay(
    mut commands: Commands,
    mut events: MessageReader<RenderDebugOverlayEvent>,
    mut config_res: ResMut<RenderDebugOverlay>,
    cameras: Query<
        (
            Entity,
            Option<&RenderDebugOverlay>,
            Has<bevy_core_pipeline::prepass::DepthPrepass>,
            Has<bevy_core_pipeline::prepass::NormalPrepass>,
            Has<bevy_core_pipeline::prepass::MotionVectorPrepass>,
            Has<bevy_core_pipeline::prepass::DeferredPrepass>,
            Has<bevy_render::occlusion_culling::OcclusionCulling>,
            Has<bevy_pbr::ScreenSpaceReflections>,
        ),
        bevy_ecs::query::With<bevy_camera::Camera>,
    >,
) {
    let mut changed = false;

    for event in events.read() {
        match event {
            RenderDebugOverlayEvent::CycleMode => {
                let modes = [
                    RenderDebugMode::Depth,
                    RenderDebugMode::Normal,
                    RenderDebugMode::MotionVectors,
                    RenderDebugMode::Deferred,
                    RenderDebugMode::DeferredBaseColor,
                    RenderDebugMode::DeferredEmissive,
                    RenderDebugMode::DeferredMetallicRoughness,
                    RenderDebugMode::DepthPyramid { mip_level: 0 },
                ];

                let is_supported = |mode: &RenderDebugMode| {
                    cameras.iter().any(
                        |(_, _, depth, normal, motion, deferred, occlusion, _ssr)| {
                            match mode {
                                RenderDebugMode::Depth => depth || deferred,
                                RenderDebugMode::Normal => normal || deferred,
                                RenderDebugMode::MotionVectors => motion,
                                RenderDebugMode::Deferred
                                | RenderDebugMode::DeferredBaseColor
                                | RenderDebugMode::DeferredEmissive
                                | RenderDebugMode::DeferredMetallicRoughness => deferred,
                                // We don't have a good way to check for DepthPyramid in the main
                                // world, but it usually depends on DepthPrepass.
                                // However, we can at least check if OcclusionCulling is present.
                                RenderDebugMode::DepthPyramid { .. } => depth && occlusion,
                            }
                        },
                    )
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

                    if let RenderDebugMode::DepthPyramid { mip_level } = config_res.mode
                        && mip_level < 7
                    {
                        config_res.mode = RenderDebugMode::DepthPyramid {
                            mip_level: mip_level + 1,
                        };
                        next_mode = Some(config_res.mode);
                    }

                    if next_mode.is_none() {
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
                                .position(|m| {
                                    core::mem::discriminant(m) == core::mem::discriminant(&mode)
                                })
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
                    info!("Debug Overlay: {:?}", config_res.mode);
                } else {
                    info!("Debug Overlay Disabled");
                }
            }
            RenderDebugOverlayEvent::CycleOpacity => {
                config_res.opacity = if config_res.opacity < 0.5 {
                    0.5
                } else if config_res.opacity < 0.8 {
                    0.8
                } else if config_res.opacity < 1.0 {
                    1.0
                } else {
                    0.5
                };
                changed = true;
                info!("Debug Overlay Opacity: {}", config_res.opacity);
            }
        }
    }

    for (entity, existing_config, ..) in &cameras {
        if existing_config.is_none() || (changed && Some(config_res.as_ref()) != existing_config) {
            commands.entity(entity).insert(config_res.clone());
        }
    }
}

/// Configure the render debug overlay.
#[derive(Message, Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum RenderDebugOverlayEvent {
    /// Cycle to the next debug mode.
    CycleMode,
    /// Cycle to the next opacity level.
    CycleOpacity,
}

/// Configure the render debug overlay.
#[derive(Resource, Component, Clone, ExtractResource, ExtractComponent, Reflect, PartialEq)]
#[reflect(Resource, Component, Default)]
pub struct RenderDebugOverlay {
    /// Enables or disables drawing the overlay.
    pub enabled: bool,
    /// The kind of data to write to the overlay.
    pub mode: RenderDebugMode,
    /// The opacity of the overlay, to allow seeing the rendered image underneath.
    pub opacity: f32,
}

impl Default for RenderDebugOverlay {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: RenderDebugMode::Depth,
            opacity: 1.0,
        }
    }
}

/// The kind of renderer data to visualize.
#[expect(missing_docs, reason = "Enum variants are self-explanatory")]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum RenderDebugMode {
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
struct RenderDebugOverlayUniform {
    pub opacity: f32,
    pub mip_level: u32,
}

#[derive(Resource, Default)]
struct RenderDebugOverlayUniforms {
    pub uniforms: DynamicUniformBuffer<RenderDebugOverlayUniform>,
}

#[derive(Component)]
struct RenderDebugOverlayUniformOffset {
    pub offset: u32,
}

#[derive(Resource)]
struct RenderDebugOverlayPipeline {
    shader: Handle<Shader>,
    mesh_view_layouts: MeshPipelineViewLayouts,
    bind_group_layout: BindGroupLayout,
    bind_group_layout_descriptor: BindGroupLayoutDescriptor,
    sampler: Sampler,
    fullscreen_vertex_shader: VertexState,
}

impl FromWorld for RenderDebugOverlayPipeline {
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
                    binding_types::uniform_buffer::<RenderDebugOverlayUniform>(true),
                    binding_types::texture_2d(TextureSampleType::Float { filterable: true }),
                    binding_types::sampler(
                        bevy_render::render_resource::SamplerBindingType::Filtering,
                    ),
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
            shader: asset_server.load("embedded://bevy_dev_tools/debug_overlay.wgsl"),
            mesh_view_layouts,
            bind_group_layout,
            bind_group_layout_descriptor,
            sampler,
            fullscreen_vertex_shader,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct RenderDebugOverlayPipelineKey {
    mode: RenderDebugMode,
    view_layout_key: MeshPipelineViewLayoutKey,
    texture_format: TextureFormat,
}

impl SpecializedRenderPipeline for RenderDebugOverlayPipeline {
    type Key = RenderDebugOverlayPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        match key.mode {
            RenderDebugMode::Depth => {
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
            RenderDebugMode::Normal => {
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
            RenderDebugMode::MotionVectors => {
                shader_defs.push("DEBUG_MOTION_VECTORS".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS)
                {
                    shader_defs.push("MOTION_VECTOR_PREPASS".into());
                }
            }
            RenderDebugMode::Deferred => {
                shader_defs.push("DEBUG_DEFERRED".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            RenderDebugMode::DeferredBaseColor => {
                shader_defs.push("DEBUG_DEFERRED_BASE_COLOR".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            RenderDebugMode::DeferredEmissive => {
                shader_defs.push("DEBUG_DEFERRED_EMISSIVE".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            RenderDebugMode::DeferredMetallicRoughness => {
                shader_defs.push("DEBUG_DEFERRED_METALLIC_ROUGHNESS".into());
                if key
                    .view_layout_key
                    .contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS)
                {
                    shader_defs.push("DEFERRED_PREPASS".into());
                }
            }
            RenderDebugMode::DepthPyramid { .. } => shader_defs.push("DEBUG_DEPTH_PYRAMID".into()),
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
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: bevy_render::render_resource::PrimitiveState::default(),
            depth_stencil: None,
            multisample: bevy_render::render_resource::MultisampleState::default(),
            immediate_size: 0,
            zero_initialize_workgroup_memory: false,
        }
    }
}

fn prepare_debug_overlay_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<RenderDebugOverlayPipeline>>,
    pipeline: Res<RenderDebugOverlayPipeline>,
    images: Res<RenderAssets<GpuImage>>,
    blue_noise: Res<Bluenoise>,
    views: Query<(
        Entity,
        &ViewTarget,
        &RenderDebugOverlay,
        &Msaa,
        Option<&bevy_core_pipeline::prepass::ViewPrepassTextures>,
        Has<bevy_core_pipeline::oit::OrderIndependentTransparencySettings>,
        Has<bevy_pbr::ExtractedAtmosphere>,
    )>,
) {
    for (entity, target, config, msaa, prepass_textures, has_oit, has_atmosphere) in &views {
        if !config.enabled {
            continue;
        }

        let mut view_layout_key = MeshPipelineViewLayoutKey::from(*msaa)
            | MeshPipelineViewLayoutKey::from(prepass_textures);

        if has_oit {
            view_layout_key |= MeshPipelineViewLayoutKey::OIT_ENABLED;
        }
        if has_atmosphere {
            view_layout_key |= MeshPipelineViewLayoutKey::ATMOSPHERE;
        }

        if let Some(gpu_image) = images.get(&blue_noise.texture)
            && gpu_image.texture.depth_or_array_layers() > 1
        {
            view_layout_key |= MeshPipelineViewLayoutKey::STBN;
        }

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            RenderDebugOverlayPipelineKey {
                mode: config.mode,
                view_layout_key,
                texture_format: target.main_texture_format(),
            },
        );

        commands
            .entity(entity)
            .insert(RenderDebugOverlayPipelineId(pipeline_id));
    }
}

#[derive(Component)]
struct RenderDebugOverlayPipelineId(CachedRenderPipelineId);

fn prepare_debug_overlay_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniforms: ResMut<RenderDebugOverlayUniforms>,
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
        let offset = writer.write(&RenderDebugOverlayUniform {
            opacity: config.opacity,
            mip_level: if let RenderDebugMode::DepthPyramid { mip_level } = config.mode {
                mip_level
            } else {
                0
            },
        });

        commands
            .entity(entity)
            .insert(RenderDebugOverlayUniformOffset { offset });
    }
}

fn render_debug_overlay(
    view: ViewQuery<(
        &ViewTarget,
        &RenderDebugOverlay,
        &RenderDebugOverlayPipelineId,
        &RenderDebugOverlayUniformOffset,
        &MeshViewBindGroup,
        &ViewUniformOffset,
        &ViewLightsUniformOffset,
        &ViewFogUniformOffset,
        &ViewLightProbesUniformOffset,
        &ViewScreenSpaceReflectionsUniformOffset,
        &ViewContactShadowsUniformOffset,
        &ViewEnvironmentMapUniformOffset,
        Has<bevy_core_pipeline::oit::OrderIndependentTransparencySettings>,
        Option<&OrderIndependentTransparencySettingsOffset>,
        Option<&ViewDepthPyramid>,
    )>,
    pipeline_cache: Res<PipelineCache>,
    pipeline_res: Res<RenderDebugOverlayPipeline>,
    uniforms: Res<RenderDebugOverlayUniforms>,
    fallback_image: Res<FallbackImage>,
    mut ctx: RenderContext,
) {
    let (
        target,
        config,
        pipeline_id,
        uniform_offset,
        mesh_view_bind_group,
        view_uniform_offset,
        view_lights_offset,
        view_fog_offset,
        view_light_probes_offset,
        view_ssr_offset,
        view_contact_shadows_offset,
        view_environment_map_offset,
        has_oit,
        view_oit_offset,
        depth_pyramid,
    ) = view.into_inner();

    if !config.enabled {
        return;
    }

    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
        return;
    };

    let Some(uniform_binding) = uniforms.uniforms.binding() else {
        return;
    };

    let post_process = target.post_process_write();

    let depth_pyramid_view = if let Some(dp) = depth_pyramid {
        &dp.all_mips
    } else {
        &fallback_image.d2.texture_view
    };

    let debug_bind_group = ctx.render_device().create_bind_group(
        "debug_buffer_bind_group",
        &pipeline_res.bind_group_layout,
        &BindGroupEntries::sequential((
            uniform_binding,
            post_process.source,
            &pipeline_res.sampler,
            depth_pyramid_view,
            &pipeline_res.sampler,
        )),
    );

    let pass_descriptor = RenderPassDescriptor {
        label: Some("debug_buffer_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post_process.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: bevy_render::render_resource::LoadOp::Clear(Default::default()),
                store: bevy_render::render_resource::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };

    let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);

    render_pass.set_pipeline(pipeline);

    let mut dynamic_offsets = vec![
        view_uniform_offset.offset,
        view_lights_offset.offset,
        view_fog_offset.offset,
        **view_light_probes_offset,
        **view_ssr_offset,
        **view_contact_shadows_offset,
        **view_environment_map_offset,
    ];
    if has_oit && let Some(view_oit_offset) = view_oit_offset {
        dynamic_offsets.push(view_oit_offset.offset);
    }

    render_pass.set_bind_group(0, &mesh_view_bind_group.main, &dynamic_offsets);
    render_pass.set_bind_group(1, &debug_bind_group, &[uniform_offset.offset]);

    render_pass.draw(0..3, 0..1);
}
